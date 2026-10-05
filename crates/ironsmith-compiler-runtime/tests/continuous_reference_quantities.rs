//! UNVALIDATED live continuous-reference quantity and linked-face regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/continuous_reference_quantities.json.fixture"
    ))
    .unwrap()
}
fn compile_row(row: &serde_json::Value, other_name: Option<&str>) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    let mut builder =
        ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name);
    if let Some(other) = other_name {
        builder = builder.other_face_name(other);
    }
    let (result, loss) = parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_builder_to_artifact(builder, lines.join("\n"), false)
    });
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let transported =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    for definition in [&direct, &transported] {
        assert_eq!(definition.card.other_face_name.as_deref(), other_name);
    }
    [direct, transported]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|r| r["name"] == name || r["canonical_name"] == name)
        .unwrap();
    compile_row(&row, row["other_face"]["name"].as_str())
}
fn linked_glamdring(game: &mut GameState, transported: usize) -> CardDefinition {
    let row = fixtures()
        .into_iter()
        .find(|row| row["other_face"].is_object())
        .unwrap();
    let mut front = compile_row(&row, row["other_face"]["name"].as_str())[transported].clone();
    let mut back = compile_row(&row["other_face"], row["name"].as_str())[transported].clone();
    front.card.other_face = Some(back.card.id);
    back.card.other_face = Some(front.card.id);
    // Adventures use their typed Adventure spell face, not the split-card
    // combined mana value or transforming-DFC rules.
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    front
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    objects_explicit: bool,
    x: u32,
    decline: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if !self.targets.is_empty() {
            assert_eq!(context.requirements.len(), self.targets.len());
            for (requirement, target) in context.requirements.iter().zip(&self.targets) {
                assert!(requirement.legal_targets.contains(target));
            }
            self.targets.clone()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if self.objects_explicit || !self.objects.is_empty() {
            for id in &self.objects {
                assert!(
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                );
            }
            self.objects.clone()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
}
fn resource(name: &str, types: &str) -> CardDefinition {
    let pt = if types.contains("Creature") {
        "\nPower/Toughness: 1/1"
    } else {
        ""
    };
    compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: {types}{pt}"), false)
        .unwrap()
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    // Checked execution already captures some triggers in the original observer frame.
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.current_power(id).unwrap(),
        game.current_toughness(id).unwrap(),
    )
}
fn pump(game: &mut GameState, source: ObjectId, id: ObjectId, p: i32, t: i32) {
    apply(
        game,
        source,
        Effect::pump(p, t, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn counter(game: &mut GameState, source: ObjectId, id: ObjectId, count: i32) {
    apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::SpecificObject(id),
        ),
    );
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}

fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..50 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}

fn cast_existing(
    game: &mut GameState,
    id: ObjectId,
    from_zone: Zone,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}
fn attach(game: &mut GameState, equipment: ObjectId, host: ObjectId) {
    apply(
        game,
        equipment,
        Effect::attach_objects(
            ChooseSpec::SpecificObject(equipment),
            ChooseSpec::SpecificObject(host),
        ),
    );
}
fn paid_total(game: &GameState, spell: ObjectId) -> u32 {
    game.object(spell).unwrap().mana_spent_to_cast.total()
}
fn copy_until_end(
    game: &mut GameState,
    source: ObjectId,
    recipient: ObjectId,
    reference: ObjectId,
) {
    let copy = ironsmith::effects::ApplyContinuousEffect::new_runtime(
        ironsmith::continuous::EffectTarget::Specific(recipient),
        ironsmith::effects::continuous::RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(reference),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: Vec::new(),
            copy_exception_surface: None,
        },
        Until::EndOfTurn,
    );
    apply(game, source, Effect::new(copy));
}

#[test]
fn three_exact_identities_and_the_adventure_face_transport_typed_quantities() {
    assert_eq!(fixtures().len(), 3);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
        if row["other_face"].is_object() {
            for definition in compile_row(&row["other_face"], row["name"].as_str()) {
                assert!(
                    !ironsmith::cards::generated_definition_has_unimplemented_content(&definition)
                );
            }
        }
    }
}

