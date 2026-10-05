//! UNVALIDATED extrema values, tie scopes and resolution-time choice regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/extrema_quantities.json.fixture"
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
fn game_with_starting_life(starting: i32) -> GameState {
    let mut game = GameState::new(
        vec!["Alice".into(), "Bob".into(), "Charlie".into()],
        starting,
    );
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
    optional: Option<bool>,
    kicker_payments: usize,
    decline_targets: bool,
    bounds: Vec<(usize, Option<usize>)>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        if let Some(choice) = self.optional {
            return choice;
        }
        let _ = ctx;
        true
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose optional costs for") {
            return context
                .options
                .iter()
                .filter(|option| option.legal)
                .take(self.kicker_payments)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.bounds.extend(
            context
                .requirements
                .iter()
                .map(|r| (r.min_targets, r.max_targets)),
        );
        if self.decline_targets {
            return Vec::new();
        }
        if !self.targets.is_empty() {
            assert!(self.targets.iter().all(|target| {
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            }));
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
        if !self.objects.is_empty() {
            let selected = self
                .objects
                .iter()
                .copied()
                .filter(|id| {
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                })
                .take(context.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>();
            assert!(
                selected.len() >= context.min,
                "requested cost objects must satisfy the current payment decision"
            );
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
    ironsmith::game_loop::check_and_apply_sbas_with(game, &mut queue, dm).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
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
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
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
        .rev()
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
use ironsmith::static_abilities::StaticAbilityId;
fn game() -> GameState {
    game_with_starting_life(20)
}
fn witness(game: &mut GameState) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("History witness", "{1}", "Human", 1, 1),
        A,
        Zone::Battlefield,
    )
}
fn library(game: &mut GameState, count: usize) {
    for _ in 0..count {
        game.create_object_from_definition(
            &vanilla("Library witness", "{1}", "Human", 1, 1),
            A,
            Zone::Library,
        );
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = cast(game, definition, CastingMethod::Normal, dm);
    let stable = game.object(spell).unwrap().stable_id;
    resolve_all(game, dm);
    game.find_object_by_stable_id(stable).unwrap()
}
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.calculated_power(id).unwrap(),
        game.calculated_toughness(id).unwrap(),
    )
}
fn has(game: &GameState, id: ObjectId, ability: StaticAbilityId) -> bool {
    game.current_has_static_ability_id(id, ability)
}

fn creature(game: &mut GameState, player: PlayerId, name: &str, p: i32, t: i32) -> ObjectId {
    game.create_object_from_definition(
        &vanilla(name, "{1}", "Human", p, t),
        player,
        Zone::Battlefield,
    )
}
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}

