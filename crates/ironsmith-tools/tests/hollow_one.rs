use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};
fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Hollow One",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
fn probe(types: Vec<CardType>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Card probe")
        .card_types(types)
        .build()
}
fn cast(game: &mut GameState, player: PlayerId, spell: ObjectId) {
    let action = compute_legal_actions(game, player).into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).expect("spell cast must be legal");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(game.stack.len(), 1);
}
#[test]
fn hollow_one_discount_after_discard() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let hollow = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let discard_spell = ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Discard spell",
    )
    .card_types(vec![CardType::Sorcery])
    .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
        ironsmith::mana::ManaSymbol::Generic(0),
    ]]))
    .parse_text("Discard three cards.")
    .unwrap();
    let discard = game.create_object_from_definition(&discard_spell, alice, Zone::Hand);
    for _ in 0..3 {
        game.create_object_from_definition(&probe(vec![CardType::Land]), alice, Zone::Hand);
    }
    // Discard the three probes, keeping Hollow One out of the hand during the choice.
    let hollow = game.move_object_by_effect(hollow, Zone::Exile).unwrap();
    cast(&mut game, alice, discard);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        0,
        "all probes discarded"
    );
    ironsmith::game_loop::drain_pending_trigger_events(
        &mut game,
        &mut ironsmith::triggers::TriggerQueue::new(),
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .cards_discarded_by_player(alice),
        3
    );
    let hollow = game.move_object_by_effect(hollow, Zone::Hand).unwrap();
    cast(&mut game, alice, hollow);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
}
