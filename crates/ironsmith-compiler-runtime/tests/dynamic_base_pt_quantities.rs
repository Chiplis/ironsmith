//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
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
        "../../../fixtures/dynamic_base_pt_quantities.json.fixture"
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
fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    let prior = game.stack.len();
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
    assert_eq!(game.stack.len(), prior + 1);
}
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
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
fn queue_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) -> usize {
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
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
fn attack(game: &mut GameState, attacker: ObjectId, defender: PlayerId, dm: &mut Choices) {
    game.remove_summoning_sickness(attacker);
    game.turn.active_player = game.current_controller(attacker).unwrap();
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::AttackerDeclaration {
            creature: attacker,
            target: ironsmith::combat_state::AttackTarget::Player(defender),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn block(game: &mut GameState, blocker: ObjectId, attacker: ObjectId, dm: &mut Choices) {
    let defender = game.current_controller(blocker).unwrap();
    game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
    let mut combat = game.combat.take().unwrap();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_blocker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::BlockerDeclaration {
            blocker,
            blocking: attacker,
        }],
        defender,
    )
    .unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}

#[test]
fn nine_exact_base_characteristic_cards_round_trip_as_real_continuous_effects() {
    assert_eq!(fixtures().len(), 9);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{definition:?}");
            assert!(
                debug.contains("resolve_set_pt_values_at_resolution: true"),
                "{debug}"
            );
        }
    }
}

#[test]
fn entrant_reference_reads_current_values_or_exact_departure_lki_and_then_freezes() {
    for name in ["Belligerent Yearling", "Eldrazi Mimic"] {
        for definition in definitions(name) {
            for blink in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                counter(&mut game, source, source, 1);
                let hand = game.create_object_from_definition(
                    &vanilla("Entering object", "{4}", "Dinosaur", 4, 6),
                    A,
                    Zone::Hand,
                );
                let stable = game.object(hand).unwrap().stable_id;
                let mut dm = Choices::default();
                let outcome = apply(
                    &mut game,
                    source,
                    Effect::put_onto_battlefield(
                        ChooseSpec::SpecificObject(hand),
                        false,
                        PlayerFilter::You,
                    ),
                );
                queue_outcome(&mut game, outcome, &mut dm);
                assert_eq!(game.stack.len(), 1);
                let entered = game.find_object_by_stable_id(stable).unwrap();
                pump(&mut game, source, entered, 3, 2);
                if blink {
                    let grave = game
                        .move_object_by_game_rule(entered, Zone::Graveyard)
                        .unwrap();
                    let returned = game
                        .move_object_by_game_rule(grave, Zone::Battlefield)
                        .unwrap();
                    assert_ne!(entered, returned);
                    pump(&mut game, source, returned, 10, 10);
                }
                resolve_all(&mut game, &mut dm);
                assert_eq!(
                    pt(&game, source),
                    if name == "Belligerent Yearling" {
                        (8, 3)
                    } else {
                        (8, 9)
                    }
                );
                if !blink {
                    pump(&mut game, source, entered, 8, 8);
                }
                assert_eq!(
                    pt(&game, source),
                    if name == "Belligerent Yearling" {
                        (8, 3)
                    } else {
                        (8, 9)
                    }
                );
                ironsmith::turn::execute_cleanup_step(&mut game);
                assert_eq!(
                    pt(&game, source),
                    if name == "Belligerent Yearling" {
                        (4, 3)
                    } else {
                        (3, 2)
                    }
                );
            }
        }
    }
}

#[test]
fn optional_entrant_assignment_can_be_declined_without_altering_either_axis() {
    for definition in definitions("Eldrazi Mimic") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = game.create_object_from_definition(
            &vanilla("Colorless entrant", "{4}", "Eldrazi", 7, 9),
            A,
            Zone::Hand,
        );
        let mut dm = Choices {
            decline: true,
            ..Default::default()
        };
        let outcome = apply(
            &mut game,
            source,
            Effect::put_onto_battlefield(
                ChooseSpec::SpecificObject(card),
                false,
                PlayerFilter::You,
            ),
        );
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, source), (2, 1));
    }
}

#[test]
fn wolfbear_copies_resolving_source_stats_to_its_announced_human_only() {
    for definition in definitions("Exuberant Wolfbear") {
        for leave_source in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let human = game.create_object_from_definition(
                &vanilla("Human recipient", "{1}", "Human", 1, 2),
                A,
                Zone::Battlefield,
            );
            let other = game.create_object_from_definition(
                &vanilla("Other Human", "{1}", "Human", 1, 2),
                A,
                Zone::Battlefield,
            );
            counter(&mut game, source, human, 1);
            let mut dm = Choices {
                targets: vec![Target::Object(human)],
                ..Default::default()
            };
            attack(&mut game, source, B, &mut dm);
            pump(&mut game, source, source, 2, 3);
            if leave_source {
                let grave = game
                    .move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap();
                let returned = game
                    .move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
                pump(&mut game, returned, returned, 20, 20);
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(pt(&game, human), (7, 8));
            assert_eq!(pt(&game, other), (1, 2));
            if !leave_source {
                pump(&mut game, source, source, 8, 8);
            }
            assert_eq!(pt(&game, human), (7, 8));
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(pt(&game, human), (2, 3));
        }
    }
}