#[test]
fn hedron_reads_the_current_host_mana_value_through_equip_copy_and_face_down_layers() {
    for definition in definitions("Hedron Matrix") {
        let mut game = game();
        let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = game.create_object_from_definition(
            &vanilla("First host", "{2}", "Human", 1, 2),
            A,
            Zone::Battlefield,
        );
        let second = game.create_object_from_definition(
            &vanilla("Second host", "{5}", "Human", 2, 4),
            A,
            Zone::Battlefield,
        );
        let donor = game.create_object_from_definition(
            &vanilla("Copy reference", "{6}", "Wizard", 4, 5),
            B,
            Zone::Battlefield,
        );
        let mut dm = Choices {
            targets: vec![Target::Object(first)],
            ..Default::default()
        };
        activate(&mut game, equipment, activated(&definition), &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, first), (3, 4));
        assert_eq!(pt(&game, second), (2, 4));
        copy_until_end(&mut game, equipment, first, donor);
        assert_eq!(
            pt(&game, first),
            (10, 11),
            "copy mana value is six, not the printed two"
        );
        assert!(game.set_face_down(first));
        assert_eq!(pt(&game, first), (2, 2), "face-down mana value is zero");
        dm.targets = vec![Target::Object(second)];
        activate(&mut game, equipment, activated(&definition), &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, first), (2, 2));
        assert_eq!(pt(&game, second), (7, 9));
        game.set_current_controller(equipment, B).unwrap();
        game.set_current_controller(second, C).unwrap();
        assert_eq!(pt(&game, second), (7, 9));
        game.move_object_by_game_rule(equipment, Zone::Graveyard)
            .unwrap();
        assert_eq!(pt(&game, second), (2, 4));
    }
}

#[test]
fn hedron_uses_the_actually_cast_prototype_mana_cost() {
    for definition in definitions("Hedron Matrix") {
        for prototype in [false, true] {
            let mut game = game();
            let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let host = compile_to_runtime_definition("Prototype host", "Mana cost: {5}\nType: Artifact Creature — Construct\nPower/Toughness: 5/5\nPrototype {1}{G} — 2/3", false).unwrap();
            let method = if prototype {
                CastingMethod::Alternative(
                    host.alternative_casts
                        .iter()
                        .position(|method| method.name().eq_ignore_ascii_case("prototype"))
                        .unwrap(),
                )
            } else {
                CastingMethod::Normal
            };
            let mut dm = Choices::default();
            let spell = cast(&mut game, &host, method, &mut dm);
            let stable = game.object(spell).unwrap().stable_id;
            resolve_all(&mut game, &mut dm);
            let host = game.find_object_by_stable_id(stable).unwrap();
            attach(&mut game, equipment, host);
            assert_eq!(pt(&game, host), if prototype { (4, 5) } else { (10, 10) });
        }
    }
}

#[test]
fn hancock_counts_all_source_counters_and_tracks_controller_and_undying_changes() {
    for definition in definitions("Hancock, Ghoulish Mayor") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let zombie = game.create_object_from_definition(
            &vanilla("Zombie recipient", "{1}", "Zombie", 1, 2),
            A,
            Zone::Battlefield,
        );
        let both = game.create_object_from_definition(
            &vanilla("Both types", "{1}", "Zombie Mutant", 1, 2),
            A,
            Zone::Battlefield,
        );
        let other = game.create_object_from_definition(
            &vanilla("Other controller", "{1}", "Zombie", 1, 2),
            B,
            Zone::Battlefield,
        );
        let human = game.create_object_from_definition(
            &vanilla("Wrong subtype", "{1}", "Human", 1, 2),
            A,
            Zone::Battlefield,
        );
        assert_eq!(pt(&game, zombie), (1, 2));
        counter(&mut game, source, zombie, 7);
        counter(&mut game, source, source, 2);
        apply(
            &mut game,
            source,
            Effect::put_counters(
                ironsmith::object::CounterType::Charge,
                3,
                ChooseSpec::SpecificObject(source),
            ),
        );
        assert_eq!(
            pt(&game, source),
            (4, 3),
            "the source is excluded from its own anthem"
        );
        assert_eq!(pt(&game, zombie), (13, 14));
        assert_eq!(
            pt(&game, both),
            (6, 7),
            "a Zombie Mutant is affected only once"
        );
        assert_eq!(pt(&game, other), (1, 2));
        assert_eq!(pt(&game, human), (1, 2));
        apply(
            &mut game,
            source,
            Effect::remove_counters(
                ironsmith::object::CounterType::PlusOnePlusOne,
                2,
                ChooseSpec::SpecificObject(source),
            ),
        );
        assert_eq!(pt(&game, both), (4, 5));
        game.set_current_controller(source, B).unwrap();
        assert_eq!(pt(&game, both), (1, 2));
        assert_eq!(pt(&game, other), (4, 5));
        let mut dm = Choices::default();
        let outcome = apply(&mut game, source, Effect::sacrifice_source());
        queue_outcome(&mut game, outcome, &mut dm);
        assert_eq!(pt(&game, other), (1, 2));
        resolve_all(&mut game, &mut dm);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(source, returned);
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.current_controller(returned), Some(A));
        assert_eq!(pt(&game, zombie), (9, 10));
        assert_eq!(pt(&game, both), (2, 3));
        assert_eq!(pt(&game, other), (1, 2));
    }
}

