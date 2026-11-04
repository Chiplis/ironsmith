//! Actual spell-payment history predicate used by combat animation.
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
const A: PlayerId = PlayerId(0);
fn compile(text: &str) -> CardDefinition {
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_runtime_definition("Payment history", text, false)).unwrap()
}
fn announce(game: &mut GameState, action: LegalAction) {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
}

#[test]
fn combat_animation_counts_actual_spell_payments_and_preserves_types_until_cleanup() {
    let rules = "Type: Artifact\nPower/Toughness: 3/4\nAt the beginning of combat on your turn, if you spent at least 4 mana to cast spells this turn, you may have this permanent become a creature in addition to its other types until end of turn.";
    let direct = compile(rules);
    let (artifact, _) = ironsmith_compiler_runtime::compile_to_artifact("Artifact payment history", rules, false).unwrap();
    let artifact = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap()] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A; game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let combat_count = |game: &GameState| ironsmith::triggers::check_triggers(game, &ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfCombatEvent::new(A), Default::default())).len();
        assert_eq!(combat_count(&game), 0);
        for price in [0, 2, 2] {
            let spell = compile(&format!("Mana cost: {{{price}}}\nType: Sorcery\nYou gain 1 life."));
            let id = game.create_object_from_definition(&spell, A, Zone::Hand);
            game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, price);
            let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell {spell_id, ..} if *spell_id == id)).unwrap();
            announce(&mut game, action);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            let spent = game.turn_store.turn_history.mana_spent_to_cast_spells_this_turn.get(&A).copied().unwrap_or(0);
            assert_eq!(combat_count(&game), usize::from(spent >= 4));
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfCombatEvent::new(A), Default::default()));
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.current_characteristics(source).unwrap().card_types.contains(&ironsmith::types::CardType::Artifact));
        assert!(game.current_characteristics(source).unwrap().card_types.contains(&ironsmith::types::CardType::Creature));
        assert_eq!(game.calculated_power(source), Some(3));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(!game.current_characteristics(source).unwrap().card_types.contains(&ironsmith::types::CardType::Creature));
    }
}
