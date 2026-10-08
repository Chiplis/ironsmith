//! Full frozen bodies; independent routes and runtime gates are authored, UNRUN.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::ManaPaymentContext;
use ironsmith::events::cause::EventCause;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ManaSymbol, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
const FOGS: [&str; 2] = ["Deep Wood", "Heavy Fog"];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/prevention_evidence_bodies.json.fixture")).unwrap();
    assert_eq!(rows.len(), 6);
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(row["source_candidate"], true);
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = result.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &materialized] {
        assert_eq!(definition.card.name, name);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialized]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.active_player = B;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 1);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
    game
}
fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let id = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Attacker witness")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 8)).build(),
        owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn declare(game: &mut GameState, declarations: &[(ObjectId, AttackTarget)]) {
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue,
        &declarations.iter().map(|(creature, target)| AttackerDeclaration {
            creature: *creature, target: target.clone(),
        }).collect::<Vec<_>>()).unwrap();
    game.combat = Some(combat);
    game.turn.priority_player = Some(A);
}
fn cast_action(game: &GameState, spell: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Normal, .. }
            if *spell_id == spell))
}
struct Choices;
impl DecisionMaker for Choices {
    fn decide_mana_payment(&mut self, _: &GameState, context: &ManaPaymentContext)
        -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: context.plan.id, request_hash: context.plan.request_hash,
        }
    }
}
fn cast(game: &mut GameState, spell: ObjectId) {
    let action = cast_action(game, spell).expect("full-cost spell is legal after direct attack");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut dm = Choices;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "printed {{1}}{{G}} was paid");
    // Native recovery must retain the pending spell's controller and full body.
    *game = game.clone();
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    assert!(game.stack.is_empty());
    assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
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

#[test]
fn complete_fogs_enforce_actual_declaration_history_and_exact_step() {
    for name in FOGS { for definition in definitions(name) {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let attacker = creature(&mut game, B);
        assert!(cast_action(&game, spell).is_none());
        declare(&mut game, &[(attacker, AttackTarget::Player(C))]);
        assert!(cast_action(&game, spell).is_none(), "another player was attacked");
    } }
    for name in FOGS { for definition in definitions(name) {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let attacker = creature(&mut game, B);
        declare(&mut game, &[(attacker, AttackTarget::Player(A))]);
        assert!(cast_action(&game, spell).is_some());
        // Removing the attacker doesn't undo the completed declaration.
        game.move_object_by_effect(attacker, Zone::Graveyard);
        game.combat.as_mut().unwrap().attackers.clear();
        game = game.clone();
        assert!(cast_action(&game, spell).is_some());
        game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        assert_eq!(game.combat.as_ref().unwrap().last_attack_declaration_step_players,
            Some([A].into_iter().collect()), "the last declaration record remains with combat");
        assert!(cast_action(&game, spell).is_none(), "a retained record never bypasses the current-step guard");
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        game.mark_combat_phase_started();
        assert!(cast_action(&game, spell).is_none(), "previous combat is not this step");
    } }
}

#[test]
fn attacking_a_planeswalker_is_not_attacking_its_controller() {
    for name in FOGS { for definition in definitions(name) {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let walker = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Walker witness")
            .card_types(vec![CardType::Planeswalker]).build(), A, Zone::Battlefield);
        game.add_counters(walker, ironsmith::CounterType::Loyalty, 5);
        let attacker = creature(&mut game, B);
        declare(&mut game, &[(attacker, AttackTarget::Planeswalker(walker))]);
        assert!(cast_action(&game, spell).is_none());
    } }
}

