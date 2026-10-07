//! Full frozen Oketra body on independent strict routes. All scenarios UNRUN.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::game_loop::{apply_attacker_declarations_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/exert_player_tenure.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &materialized] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialized]
}

fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget, amount: u32,
    combat: bool, unpreventable: bool) -> (u32, u32) {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(game, source,
        target, amount, combat, unpreventable, EventCause::effect(), None).unwrap();
    let remaining = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented = game.take_pending_trigger_events().iter().filter_map(|event|
        event.downcast::<DamagePreventedEvent>().map(|event| event.amount)).sum();
    (remaining, prevented)
}


#[derive(Default)]
struct Choice { exert: bool, suspend: bool, pending: bool, calls: usize }
impl DecisionMaker for Choice {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.calls += 1;
        self.pending = self.suspend;
        self.exert
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn setup(definition: &CardDefinition) -> (GameState, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 30);
    let avenger = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.remove_summoning_sickness(avenger);
    let enemy = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Witness")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 9))
        .build(), B, Zone::Battlefield);
    (game, avenger, enemy)
}
fn attack(game: &mut GameState, avenger: ObjectId, choice: &mut Choice) -> TriggerQueue {
    game.turn.active_player = A;
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations_with_dm(game, &mut combat, &mut queue,
        &[AttackerDeclaration { creature: avenger, target: AttackTarget::Player(B) }], choice).unwrap();
    if !choice.pending { game.combat = Some(combat); }
    queue
}
fn resolve(game: &mut GameState, queue: &mut TriggerQueue, choice: &mut Choice) {
    put_triggers_on_stack_with_dm(game, queue, choice).unwrap();
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry_with(game, choice).unwrap();
    assert!(game.stack.is_empty());
}
fn untap(game: &mut GameState, player: PlayerId) {
    game.turn.turn_number += 1;
    game.turn.active_player = player;
    game.turn.phase = ironsmith::Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Untap);
    ironsmith::turn::execute_untap_step_with(game, &mut SelectFirstDecisionMaker).unwrap();
}
#[test]
fn complete_body_choice_reflexive_recipient_damage_domain_and_cleanup() {
    for definition in definitions("Oketra's Avenger") {
        for exert in [false, true] {
            let (mut game, avenger, enemy) = setup(&definition);
            let mut choice = Choice { exert, ..Default::default() };
            let mut queue = attack(&mut game, avenger, &mut choice);
            assert_eq!(choice.calls, 1);
            assert!(game.is_tapped(avenger));
            // Cost alone does not prevent damage before its linked trigger resolves.
            assert_eq!(damage(&mut game, enemy, DamageTarget::Object(avenger), 2, true, false), (2, 0));
            if exert { resolve(&mut game, &mut queue, &mut choice); }
            else {
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choice).unwrap();
                assert!(game.stack.is_empty());
                assert!(game.effect_store.restriction_effects.is_empty());
            }
            for mut branch in [game.clone(), game] {
                for amount in [1, 4, 7] {
                    assert_eq!(damage(&mut branch, enemy, DamageTarget::Object(avenger), amount, true, false),
                        if exert { (0, amount) } else { (amount, 0) });
                }
                assert_eq!(damage(&mut branch, enemy, DamageTarget::Object(avenger), 3, false, false), (3, 0));
                assert_eq!(damage(&mut branch, enemy, DamageTarget::Object(avenger), 3, true, true), (3, 0));
                assert_eq!(damage(&mut branch, avenger, DamageTarget::Object(enemy), 3, true, false), (3, 0));
                assert_eq!(damage(&mut branch, enemy, DamageTarget::Player(A), 3, true, false), (3, 0));
                ironsmith::turn::execute_cleanup_step(&mut branch);
                assert_eq!(damage(&mut branch, enemy, DamageTarget::Object(avenger), 3, true, false), (3, 0));
                untap(&mut branch, B);
                untap(&mut branch, A);
                assert_eq!(branch.is_tapped(avenger), exert);
                untap(&mut branch, A);
                assert!(!branch.is_tapped(avenger));
            }
        }
    }
}
#[test]
fn control_change_preserves_exerting_player_and_consumes_without_current_control() {
    for definition in definitions("Oketra's Avenger") {
        let (mut game, avenger, enemy) = setup(&definition);
        let mut choice = Choice { exert: true, ..Default::default() };
        let mut queue = attack(&mut game, avenger, &mut choice);
        game.set_current_controller(avenger, B).unwrap();
        resolve(&mut game, &mut queue, &mut choice);
        assert_eq!(damage(&mut game, enemy, DamageTarget::Object(avenger), 5, true, false), (0, 5));
        let receipt = &game.effect_store.restriction_effects[0];
        assert_eq!(receipt.untap_step_player(&game), Some(A));
        assert!(receipt.untap_step_object.is_none());
        untap(&mut game, C);
        assert_eq!(game.effect_store.restriction_effects.len(), 1);
        untap(&mut game, B);
        assert!(!game.is_tapped(avenger), "B's step is not the exerting player's step");
        game.tap(avenger);
        untap(&mut game, A);
        assert!(game.is_tapped(avenger), "B controls it, so it is not normally untapped by A");
        assert!(game.effect_store.restriction_effects.is_empty(), "A's step consumes even without control");
        game.set_current_controller(avenger, A).unwrap();
        untap(&mut game, A);
        assert!(!game.is_tapped(avenger));
    }
}
#[test]
fn repeated_combat_exertions_expire_together_and_suspended_choice_rolls_back() {
    for definition in definitions("Oketra's Avenger") {
        let (mut game, avenger, _) = setup(&definition);
        let mut pending = Choice { exert: true, suspend: true, ..Default::default() };
        let mut queue = attack(&mut game, avenger, &mut pending);
        assert!(pending.pending);
        assert!(!game.is_tapped(avenger));
        assert!(game.effect_store.restriction_effects.is_empty());
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.is_empty());
        for _ in 0..2 {
            game.untap(avenger); // Effect-driven untap between combats is permitted.
            game.combat = None;
            let mut choice = Choice { exert: true, ..Default::default() };
            let mut queue = attack(&mut game, avenger, &mut choice);
            resolve(&mut game, &mut queue, &mut choice);
        }
        assert_eq!(game.effect_store.restriction_effects.len(), 2);
        untap(&mut game, A);
        assert!(game.is_tapped(avenger));
        assert!(game.effect_store.restriction_effects.is_empty());
        untap(&mut game, A);
        assert!(!game.is_tapped(avenger));
    }
}
#[test]
fn source_departure_before_or_after_resolution_cannot_protect_a_new_incarnation() {
    for definition in definitions("Oketra's Avenger") {
        for before_resolution in [false, true] {
            let (mut game, avenger, enemy) = setup(&definition);
            let mut choice = Choice { exert: true, ..Default::default() };
            let mut queue = attack(&mut game, avenger, &mut choice);
            if !before_resolution { resolve(&mut game, &mut queue, &mut choice); }
            let hand = game.move_object_by_effect(avenger, Zone::Hand).unwrap();
            let fresh = game.move_object_by_effect(hand, Zone::Battlefield).unwrap();
            assert_ne!(fresh, avenger);
            if before_resolution { resolve(&mut game, &mut queue, &mut choice); }
            assert_eq!(damage(&mut game, enemy, DamageTarget::Object(fresh), 3, true, false), (3, 0));
            game.tap(fresh);
            untap(&mut game, A);
            assert!(!game.is_tapped(fresh));
        }
    }
}