#[test]
fn riptide_announces_a_separate_numeric_target_and_an_illegal_target_cannot_supply_lki() {
    for definition in definitions("Riptide Mangler") {
        for blink in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            counter(&mut game, source, source, 1);
            let reference = game.create_object_from_definition(
                &vanilla("Numeric target", "{5}", "Human", 8, 10),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(reference)],
                ..Default::default()
            };
            activate(&mut game, source, activated(&definition), &mut dm);
            pump(&mut game, source, reference, 3, 0);
            if blink {
                let grave = game
                    .move_object_by_game_rule(reference, Zone::Graveyard)
                    .unwrap();
                game.move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
            }
            resolve_all(&mut game, &mut dm);
            assert_eq!(pt(&game, source), if blink { (1, 4) } else { (12, 4) });
            if !blink {
                pump(&mut game, source, reference, 5, 0);
            }
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(pt(&game, source), if blink { (1, 4) } else { (12, 4) });
        }
    }
}

#[test]
fn sentinel_numeric_target_must_be_a_combat_partner_and_sets_only_toughness() {
    for definition in definitions("Sentinel") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let partner = game.create_object_from_definition(
            &vanilla("Blocked attacker", "{4}", "Human", 5, 8),
            B,
            Zone::Battlefield,
        );
        let unrelated = game.create_object_from_definition(
            &vanilla("Unrelated creature", "{4}", "Human", 20, 20),
            B,
            Zone::Battlefield,
        );
        let mut dm = Choices::default();
        attack(&mut game, partner, A, &mut dm);
        block(&mut game, source, partner, &mut dm);
        game.turn.priority_player = Some(A);
        dm.targets = vec![Target::Object(partner)];
        activate(&mut game, source, activated(&definition), &mut dm);
        pump(&mut game, source, partner, 3, 0);
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, source), (1, 9));
        game.move_object_by_game_rule(partner, Zone::Graveyard)
            .unwrap();
        assert_eq!(pt(&game, source), (1, 9));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(pt(&game, source), (1, 9));
        assert_eq!(pt(&game, unrelated), (20, 20));
        // With no current combat partner, its announced numeric target is
        // unavailable. The ability is not an unrestricted source-only action.
        game.combat = None;
        assert!(!compute_legal_actions(&game, A).unwrap().contains(
            &LegalAction::ActivateAbility {
                source,
                ability_index: activated(&definition)
            }
        ));
    }
}

#[test]
fn sita_pays_exhaust_x_then_freezes_source_power_for_the_current_other_creature_set() {
    for definition in definitions("Sita Varma, Masked Racer") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let recipient = game.create_object_from_definition(
            &vanilla("Recipient with counter", "{1}", "Human", 1, 2),
            A,
            Zone::Battlefield,
        );
        counter(&mut game, source, recipient, 1);
        let opponent = game.create_object_from_definition(
            &vanilla("Opponent's creature", "{1}", "Human", 8, 9),
            B,
            Zone::Battlefield,
        );
        let index = activated(&definition);
        let mut dm = Choices {
            x: 2,
            ..Default::default()
        };
        activate(&mut game, source, index, &mut dm);
        game.set_current_controller(source, C).unwrap();
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, source), (4, 5));
        assert_eq!(pt(&game, recipient), (5, 5));
        assert_eq!(pt(&game, opponent), (8, 9));
        pump(&mut game, source, source, 7, 0);
        game.set_current_controller(recipient, B).unwrap();
        let later = game.create_object_from_definition(
            &vanilla("Late creature", "{1}", "Human", 1, 2),
            A,
            Zone::Battlefield,
        );
        assert_eq!(pt(&game, recipient), (5, 5));
        assert_eq!(pt(&game, later), (1, 2));
        game.set_current_controller(source, A).unwrap();
        game.turn.priority_player = Some(A);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(
            &LegalAction::ActivateAbility {
                source,
                ability_index: index
            }
        ));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(pt(&game, source), (4, 5));
        assert_eq!(pt(&game, recipient), (2, 3));
    }
}