#[test]
fn paid_complete_fogs_filter_live_attackers_and_recipients_and_expire_at_cleanup() {
    for name in FOGS { for definition in definitions(name) {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let attacking_a = creature(&mut game, B);
        let attacking_c = creature(&mut game, B);
        let idle = creature(&mut game, B);
        let own = creature(&mut game, A);
        declare(&mut game, &[(attacking_a, AttackTarget::Player(A)), (attacking_c, AttackTarget::Player(C))]);
        cast(&mut game, spell);
        // The spell has left the stack, but its resolved turn-long shield persists.
        for combat in [true, false, true] {
            for source in [attacking_a, attacking_c] {
                assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, combat, false), (0, 3));
                assert_eq!(damage(&mut game, source, DamageTarget::Player(B), 3, combat, false), (3, 0));
                assert_eq!(damage(&mut game, source, DamageTarget::Object(own), 3, combat, false), (3, 0));
            }
            assert_eq!(damage(&mut game, idle, DamageTarget::Player(A), 3, combat, false), (3, 0));
        }
        assert_eq!(damage(&mut game, attacking_a, DamageTarget::Player(A), 3, true, true), (3, 0));
        game = game.clone();
        assert_eq!(damage(&mut game, attacking_a, DamageTarget::Player(A), 5, false, false), (0, 5));
        game.combat.as_mut().unwrap().attackers.retain(|entry| entry.creature != attacking_a);
        assert_eq!(damage(&mut game, attacking_a, DamageTarget::Player(A), 3, false, false), (3, 0));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(damage(&mut game, attacking_c, DamageTarget::Player(A), 3, false, false), (3, 0));
    } }
}

#[test]
fn attacking_a_protected_battle_does_not_open_the_player_attack_window() {
    for name in FOGS { for definition in definitions(name) {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let battle = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Battle witness")
            .card_types(vec![CardType::Battle]).subtypes(vec![ironsmith::Subtype::Siege]).build(),
            C, Zone::Battlefield);
        game.add_counters(battle, ironsmith::CounterType::Defense, 5);
        assert!(game.set_battle_protector(battle, A));
        let attacker = creature(&mut game, B);
        declare(&mut game, &[(attacker, AttackTarget::Battle(battle))]);
        assert!(cast_action(&game, spell).is_none());
    } }
}

fn fund_fog(game: &mut GameState) {
    game.turn.priority_player = Some(A);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 1);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
}

#[test]
fn complete_fogs_require_a_fresh_direct_attack_in_an_added_declaration_step() {
    use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
    use ironsmith::game_state::Step;
    for name in FOGS { for definition in definitions(name) {
        let mut game = game();
        game.turn.phase = ironsmith::Phase::Combat;
        game.mark_combat_phase_started();
        let phase = game.turn_store.combat_phases_started_this_turn;
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let attacker = creature(&mut game, B);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareAttackersDecision);
        let mut queue = TriggerQueue::new();
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Decision(_)));
        assert!(game.combat.as_ref().unwrap().last_attack_declaration_step_players.is_none());
        runner.respond_attackers(vec![AttackerDeclaration { creature: attacker, target: AttackTarget::Player(A) }]);
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::RunPriority));
        fund_fog(&mut game);
        assert!(cast_action(&game, spell).is_some());
        game.add_step_after(Step::DeclareAttackers, Step::DeclareAttackers);
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
        assert!(matches!(runner.state(), TurnState::DeclareAttackersDecision));
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Decision(_)));
        assert_eq!(game.turn_store.combat_phases_started_this_turn, phase);
        assert!(game.combat.as_ref().unwrap().last_attack_declaration_step_players.is_none());
        // The previous attacker remains attacking, but did not attack this step.
        assert!(game.combat.as_ref().unwrap().attackers.iter().any(|entry| entry.creature == attacker));
        fund_fog(&mut game);
        assert!(cast_action(&game, spell).is_none());
        runner.respond_attackers(vec![]);
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::RunPriority));
        fund_fog(&mut game);
        assert_eq!(game.combat.as_ref().unwrap().last_attack_declaration_step_players, Some(Default::default()));
        assert!(cast_action(&game, spell).is_none());
        // Melee still needs all direct attacks made in this combat phase.
        assert!(game.turn_store.turn_history.players_attacked_in_combat[&(phase, B)].contains(&A));
    } }
}