fn attack(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    game.remove_summoning_sickness(source);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::AttackerDeclaration {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
#[test]
fn twelve_quantity_candidates_keep_full_metadata_and_artifact_semantics() {
    for name in [
        "Freelance Muscle",
        "Investigator's Journal",
        "Repay in Kind",
        "Wretched Banquet",
        "Dispersal Shield",
        "Strength-Testing Hammer",
        "Desecrator Hag",
        "Drop of Honey",
        "Porphyry Nodes",
        "Purging Scythe",
        "Cabal Conditioning",
        "Gor Muldrak, Amphinologist",
    ] {
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
        }
    }
}
#[test]
fn freelance_muscle_uses_the_greatest_current_axis_of_other_controlled_creatures_at_resolution() {
    for definition in definitions("Freelance Muscle") {
        for other in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let base = pt(&game, source);
            let yours = creature(&mut game, A, "Other axes", 2, 7);
            let theirs = creature(&mut game, B, "Enemy axes", 15, 16);
            if !other {
                game.set_current_controller(yours, B).unwrap();
            }
            attack(&mut game, source, &mut Choices::default());
            if other {
                // A current-characteristic change after triggering is read
                // on resolution; a greater opposing axis must not enter scope.
                apply(
                    &mut game,
                    theirs,
                    Effect::pump(7, 0, ChooseSpec::SpecificObject(yours), Until::EndOfTurn),
                );
            }
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                pt(&game, source),
                (
                    base.0 + if other { 9 } else { 0 },
                    base.1 + if other { 9 } else { 0 }
                )
            );
            assert_eq!(pt(&game, theirs), (15, 16));
        }
    }
}
#[test]
fn investigators_journal_counts_the_largest_single_players_population_and_both_draw_costs_execute()
{
    for definition in definitions("Investigator's Journal") {
        let mut game = game();
        library(&mut game, 4);
        creature(&mut game, A, "Alice creature", 1, 1);
        for _ in 0..3 {
            creature(&mut game, B, "Bob creature", 1, 1);
        }
        for _ in 0..2 {
            creature(&mut game, C, "Charlie creature", 1, 1);
        }
        let journal = enter(&mut game, &definition, &mut Choices::default());
        let suspect = ironsmith::object::CounterType::Named("suspect".into());
        assert_eq!(
            game.counter_count(journal, suspect.clone()),
            3,
            "greatest single-player count, not six creatures globally"
        );
        activate(
            &mut game,
            journal,
            activated(&definition),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.counter_count(journal, suspect), 2);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        let sacrifice_index = definition
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, ability)| {
                matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_))
            })
            .nth(1)
            .unwrap()
            .0;
        activate(&mut game, journal, sacrifice_index, &mut Choices::default());
        resolve_all(&mut game, &mut Choices::default());
        assert!(game.object(journal).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
    }
}
#[test]
fn repay_in_kind_freezes_the_original_minimum_before_any_life_change_replacement() {
    for definition in definitions("Repay in Kind") {
        let mut game = game();
        let source = witness(&mut game);
        apply(
            &mut game,
            source,
            Effect::set_life_total_player(8, PlayerFilter::Specific(B)),
        );
        apply(
            &mut game,
            source,
            Effect::set_life_total_player(12, PlayerFilter::Specific(C)),
        );
        // A real replacement transforms Alice's loss to 24, leaving a new
        // minimum -4. Bob and Charlie must still use the original minimum 8.
        game.effect_store.replacement_effects.add_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                A,
                ironsmith::events::life::matchers::WouldLoseLifeMatcher::you(),
                ironsmith::replacement::ReplacementAction::Double,
            ),
        );
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, -4);
        assert_eq!(game.player(B).unwrap().life, 8);
        assert_eq!(game.player(C).unwrap().life, 8);
    }
}