#[test]
fn wall_counts_the_trigger_controllers_graveyard_once_and_next_upkeep_replaces_the_value() {
    for definition in definitions("Wall of Tombstones") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..2 {
            game.create_object_from_definition(
                &vanilla("Buried creature", "{2}", "Human", 2, 2),
                A,
                Zone::Graveyard,
            );
        }
        game.create_object_from_definition(
            &resource("Not a creature", "Artifact"),
            A,
            Zone::Graveyard,
        );
        game.create_object_from_definition(
            &vanilla("Other graveyard", "{2}", "Human", 2, 2),
            C,
            Zone::Graveyard,
        );
        let response = game.create_object_from_definition(
            &vanilla("Response creature", "{2}", "Human", 2, 2),
            A,
            Zone::Battlefield,
        );
        let mut dm = Choices::default();
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::phase::BeginningOfUpkeepEvent::new(A),
            Default::default(),
        );
        assert_eq!(queue_event(&mut game, event, &mut dm), 1);
        game.set_current_controller(source, C).unwrap();
        game.move_object_by_game_rule(response, Zone::Graveyard)
            .unwrap();
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, source), (0, 4));
        let graveyard = game.player(A).unwrap().graveyard.clone();
        for id in graveyard {
            game.move_object_by_game_rule(id, Zone::Exile).unwrap();
        }
        assert_eq!(pt(&game, source), (0, 4));
        let event = TriggerEvent::new_with_provenance(
            ironsmith::events::phase::BeginningOfUpkeepEvent::new(C),
            Default::default(),
        );
        assert_eq!(queue_event(&mut game, event, &mut dm), 1);
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, source), (0, 2));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(pt(&game, source), (0, 2));
    }
}

#[test]
fn pupu_puts_a_land_then_sets_power_from_live_towns_without_touching_toughness() {
    for definition in definitions("PuPu UFO") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let town = resource("Town", "Land — Town");
        game.create_object_from_definition(&town, A, Zone::Battlefield);
        let hand = game.create_object_from_definition(&town, A, Zone::Hand);
        game.create_object_from_definition(&town, B, Zone::Battlefield);
        let indices: Vec<_> = definition
            .abilities
            .iter()
            .enumerate()
            .filter_map(|(index, ability)| {
                matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
                    .then_some(index)
            })
            .collect();
        let mut dm = Choices {
            objects: vec![hand],
            ..Default::default()
        };
        activate(&mut game, source, indices[0], &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 0);
        dm.objects.clear();
        activate(&mut game, source, indices[1], &mut dm);
        let response = game.create_object_from_definition(&town, A, Zone::Hand);
        let stable = game.object(response).unwrap().stable_id;
        let outcome = apply(
            &mut game,
            source,
            Effect::put_onto_battlefield(
                ChooseSpec::SpecificObject(response),
                false,
                PlayerFilter::You,
            ),
        );
        queue_outcome(&mut game, outcome, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(pt(&game, source), (3, 4));
        let entered = game.find_object_by_stable_id(stable).unwrap();
        game.move_object_by_game_rule(entered, Zone::Graveyard)
            .unwrap();
        assert_eq!(pt(&game, source), (3, 4));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(pt(&game, source), (0, 4));
    }
}

#[test]
fn shape_stealer_uses_the_other_object_for_both_combat_directions_and_never_follows_a_blink() {
    for definition in definitions("Shape Stealer") {
        for source_attacks in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            counter(&mut game, source, source, 1);
            let partner = game.create_object_from_definition(
                &vanilla("Combat partner", "{4}", "Human", 5, 8),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices::default();
            if source_attacks {
                attack(&mut game, source, B, &mut dm);
                block(&mut game, partner, source, &mut dm);
            } else {
                attack(&mut game, partner, A, &mut dm);
                block(&mut game, source, partner, &mut dm);
            }
            assert_eq!(game.stack.len(), 1);
            pump(&mut game, source, partner, 2, 3);
            let grave = game
                .move_object_by_game_rule(partner, Zone::Graveyard)
                .unwrap();
            let returned = game
                .move_object_by_game_rule(grave, Zone::Battlefield)
                .unwrap();
            pump(&mut game, source, returned, 20, 20);
            resolve_all(&mut game, &mut dm);
            assert_eq!(pt(&game, source), (8, 12));
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(pt(&game, source), (2, 2));
        }
    }
}

#[test]
fn single_axis_assignment_preserves_negative_and_zero_reference_power() {
    for definition in definitions("Riptide Mangler") {
        for power in [-4, 0, 7] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let reference = game.create_object_from_definition(
                &vanilla("Signed reference", "{2}", "Human", power, 5),
                B,
                Zone::Battlefield,
            );
            let mut dm = Choices {
                targets: vec![Target::Object(reference)],
                ..Default::default()
            };
            activate(&mut game, source, activated(&definition), &mut dm);
            resolve_all(&mut game, &mut dm);
            assert_eq!(pt(&game, source), (power, 3));
        }
    }
}
