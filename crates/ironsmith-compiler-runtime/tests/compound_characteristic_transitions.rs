//! UNVALIDATED full-card compound characteristic transitions and exact target ownership.
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
        "../../../fixtures/compound_characteristic_transitions.json.fixture"
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
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
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
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index: index }
            | LegalAction::ActivateManaAbility { source: id, ability_index: index }
            if *id == source && *index == ability_index)
        })
        .expect("the current ability must be legally activatable");
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
#[test]
fn eight_full_frozen_card_artifacts_round_trip_without_loss() {
    let rows = fixtures()
        .into_iter()
        .filter(|r| r["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 8);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn dragonsoul_and_paragon_pay_actual_five_color_cost_and_all_three_effects_expire_together() {
    use ironsmith::Subtype;
    use ironsmith::static_abilities::StaticAbilityId as Id;
    for (name, subtype, gain, expected) in [
        ("Dragonsoul Knight", Subtype::Dragon, Id::Trample, (7, 5)),
        (
            "Paragon of the Amesha",
            Subtype::Angel,
            Id::Lifelink,
            (5, 5),
        ),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let before = game.player(A).unwrap().mana_pool.total();
            activate(
                &mut game,
                s,
                activated_at(&definition, 0),
                &mut Choices::default(),
            );
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 5);
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(pt(&game, s), expected);
            assert!(game.current_subtypes(s).unwrap().contains(&subtype));
            assert!(!game.current_subtypes(s).unwrap().contains(&Subtype::Knight));
            for a in [Id::Flying, gain, Id::FirstStrike] {
                assert!(has(&game, s, a));
            }
            ironsmith::turn::execute_cleanup_step(&mut game);
            game.refresh_continuous_state().unwrap();
            assert_eq!(pt(&game, s), (2, 2));
            assert!(has(&game, s, Id::FirstStrike));
            assert!(!has(&game, s, Id::Flying));
            assert!(game.current_subtypes(s).unwrap().contains(&Subtype::Knight));
        }
    }
}
#[test]
fn defiling_tears_announces_one_target_then_keeps_color_pump_and_usable_regeneration() {
    for definition in definitions("Defiling Tears") {
        let mut game = game();
        let target = game.create_object_from_definition(
            &vanilla("Witness", "{2}", "Elf", 2, 2),
            A,
            Zone::Battlefield,
        );
        let untouched = game.create_object_from_definition(
            &vanilla("Untouched", "{2}", "Elf", 2, 2),
            A,
            Zone::Battlefield,
        );
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(pt(&game, target), (3, 1));
        assert_eq!(pt(&game, untouched), (2, 2));
        assert_eq!(
            game.current_colors(target),
            Some(ironsmith::color::ColorSet::BLACK)
        );
        let ability = game
            .calculated_characteristics(target)
            .unwrap()
            .abilities
            .iter()
            .position(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .unwrap();
        activate(&mut game, target, ability, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        apply(
            &mut game,
            target,
            Effect::destroy(ChooseSpec::SpecificObject(target)),
        );
        assert_eq!(game.object(target).unwrap().zone, Zone::Battlefield);
        assert!(game.is_tapped(target));
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(pt(&game, target), (2, 2));
    }
}
#[test]
fn kellan_subtype_only_levels_keep_artifact_type_and_base_size_and_retained_draw_permission() {
    use ironsmith::static_abilities::StaticAbilityId as Id;
    use ironsmith::{CardType, Subtype};
    for definition in definitions("Kellan, Planar Trailblazer") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let witness = game.create_object_from_definition(
            &vanilla("Permission witness", "{1}", "Human", 1, 1),
            A,
            Zone::Library,
        );
        let stable = game.object(witness).unwrap().stable_id;
        apply(
            &mut game,
            s,
            Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
                ironsmith::continuous::EffectTarget::Specific(s),
                ironsmith::continuous::Modification::AddCardTypes(vec![CardType::Artifact]),
                Until::Forever,
            )),
        );
        activate(
            &mut game,
            s,
            activated_at(&definition, 0),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.object_has_card_type(s, CardType::Artifact));
        assert_eq!(pt(&game, s), (2, 1));
        assert!(
            game.current_subtypes(s)
                .unwrap()
                .contains(&Subtype::Detective)
        );
        assert!(!game.current_subtypes(s).unwrap().contains(&Subtype::Scout));
        activate(
            &mut game,
            s,
            activated_at(&definition, 1),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(pt(&game, s), (3, 2));
        assert!(has(&game, s, Id::DoubleStrike));
        assert!(game.object_has_card_type(s, CardType::Artifact));
        let outcome = apply(
            &mut game,
            s,
            Effect::new(
                ironsmith::effects::DealDamageEffect::new(
                    1,
                    ChooseSpec::Player(PlayerFilter::Specific(B)),
                )
                .with_combat(true),
            ),
        );
        queue_outcome(&mut game, outcome, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        game.turn.priority_player = Some(A);
        assert!(compute_legal_actions(&game,A).unwrap().iter().any(|a|matches!(a,LegalAction::CastSpell {spell_id,from_zone:Zone::Exile,..} if *spell_id==exiled)));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(
            !compute_legal_actions(&game, A)
                .unwrap()
                .iter()
                .any(|a| matches!(a,LegalAction::CastSpell {spell_id,..} if *spell_id==exiled))
        );
    }
}
#[test]
fn possessed_goat_pays_discard_keeps_original_color_and_type_and_is_once_per_incarnation() {
    use ironsmith::Subtype;
    use ironsmith::color::ColorSet;
    for definition in definitions("Possessed Goat") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let paid = game.create_object_from_definition(
            &vanilla("Discarded", "{1}", "Human", 1, 1),
            A,
            Zone::Hand,
        );
        game.create_object_from_definition(
            &vanilla("Other discard", "{1}", "Human", 1, 1),
            A,
            Zone::Hand,
        );
        activate(
            &mut game,
            s,
            activated_at(&definition, 0),
            &mut Choices {
                objects: vec![paid],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.object(paid).is_none());
        assert_eq!(
            game.counter_count(s, ironsmith::object::CounterType::PlusOnePlusOne),
            3
        );
        assert_eq!(
            game.current_colors(s),
            Some(ColorSet::WHITE.union(ColorSet::BLACK))
        );
        let types = game.current_subtypes(s).unwrap();
        assert!(types.contains(&Subtype::Goat) && types.contains(&Subtype::Demon));
        let action = LegalAction::ActivateAbility {
            source: s,
            ability_index: activated_at(&definition, 0),
        };
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.turn.turn_number += 1;
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 10);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
        let exile = game.move_object_by_game_rule(s, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_game_rule(exile, Zone::Battlefield)
            .unwrap();
        assert!(
            compute_legal_actions(&game, A)
                .unwrap()
                .contains(&LegalAction::ActivateAbility {
                    source: returned,
                    ability_index: activated_at(&definition, 0)
                })
        );
    }
}
#[test]
fn surge_engine_gates_each_paid_stage_and_only_the_draw_is_lifetime_limited() {
    use ironsmith::CardType;
    use ironsmith::static_abilities::StaticAbilityId as Id;
    for definition in definitions("Surge Engine") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..6 {
            game.create_object_from_definition(
                &vanilla("Library", "{1}", "Human", 1, 1),
                A,
                Zone::Library,
            );
        }
        let action = |game: &GameState, n| LegalAction::ActivateAbility {
            source: s,
            ability_index: current_activated_at(game, s, n),
        };
        assert!(
            !compute_legal_actions(&game, A)
                .unwrap()
                .contains(&action(&game, 1))
        );
        assert!(
            !compute_legal_actions(&game, A)
                .unwrap()
                .contains(&action(&game, 2))
        );
        let index = current_activated_at(&game, s, 0);
        activate(&mut game, s, index, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        assert!(!has(&game, s, Id::Defender));
        let index = current_activated_at(&game, s, 1);
        activate(&mut game, s, index, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(pt(&game, s), (5, 4));
        assert!(game.object_has_card_type(s, CardType::Artifact));
        assert!(game.object_has_card_type(s, CardType::Creature));
        assert_eq!(
            game.current_colors(s),
            Some(ironsmith::color::ColorSet::BLUE)
        );
        let index = current_activated_at(&game, s, 2);
        activate(&mut game, s, index, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        assert!(
            !compute_legal_actions(&game, A)
                .unwrap()
                .contains(&action(&game, 2))
        );
        assert!(
            compute_legal_actions(&game, A)
                .unwrap()
                .contains(&action(&game, 1))
        );
    }
}
#[test]
fn origin_saga_all_chapters_keep_target_identity_legendary_addition_and_temporary_double_strike() {
    use ironsmith::static_abilities::StaticAbilityId as Id;
    use ironsmith::{Subtype, Supertype};
    for definition in definitions("Origin of Spider-Man") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        lore(&mut game, s, &mut Choices::default());
        let spider = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                *id != s
                    && game
                        .current_subtypes(*id)
                        .unwrap()
                        .contains(&Subtype::Spider)
            })
            .unwrap();
        assert_eq!(pt(&game, spider), (2, 1));
        assert!(has(&game, spider, Id::Reach));
        let hero = game.create_object_from_definition(
            &vanilla("Hero subject", "{2}", "Elf", 2, 2),
            A,
            Zone::Battlefield,
        );
        lore(
            &mut game,
            s,
            &mut Choices {
                targets: vec![Target::Object(hero)],
                ..Default::default()
            },
        );
        assert_eq!(pt(&game, hero), (3, 3));
        assert!(
            game.current_supertypes(hero)
                .unwrap()
                .contains(&Supertype::Legendary)
        );
        for subtype in [Subtype::Elf, Subtype::Spider, Subtype::Hero] {
            assert!(game.current_subtypes(hero).unwrap().contains(&subtype));
        }
        lore(
            &mut game,
            s,
            &mut Choices {
                targets: vec![Target::Object(hero)],
                ..Default::default()
            },
        );
        assert!(has(&game, hero, Id::DoubleStrike));
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert!(!has(&game, hero, Id::DoubleStrike));
        assert!(
            game.current_supertypes(hero)
                .unwrap()
                .contains(&Supertype::Legendary)
        );
    }
}
#[test]
fn kitesail_template_clears_old_types_and_abilities_grants_mana_and_expires_with_exact_source() {
    use ironsmith::static_abilities::StaticAbilityId as Id;
    use ironsmith::{CardType, Subtype};
    for definition in definitions("Kitesail Larcenist") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let witness = compile_to_runtime_definition(
            "Flying witness",
            "Mana cost: {1}\nType: Artifact Creature — Bird\nPower/Toughness: 1/1\nFlying",
            false,
        )
        .unwrap();
        let targets = [A, B, C]
            .map(|owner| game.create_object_from_definition(&witness, owner, Zone::Battlefield));
        event(
            &mut game,
            TriggerEvent::new(
                ironsmith::events::ZoneChangeEvent::with_cause(
                    s,
                    Zone::Hand,
                    Zone::Battlefield,
                    ironsmith::events::cause::EventCause::effect(),
                    None,
                ),
                Default::default(),
            ),
            &mut Choices {
                targets: targets.into_iter().map(Target::Object).collect(),
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        for id in targets {
            assert!(game.object_has_card_type(id, CardType::Artifact));
            assert!(!game.object_has_card_type(id, CardType::Creature));
            assert!(
                game.current_subtypes(id)
                    .unwrap()
                    .contains(&Subtype::Treasure)
            );
            assert!(
                game.current_subtypes(id)
                    .unwrap()
                    .iter()
                    .all(|t| *t != Subtype::Bird)
            );
            assert!(!has(&game, id, Id::Flying));
        }
        assert!(has(&game, s, Id::Flying));
        let mana_index = game
            .calculated_characteristics(targets[0])
            .unwrap()
            .abilities
            .iter()
            .position(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .unwrap();
        activate(&mut game, targets[0], mana_index, &mut Choices::default());
        assert!(game.object(targets[0]).is_none());
        game.move_object_by_game_rule(s, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        for id in targets.into_iter().skip(1) {
            assert!(game.object_has_card_type(id, CardType::Creature));
            assert!(game.current_subtypes(id).unwrap().contains(&Subtype::Bird));
            assert!(has(&game, id, Id::Flying));
        }
    }
}