#[test]
fn wretched_banquet_announces_any_creature_then_checks_the_current_minimum_including_ties() {
    for definition in definitions("Wretched Banquet") {
        for mode in 0..3 {
            let mut game = game();
            let target = creature(
                &mut game,
                B,
                "Conditional victim",
                if mode == 0 { 4 } else { 1 },
                4,
            );
            let other = creature(&mut game, C, "Other minimum", 1, 4);
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            if mode == 0 {
                apply(
                    &mut game,
                    other,
                    Effect::pump(-4, 0, ChooseSpec::SpecificObject(target), Until::EndOfTurn),
                );
            }
            if mode == 1 {
                apply(
                    &mut game,
                    other,
                    Effect::pump(3, 0, ChooseSpec::SpecificObject(target), Until::EndOfTurn),
                );
            }
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.object(target).is_some(), mode == 1);
            assert!(
                game.object(other).is_some(),
                "equality tests membership, it does not destroy all tied creatures"
            );
        }
    }
}
#[test]
fn dispersal_shield_keeps_unconditional_spell_targeting_and_rechecks_the_controlled_maximum() {
    for definition in definitions("Dispersal Shield") {
        for mode in 0..3 {
            let mut game = game();
            library(&mut game, 3);
            let permanent = compile_to_runtime_definition(
                "Mana value reference",
                format!(
                    "Mana cost: {{{}}}\nType: Artifact",
                    if mode == 2 { 1 } else { 5 }
                ),
                false,
            )
            .unwrap();
            let reference = game.create_object_from_definition(&permanent, A, Zone::Battlefield);
            let spell = compile_to_runtime_definition(
                "Conditional counter target",
                "Mana cost: {4}\nType: Instant\nDraw a card.",
                false,
            )
            .unwrap();
            let target = cast(
                &mut game,
                &spell,
                CastingMethod::Normal,
                &mut Choices::default(),
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
            if mode == 1 {
                apply(
                    &mut game,
                    reference,
                    Effect::destroy(ChooseSpec::SpecificObject(reference)),
                );
            }
            resolve(&mut game, &mut Choices::default());
            assert_eq!(
                game.stack.iter().any(|entry| entry.object_id == target),
                mode != 0
            );
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(mode != 0));
        }
    }
}
#[test]
fn strength_testing_hammer_rolls_for_the_actual_equipped_attacker_then_tests_its_modified_power() {
    for definition in definitions("Strength-Testing Hammer") {
        for opposing_power in [0, 8, 20] {
            let mut game = game();
            game.set_random_seed(41);
            library(&mut game, 3);
            let hammer = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let attacker = creature(&mut game, A, "Hammer attacker", 4, 4);
            let enemy = creature(&mut game, B, "Extremum competitor", opposing_power, 30);
            activate(
                &mut game,
                hammer,
                activated(&definition),
                &mut Choices {
                    targets: vec![Target::Object(attacker)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            attack(&mut game, attacker, &mut Choices::default());
            resolve_all(&mut game, &mut Choices::default());
            let (power, toughness) = pt(&game, attacker);
            assert!((5..=10).contains(&power));
            assert_eq!(toughness, 4);
            assert_eq!(pt(&game, enemy), (opposing_power, 30));
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                usize::from(power >= opposing_power)
            );
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(pt(&game, attacker), (4, 4));
        }
    }
}

#[test]
fn hammer_does_not_pump_a_blinked_attacker_and_its_condition_uses_exact_departure_power() {
    for definition in definitions("Strength-Testing Hammer") {
        let mut game = game();
        library(&mut game, 3);
        let hammer = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = creature(&mut game, A, "Blinking attacker", 4, 4);
        let enemy = creature(&mut game, B, "Equal to departure power", 14, 30);
        activate(
            &mut game,
            hammer,
            activated(&definition),
            &mut Choices {
                targets: vec![Target::Object(attacker)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        attack(&mut game, attacker, &mut Choices::default());
        let stable = game.object(attacker).unwrap().stable_id;
        apply(
            &mut game,
            enemy,
            Effect::pump(
                10,
                0,
                ChooseSpec::SpecificObject(attacker),
                Until::EndOfTurn,
            ),
        );
        let exiled = game
            .move_object_by_game_rule(attacker, Zone::Exile)
            .unwrap();
        let returned = game
            .move_object_by_game_rule(exiled, Zone::Battlefield)
            .unwrap();
        assert_eq!(game.object(returned).unwrap().stable_id, stable);
        assert_ne!(returned, attacker);
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(
            pt(&game, returned),
            (4, 4),
            "the independent pending attack trigger cannot modify the new incarnation"
        );
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            1,
            "actual departure power 14 ties the remaining maximum; the earlier attack snapshot had only 4"
        );
    }
}
#[test]
fn an_explicit_move_in_the_same_resolution_still_follows_the_object_it_moved() {
    for definition in definitions_text(
        "Linked move variant",
        "Mana cost: {1}\nType: Instant\nExile target creature. Return that card to the battlefield under its owner's control.",
    ) {
        let mut game = game();
        let original = creature(&mut game, B, "Moved by this spell", 2, 4);
        let stable = game.object(original).unwrap().stable_id;
        cast(
            &mut game,
            &definition,
            CastingMethod::Normal,
            &mut Choices {
                targets: vec![Target::Object(original)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(returned, original);
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.current_controller(returned), Some(B));
    }
}

fn upkeep(game: &mut GameState, dm: &mut Choices) {
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::BeginningOfUpkeepEvent::new(A),
        Default::default(),
    );
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
#[test]
fn desecrator_hag_chooses_one_tied_graveyard_maximum_in_its_own_scope_including_negative_power() {
    for definition in definitions("Desecrator Hag") {
        for maximum in [5, -1] {
            let mut game = game();
            let tied = game.create_object_from_definition(
                &vanilla("First graveyard maximum", "{1}", "Human", maximum, 4),
                A,
                Zone::Graveyard,
            );
            let selected = game.create_object_from_definition(
                &vanilla("Chosen graveyard maximum", "{1}", "Human", maximum, 4),
                A,
                Zone::Graveyard,
            );
            let lesser = game.create_object_from_definition(
                &vanilla("Lesser graveyard power", "{1}", "Human", maximum - 2, 4),
                A,
                Zone::Graveyard,
            );
            let opposing = game.create_object_from_definition(
                &vanilla("Opponent graveyard maximum", "{1}", "Human", 20, 4),
                B,
                Zone::Graveyard,
            );
            let stable = game.object(selected).unwrap().stable_id;
            let mut dm = Choices {
                objects: vec![selected],
                ..Default::default()
            };
            enter(&mut game, &definition, &mut dm);
            let moved = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(moved).unwrap().zone, Zone::Hand);
            for unchanged in [tied, lesser, opposing] {
                assert_eq!(game.object(unchanged).unwrap().zone, Zone::Graveyard);
            }
            assert!(
                dm.bounds.is_empty(),
                "the resolution choice is not a target declaration"
            );
        }
    }
}
#[test]
fn purging_scythe_damages_only_the_chosen_tied_minimum_and_an_empty_set_does_nothing() {
    for definition in definitions("Purging Scythe") {
        for empty in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut creatures = Vec::new();
            if !empty {
                creatures.push(creature(&mut game, B, "First toughness minimum", 2, 3));
                creatures.push(creature(&mut game, C, "Chosen toughness minimum", 4, 3));
                creatures.push(creature(&mut game, B, "Greater toughness", 1, 8));
            }
            let mut dm = Choices {
                objects: creatures.get(1).copied().into_iter().collect(),
                ..Default::default()
            };
            upkeep(&mut game, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert!(game.object(source).is_some());
            if !empty {
                assert_eq!(game.damage_on(creatures[0]), 0);
                assert_eq!(game.damage_on(creatures[1]), 2);
                assert_eq!(game.damage_on(creatures[2]), 0);
            }
            assert!(dm.bounds.is_empty());
        }
    }
}
#[test]
fn drop_and_nodes_choose_one_without_targeting_prevent_regeneration_and_keep_true_state_trigger_lifecycle()
 {
    for name in ["Drop of Honey", "Porphyry Nodes"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let hexproof = compile_to_runtime_definition(
                "Untargetable minimum",
                "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/4\nHexproof",
                false,
            )
            .unwrap();
            let victim = game.create_object_from_definition(&hexproof, B, Zone::Battlefield);
            let tied = creature(&mut game, C, "Other power minimum", 1, 4);
            apply(
                &mut game,
                source,
                Effect::regenerate(ChooseSpec::SpecificObject(victim), Until::EndOfTurn),
            );
            let mut dm = Choices {
                objects: vec![victim],
                ..Default::default()
            };
            upkeep(&mut game, &mut dm);
            resolve_all(&mut game, &mut dm);
            assert!(
                game.object(victim).is_none(),
                "a regeneration shield cannot replace this destruction"
            );
            assert!(game.object(tied).is_some());
            assert!(
                dm.bounds.is_empty(),
                "hexproof does not prohibit a nontargeted tie choice"
            );
            let outcome = apply(
                &mut game,
                source,
                Effect::destroy(ChooseSpec::SpecificObject(tied)),
            );
            queue_outcome(&mut game, outcome, &mut Choices::default());
            assert_eq!(
                game.stack.len(),
                1,
                "empty-battlefield state trigger becomes pending"
            );
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
            assert_eq!(
                game.stack.len(),
                1,
                "state scans cannot duplicate an already pending instance"
            );
            let entering = game.create_object_from_definition(
                &vanilla("Response creature", "{1}", "Human", 2, 4),
                B,
                Zone::Hand,
            );
            let entered = game
                .move_object_by_game_rule(entering, Zone::Battlefield)
                .unwrap();
            resolve_all(&mut game, &mut Choices::default());
            assert!(
                game.object(source).is_none(),
                "the state-triggering event is not an intervening-if recheck"
            );
            assert!(game.object(entered).is_some());
        }
    }
}

#[test]
fn cabal_conditioning_keeps_zero_or_many_player_targets_and_the_casters_current_permanent_scope() {
    for definition in definitions("Cabal Conditioning") {
        for no_targets in [false, true] {
            let mut game = game();
            let greatest = compile_to_runtime_definition(
                "Caster's maximum",
                "Mana cost: {5}\nType: Artifact",
                false,
            )
            .unwrap();
            let maximum = game.create_object_from_definition(&greatest, A, Zone::Battlefield);
            let small = compile_to_runtime_definition(
                "Caster's remaining maximum",
                "Mana cost: {2}\nType: Artifact",
                false,
            )
            .unwrap();
            let source = game.create_object_from_definition(&small, A, Zone::Battlefield);
            let opposing = compile_to_runtime_definition(
                "Target player's larger permanent",
                "Mana cost: {9}\nType: Artifact",
                false,
            )
            .unwrap();
            game.create_object_from_definition(&opposing, B, Zone::Battlefield);
            for (player, count) in [(A, 3), (B, 3), (C, 1)] {
                for n in 0..count {
                    game.create_object_from_definition(
                        &vanilla(&format!("Hand {player:?} {n}"), "{1}", "Human", 1, 1),
                        player,
                        Zone::Hand,
                    );
                }
            }
            let mut dm = Choices {
                targets: if no_targets {
                    vec![]
                } else {
                    vec![Target::Player(B), Target::Player(C)]
                },
                decline_targets: no_targets,
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            assert!(
                dm.bounds
                    .iter()
                    .any(|(min, max)| *min == 0 && max.is_none())
            );
            apply(
                &mut game,
                source,
                Effect::destroy(ChooseSpec::SpecificObject(maximum)),
            );
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().hand.len(), 3);
            assert_eq!(
                game.player(B).unwrap().hand.len(),
                if no_targets { 3 } else { 1 }
            );
            assert_eq!(game.player(C).unwrap().hand.len(), usize::from(no_targets));
        }
    }
}

#[test]
fn gor_muldrak_freezes_all_tied_minimum_players_before_creating_tokens_and_preserves_protection() {
    for definition in definitions("Gor Muldrak, Amphinologist") {
        for mode in 0..3 {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut bob = None;
            if mode != 1 {
                bob = Some(creature(&mut game, B, "Bob's one creature", 1, 4));
                creature(&mut game, C, "Charlie's first creature", 1, 4);
                creature(&mut game, C, "Charlie's second creature", 1, 4);
            }
            game.turn.phase = Phase::Ending;
            game.turn.step = Some(ironsmith::game_state::Step::End);
            let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(A),
                Default::default(),
            );
            let mut queue = TriggerQueue::new();
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
            if mode == 2 {
                apply(
                    &mut game,
                    source,
                    Effect::new(ironsmith::effects::GainControlEffect::new(
                        ChooseSpec::SpecificObject(bob.unwrap()),
                        Until::EndOfTurn,
                    )),
                );
            }
            resolve_all(&mut game, &mut Choices::default());
            let salamanders = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.current_has_subtype(*id, ironsmith::Subtype::Salamander))
                .collect::<Vec<_>>();
            for player in [A, B, C] {
                let count = salamanders
                    .iter()
                    .filter(|id| game.current_controller(**id) == Some(player))
                    .count();
                let expected = match mode {
                    0 => usize::from(player == A || player == B),
                    1 => usize::from(player == B || player == C),
                    _ => usize::from(player == B),
                };
                assert_eq!(
                    count, expected,
                    "mode {mode}, player {player:?}: set membership is fixed before earlier recipients change the minimum"
                );
            }
            for id in &salamanders {
                assert_eq!(pt(&game, *id), (4, 3));
                assert!(game.current_has_subtype(*id, ironsmith::Subtype::Warrior));
            }
            let salamander = *salamanders
                .iter()
                .find(|id| game.current_controller(**id) == Some(B))
                .unwrap();
            apply(
                &mut game,
                salamander,
                Effect::deal_damage(5, ChooseSpec::SpecificPlayer(A)),
            );
            apply(
                &mut game,
                salamander,
                Effect::deal_damage(5, ChooseSpec::SpecificObject(source)),
            );
            assert_eq!(
                game.player(A).unwrap().life,
                20,
                "the player retains protection from Salamanders"
            );
            assert_eq!(
                game.damage_on(source),
                0,
                "the controller's permanents retain the same protection"
            );
            apply(
                &mut game,
                salamander,
                Effect::new(ironsmith::effects::GainControlEffect::new(
                    ChooseSpec::SpecificObject(source),
                    Until::EndOfTurn,
                )),
            );
            assert_eq!(game.current_controller(source), Some(B));
            apply(
                &mut game,
                salamander,
                Effect::deal_damage(5, ChooseSpec::SpecificPlayer(A)),
            );
            apply(
                &mut game,
                salamander,
                Effect::deal_damage(5, ChooseSpec::SpecificPlayer(B)),
            );
            apply(
                &mut game,
                salamander,
                Effect::deal_damage(5, ChooseSpec::SpecificObject(source)),
            );
            assert_eq!(
                game.player(A).unwrap().life,
                15,
                "protection follows the current source controller"
            );
            assert_eq!(game.player(B).unwrap().life, 20);
            assert_eq!(game.damage_on(source), 0);
        }
    }
}