#[test]
fn glamdring_uses_the_live_exact_host_and_equipment_controller_during_actual_payment() {
    for transported in 0..2 {
        let mut game = game();
        let definition = linked_glamdring(&mut game, transported);
        let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let host = game.create_object_from_definition(
            &vanilla("Equipped reference", "{2}", "Human", 3, 4),
            B,
            Zone::Battlefield,
        );
        let decoy = game.create_object_from_definition(
            &vanilla("Other equipped creature", "{2}", "Human", 20, 20),
            A,
            Zone::Battlefield,
        );
        let other_equipment = game.create_object_from_definition(
            &resource("Other Equipment", "Artifact — Equipment"),
            A,
            Zone::Battlefield,
        );
        attach(&mut game, other_equipment, decoy);
        let spell_def = compile_to_runtime_definition(
            "Cost witness",
            "Mana cost: {7}{R}\nType: Sorcery\nGain 1 life.",
            false,
        )
        .unwrap();
        let mut dm = Choices::default();
        let spell = cast(&mut game, &spell_def, CastingMethod::Normal, &mut dm);
        assert_eq!(
            paid_total(&game, spell),
            8,
            "an unattached source must not borrow another Equipment's host"
        );
        resolve_all(&mut game, &mut dm);
        attach(&mut game, equipment, host);
        let spell = cast(&mut game, &spell_def, CastingMethod::Normal, &mut dm);
        assert_eq!(paid_total(&game, spell), 5);
        resolve_all(&mut game, &mut dm);
        pump(&mut game, equipment, host, 4, 0);
        let spell = cast(&mut game, &spell_def, CastingMethod::Normal, &mut dm);
        assert_eq!(
            paid_total(&game, spell),
            1,
            "the colored pip remains payable"
        );
        resolve_all(&mut game, &mut dm);
        game.set_current_controller(equipment, B).unwrap();
        let spell = cast(&mut game, &spell_def, CastingMethod::Normal, &mut dm);
        assert_eq!(
            paid_total(&game, spell),
            8,
            "you is the Equipment's current controller"
        );
        resolve_all(&mut game, &mut dm);
        game.set_current_controller(equipment, A).unwrap();
        pump(&mut game, equipment, host, -10, 0);
        let spell = cast(&mut game, &spell_def, CastingMethod::Normal, &mut dm);
        assert_eq!(
            paid_total(&game, spell),
            8,
            "negative power does not increase cost"
        );
        resolve_all(&mut game, &mut dm);
        let creature = vanilla("Excluded creature spell", "{7}{R}", "Human", 1, 1);
        attach(&mut game, equipment, decoy);
        let spell = cast(&mut game, &creature, CastingMethod::Normal, &mut dm);
        assert_eq!(
            paid_total(&game, spell),
            8,
            "only instant or sorcery spells are reduced"
        );
    }
}

#[test]
fn glamdrings_adventure_face_mills_filters_and_exiles_then_allows_the_artifact_cast() {
    for transported in 0..2 {
        let mut game = game();
        let definition = linked_glamdring(&mut game, transported);
        for types in [
            "Instant",
            "Sorcery",
            "Instant Sorcery",
            "Artifact",
            "Creature — Human",
            "Land",
        ] {
            game.create_object_from_definition(&resource("Milled card", types), A, Zone::Library);
        }
        let mut dm = Choices::default();
        let spell = cast(
            &mut game,
            &definition,
            CastingMethod::SplitOtherHalf,
            &mut dm,
        );
        assert_eq!(game.object(spell).unwrap().name.as_ref(), "Gleam of Death");
        assert_eq!(paid_total(&game, spell), 4);
        let stable = game.object(spell).unwrap().stable_id;
        resolve_all(&mut game, &mut dm);
        assert!(game.player(A).unwrap().library.is_empty());
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert_eq!(game.adventure_exiled_player(exiled), Some(A));
        let spell = cast_existing(
            &mut game,
            exiled,
            Zone::Exile,
            CastingMethod::Normal,
            &mut dm,
        );
        assert_eq!(paid_total(&game, spell), 2);
        assert_eq!(
            game.object(spell).unwrap().name.as_ref(),
            "Glamdring, Foe-hammer"
        );
        resolve_all(&mut game, &mut dm);
        let equipment = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.object_has_card_type(equipment, ironsmith::CardType::Artifact));
        assert!(
            game.calculated_subtypes(equipment)
                .contains(&ironsmith::Subtype::Equipment)
        );
    }
}
