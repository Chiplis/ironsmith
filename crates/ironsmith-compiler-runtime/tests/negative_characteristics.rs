//! UNVALIDATED full-card Snow/type-negation scenarios. No execution under deferred workflow.
use ironsmith::ability::ActivatedAbilityRuntimeExt;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
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
        "../../../fixtures/negative_characteristics.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
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
    mode: usize,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose mode") {
            return vec![ctx.options[self.mode].index];
        }
        if ctx.description.starts_with("Choose optional costs") {
            if self.decline {
                return vec![];
            }
            return ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
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
    let payer = game.current_controller(source).unwrap();
    game.turn.priority_player = Some(payer);
    let ability = game
        .current_activated_ability(source, ability_index)
        .unwrap();
    let action = if ability.is_runtime_mana_ability(game, source, payer) {
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        }
    } else {
        LegalAction::ActivateAbility {
            source,
            ability_index,
        }
    };
    assert!(
        compute_legal_actions(game, payer)
            .unwrap()
            .contains(&action)
    );
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
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn activated_at(definition: &CardDefinition, position: usize) -> usize {
    definition
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn current_activated_at(game: &GameState, source: ObjectId, position: usize) -> usize {
    game.calculated_characteristics(source)
        .unwrap()
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn has(
    game: &GameState,
    id: ObjectId,
    ability: ironsmith::static_abilities::StaticAbilityId,
) -> bool {
    game.current_has_static_ability_id(id, ability)
}
fn event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    for trigger in ironsmith::triggers::check_delayed_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn lore(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let outcome = apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::Lore,
            1,
            ChooseSpec::SpecificObject(source),
        ),
    );
    queue_outcome(game, outcome, dm);
    resolve_all(game, &mut Choices::default());
}
fn snow(game: &GameState, id: ObjectId) -> bool {
    game.current_has_supertype(id, ironsmith::Supertype::Snow)
}
fn permanent(game: &mut GameState, owner: PlayerId, name: &str, types: &str) -> ObjectId {
    game.create_object_from_definition(&resource(name, types), owner, Zone::Battlefield)
}
fn cleanup(game: &mut GameState) {
    ironsmith::turn::execute_cleanup_step(game);
    game.refresh_continuous_state().unwrap();
}
fn upkeep(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
    event(
        game,
        TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfUpkeepEvent::new(player),
            Default::default(),
        ),
        &mut Choices::default(),
    );
}
#[test]
fn five_exact_full_bodies_round_trip_without_loss() {
    let rows = fixtures()
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 5);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn weathervane_pays_both_tap_costs_and_changes_only_the_exact_land_until_it_leaves() {
    for definition in definitions("Arcum's Weathervane") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let land = permanent(&mut game, B, "Snow basic land", "Basic Snow Land — Forest");
        let other = permanent(&mut game, A, "Other snow", "Snow Artifact");
        let before = game.player(A).unwrap().mana_pool.total();
        activate(
            &mut game,
            host,
            activated_at(&definition, 0),
            &mut Choices {
                targets: vec![Target::Object(land)],
                ..Default::default()
            },
        );
        assert!(game.is_tapped(host));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 2);
        resolve_all(&mut game, &mut Choices::default());
        assert!(!snow(&game, land));
        assert!(snow(&game, other));
        assert!(game.current_has_supertype(land, ironsmith::Supertype::Basic));
        cleanup(&mut game);
        assert!(!snow(&game, land));
        game.untap(host);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        activate(
            &mut game,
            host,
            activated_at(&definition, 1),
            &mut Choices {
                targets: vec![Target::Object(land)],
                ..Default::default()
            },
        );
        assert!(game.is_tapped(host));
        resolve_all(&mut game, &mut Choices::default());
        assert!(snow(&game, land));
        let exile = game.move_object_by_game_rule(land, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_game_rule(exile, Zone::Battlefield)
            .unwrap();
        assert_ne!(returned, land);
        assert!(snow(&game, returned));
    }
}
#[test]
fn melting_is_a_live_land_only_static_effect_and_phasing_retains_its_duration() {
    for definition in definitions("Melting") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let land = permanent(&mut game, B, "Affected", "Basic Snow Land — Island");
        let artifact = permanent(&mut game, B, "Unaffected", "Snow Artifact");
        assert!(!snow(&game, land));
        assert!(snow(&game, artifact));
        let later = permanent(&mut game, C, "Later entrant", "Snow Land");
        assert!(!snow(&game, later));
        game.phase_out(host);
        assert!(snow(&game, land));
        game.phase_in(host);
        assert!(!snow(&game, land));
        game.move_object_by_game_rule(host, Zone::Graveyard)
            .unwrap();
        assert!(snow(&game, land));
        assert!(snow(&game, later));
    }
}
#[test]
fn glittering_frost_cast_attaches_and_both_actual_mana_units_have_snow_provenance() {
    for definition in definitions("Glittering Frost") {
        let mut game = game();
        let land = permanent(&mut game, A, "Originally plain", "Basic Land — Forest");
        let spell = cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                targets: vec![Target::Object(land)],
                ..Default::default()
            },
        );
        let stable = game.object(spell).unwrap().stable_id;
        resolve_all(&mut game, &mut Choices::default());
        let aura = game.find_object_by_stable_id(stable).unwrap();
        assert!(snow(&game, land));
        game.set_current_controller(land, B).unwrap();
        game.player_mut(B).unwrap().mana_pool.empty();
        let mana=game.calculated_characteristics(land).unwrap().abilities.iter().enumerate().find_map(|(i,a)|
            matches!(a.kind,ironsmith::ability::AbilityKind::Activated(ref v) if v.is_mana_ability()).then_some(i)).unwrap();
        activate(&mut game, land, mana, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 2);
        let snow_cost =
            ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Snow, ManaSymbol::Snow]);
        assert!(game.can_pay_mana_cost(B, None, &snow_cost, 0));
        game.move_object_by_game_rule(aura, Zone::Graveyard)
            .unwrap();
        assert!(!snow(&game, land));
        assert!(
            game.can_pay_mana_cost(B, None, &snow_cost, 0),
            "spent source properties are retained at production, not reread after Aura departure"
        );
    }
}
#[test]
fn thermal_flux_keeps_both_modes_temporary_and_draws_only_at_the_next_turns_upkeep() {
    for definition in definitions("Thermal Flux") {
        for mode in 0..2 {
            let mut game = game();
            let target = permanent(
                &mut game,
                B,
                "Mode target",
                if mode == 0 {
                    "Artifact"
                } else {
                    "Snow Artifact"
                },
            );
            for _ in 0..3 {
                game.create_object_from_definition(
                    &vanilla("Library", "{1}", "Human", 1, 1),
                    A,
                    Zone::Library,
                );
            }
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    mode,
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(snow(&game, target), mode == 0);
            assert!(game.player(A).unwrap().hand.is_empty());
            upkeep(&mut game, B);
            assert!(
                game.stack_is_empty(),
                "an extra upkeep this turn is too early"
            );
            cleanup(&mut game);
            assert_eq!(snow(&game, target), mode == 1);
            game.turn.turn_number += 1;
            upkeep(&mut game, C);
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert!(game.player(C).unwrap().hand.is_empty());
            upkeep(&mut game, C);
            assert!(game.stack_is_empty(), "the delayed draw is one shot");
        }
    }
}
#[test]
fn neurok_has_one_announced_target_and_preserves_creature_subtypes_and_abilities() {
    for definition in definitions("Neurok Transmuter") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = game.create_object_from_definition(
            &vanilla("Red creature", "{R}", "Elf", 2, 2),
            B,
            Zone::Battlefield,
        );
        let original = game.current_colors(target);
        activate(
            &mut game,
            host,
            activated_at(&definition, 0),
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.object_has_card_type(target, ironsmith::CardType::Artifact));
        activate(
            &mut game,
            host,
            activated_at(&definition, 1),
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        assert_eq!(
            game.stack.last().unwrap().targets,
            vec![Target::Object(target)]
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(!game.object_has_card_type(target, ironsmith::CardType::Artifact));
        assert!(game.object_has_card_type(target, ironsmith::CardType::Creature));
        assert!(
            game.current_subtypes(target)
                .unwrap()
                .contains(&ironsmith::Subtype::Elf)
        );
        assert_eq!(
            game.current_colors(target),
            Some(ironsmith::color::ColorSet::BLUE)
        );
        assert_eq!(pt(&game, target), (2, 2));
        cleanup(&mut game);
        assert_eq!(game.current_colors(target), original);
        assert!(!game.object_has_card_type(target, ironsmith::CardType::Artifact));
    }
}
#[test]
fn generic_source_color_and_type_negation_does_not_invent_a_previous_target() {
    for definition in definitions_text(
        "Self modifier",
        "Type: Artifact Creature — Construct\nPower/Toughness: 1/1\n{U}: Until end of turn, this creature becomes blue and isn't an artifact.",
    ) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        activate(
            &mut game,
            host,
            activated_at(&definition, 0),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(!game.object_has_card_type(host, ironsmith::CardType::Artifact));
        assert_eq!(
            game.current_colors(host),
            Some(ironsmith::color::ColorSet::BLUE)
        );
    }
}