#[test]
fn legacy_added_declaration_step_clears_evidence_before_priority_and_can_commit_empty() {
    use ironsmith::game_state::Step;
    for name in FOGS { for definition in definitions(name) {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let attacker = creature(&mut game, B);
        declare(&mut game, &[(attacker, AttackTarget::Player(A))]);
        assert!(cast_action(&game, spell).is_some());
        let phase = game.turn_store.combat_phases_started_this_turn;
        game.add_step_after(Step::DeclareAttackers, Step::DeclareAttackers);
        ironsmith::turn::advance_step(&mut game).unwrap();
        fund_fog(&mut game);
        assert_eq!(game.turn.step, Some(Step::DeclareAttackers));
        assert_eq!(game.turn_store.combat_phases_started_this_turn, phase);
        assert!(game.combat.as_ref().unwrap().last_attack_declaration_step_players.is_none());
        assert!(cast_action(&game, spell).is_none());
        let mut combat = game.combat.clone().unwrap();
        apply_attacker_declarations(&mut game, &mut combat, &mut TriggerQueue::new(), &[]).unwrap();
        assert_eq!(combat.last_attack_declaration_step_players, Some(Default::default()));
        assert!(cast_action(&game, spell).is_none());
        assert!(game.turn_store.turn_history.players_attacked_in_combat[&(phase, B)].contains(&A));
    } }
}

#[test]
fn shared_turn_commits_union_direct_player_defenders_without_importing_existing_attackers() {
    let d = PlayerId::from_index(3);
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    game.set_teams(vec![vec![A, B], vec![C, d]]).unwrap();
    game.enable_shared_team_turns().unwrap();
    game.turn.active_player = A;
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let first = creature(&mut game, A);
    let second = creature(&mut game, B);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(&mut game, &mut combat, &mut queue, &[
        AttackerDeclaration { creature: first, target: AttackTarget::Player(C) },
        AttackerDeclaration { creature: second, target: AttackTarget::Player(d) },
    ]).unwrap();
    assert_eq!(combat.last_attack_declaration_step_players, Some([C, d].into_iter().collect()));
    // A later same-step transaction does not erase previously committed players.
    apply_attacker_declarations(&mut game, &mut combat, &mut queue, &[]).unwrap();
    assert_eq!(combat.last_attack_declaration_step_players, Some([C, d].into_iter().collect()));
    combat.last_attack_declaration_step_players = None;
    game.combat = Some(combat.clone());
    apply_attacker_declarations(&mut game, &mut combat, &mut queue, &[]).unwrap();
    assert_eq!(combat.last_attack_declaration_step_players, Some(Default::default()));
    assert_eq!(combat.attackers.len(), 2);
}

#[test]
fn native_grand_melee_lanes_keep_step_evidence_independent() {
    let mut game = GameState::new((0..8).map(|index| format!("Player {index}")).collect(), 20);
    let seats = (0..8).map(PlayerId::from_index).collect();
    game.restore_grand_melee(seats).unwrap();
    game.combat = Some(CombatState { last_attack_declaration_step_players: Some([A].into_iter().collect()),
        ..Default::default() });
    game.select_grand_melee_turn_marker(2).unwrap();
    assert!(game.combat.as_ref().is_none_or(|combat| combat.last_attack_declaration_step_players.is_none()));
    game.combat = Some(CombatState { last_attack_declaration_step_players: Some(Default::default()),
        ..Default::default() });
    game.select_grand_melee_turn_marker(1).unwrap();
    assert_eq!(game.combat.as_ref().unwrap().last_attack_declaration_step_players, Some([A].into_iter().collect()));
    game = game.clone();
    game.select_grand_melee_turn_marker(2).unwrap();
    assert_eq!(game.combat.as_ref().unwrap().last_attack_declaration_step_players, Some(Default::default()));
    let restored = game.grand_melee_restore_snapshot().unwrap();
    assert_eq!(restored.markers[0].combat.as_ref().unwrap().last_attack_declaration_step_players,
        Some([A].into_iter().collect()));
}

#[test]
fn invalid_declaration_never_publishes_committed_step_evidence() {
    let mut game = game();
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let attacker = creature(&mut game, B);
    let mut combat = CombatState::default();
    game.combat = Some(combat.clone());
    let mut queue = TriggerQueue::new();
    let same = AttackerDeclaration { creature: attacker, target: AttackTarget::Player(A) };
    assert!(apply_attacker_declarations(&mut game, &mut combat, &mut queue, &[same.clone(), same]).is_err());
    assert!(combat.last_attack_declaration_step_players.is_none());
    assert!(game.combat.as_ref().unwrap().last_attack_declaration_step_players.is_none());
    assert!(!game.is_tapped(attacker));
}