/// Supplemental native-owner boundary gate; does not substitute for any full-body route above.
#[test]
fn native_exert_registered_during_untap_waits_for_next_occurrence_even_same_turn() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, EffectExecutor, ExertCostEffect, ScheduleDelayedTriggerEffect};
    use ironsmith::target::PlayerFilter;
    use ironsmith::triggers::Trigger;
    use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
    use ironsmith::game_state::Step;
    for definition in definitions("Oketra's Avenger") {
        let (mut game, avenger, _) = setup(&definition);
        game.tap(avenger);
        ScheduleDelayedTriggerEffect::new(
            Trigger::as_permanents_untap(PlayerFilter::Specific(A), true),
            vec![Effect::new(ExertCostEffect::new("Exert this permanent"))], true,
            Vec::new(), PlayerFilter::Specific(A))
            .execute(&mut game, &mut EffectContext::new_default(avenger, A)).unwrap();
        game.turn.active_player = A;
        game.turn.phase = ironsmith::Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        game.add_step_after(Step::Untap, Step::Untap);
        let mut runner = TurnRunner::new();
        let mut queue = TriggerQueue::new();
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::UntapEndMana));
        assert!(!game.is_tapped(avenger));
        let boundary = game.turn_store.untap_step_started_at.unwrap();
        assert_eq!(game.effect_store.restriction_effects.len(), 1);
        let receipt = &game.effect_store.restriction_effects[0];
        assert!(receipt.timestamp > boundary.1);
        assert!(!receipt.is_active(&game, game.turn.turn_number));
        game.tap(avenger);
        for mut branch in [game.clone(), game] {
            // Native clones retain both the exerting player and registration cutoff.
            let turn = branch.turn.turn_number;
            ironsmith::turn::execute_untap_step_with(&mut branch, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(branch.turn.turn_number, turn);
            assert!(branch.turn_store.untap_step_started_at.unwrap().1 > boundary.1);
            assert!(branch.is_tapped(avenger));
            assert!(branch.effect_store.restriction_effects.is_empty());
            ironsmith::turn::execute_untap_step_with(&mut branch, &mut SelectFirstDecisionMaker).unwrap();
            assert!(!branch.is_tapped(avenger));
        }
    }
}

#[test]
fn native_exert_uses_the_executing_player_and_not_another_active_grand_melee_lane() {
    use ironsmith::effects::{EffectContext, EffectExecutor, ExertCostEffect};
    for definition in definitions("Oketra's Avenger") {
        let mut game = GameState::new((0..8).map(|i| format!("Player {i}")).collect(), 30);
        let seats = (0..8).map(PlayerId::from_index).collect();
        game.restore_grand_melee_with_starting_player(seats, A).unwrap();
        let other = game.grand_melee_marker_views().into_iter().find(|marker| marker.holder != A).unwrap();
        let avenger = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.tap(avenger);
        ExertCostEffect::new("Exert this creature")
            .execute(&mut game, &mut EffectContext::new_default(avenger, A)).unwrap();
        game.set_current_controller(avenger, other.holder).unwrap();
        game.select_grand_melee_turn_marker(other.number).unwrap();
        assert!(game.is_active_player(A), "A remains active in its independent lane");
        assert!(!game.turn_players().contains(&A));
        game.turn.phase = ironsmith::Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Untap);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!game.is_tapped(avenger));
        assert_eq!(game.effect_store.restriction_effects.len(), 1);
        assert_eq!(game.effect_store.restriction_effects[0].untap_step_player(&game), Some(A));
        let a_marker = game.grand_melee_marker_views().into_iter().find(|marker| marker.holder == A).unwrap();
        game.select_grand_melee_turn_marker(a_marker.number).unwrap();
        game.turn.phase = ironsmith::Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        assert!(!game.effect_store.restriction_effects[0].is_active(&game, game.turn.turn_number));
        game.turn.step = Some(ironsmith::game_state::Step::Untap);
        ironsmith::turn::execute_untap_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.effect_store.restriction_effects.is_empty());
    }
}
