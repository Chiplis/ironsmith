//! UNVALIDATED: source-only exact-incarnation hand-cost regressions.
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effect::Effect;
use ironsmith::effects::{DrawCardsEffect, EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
// Frozen oracle_id 737db899-dcdb-48f9-8d30-cbbce3ae3434.
const TEXT: &str = "Mana cost: {6}\nType: Artifact\n{2}, {T}, Discard the last card you drew this turn: Draw a card.";
fn definitions() -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition("Jandor's Ring", TEXT, false).unwrap();
    let (artifact, _) = compile_to_artifact("Jandor's Ring", TEXT, false).unwrap();
    let transported = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, transported);
    [direct, materialize_artifact(&transported).unwrap()]
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build(),
        owner,
        zone,
    )
}
fn setup(definition: &CardDefinition) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 8);
    (game, source)
}
fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: i32) {
    let mut ctx = EffectContext::new_default(source, player);
    execute_effect(
        game,
        &Effect::new(DrawCardsEffect::new(count, PlayerFilter::You)),
        &mut ctx,
    )
    .unwrap();
}
fn legal(game: &GameState, source: ObjectId) -> bool {
    compute_legal_actions(game, A).unwrap().iter().any(
        |action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source),
    )
}
fn activate(game: &mut GameState, source: ObjectId) {
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..12 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished payment: {progress:?}");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
}
#[test]
fn exact_ring_pays_only_latest_actual_draw_and_a_replaced_draw_does_not_invent_a_new_one() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition);
        card(&mut game, A, Zone::Library, "First library card");
        card(&mut game, A, Zone::Library, "Second library card");
        draw(&mut game, source, A, 2);
        let latest = game
            .turn_store
            .turn_history
            .last_card_drawn_by_player(A)
            .unwrap();
        let latest_stable = game.object(latest).unwrap().stable_id;
        let older = game
            .player(A)
            .unwrap()
            .hand
            .iter()
            .copied()
            .find(|id| *id != latest)
            .unwrap();
        card(&mut game, B, Zone::Library, "Bob's later draw");
        draw(&mut game, source, B, 1);
        let replacement = compile_to_runtime_definition("Empty draw control", "Type: Enchantment\nIf you would draw a card while your library has no cards in it, instead you gain 1 life.", false).unwrap();
        game.create_object_from_definition(&replacement, A, Zone::Battlefield);
        let life = game.player(A).unwrap().life;
        draw(&mut game, source, A, 1);
        assert_eq!(game.player(A).unwrap().life, life + 1);
        assert_eq!(
            game.turn_store.turn_history.last_card_drawn_by_player(A),
            Some(latest)
        );
        assert!(legal(&game, source));
        activate(&mut game, source);
        let discarded = game.find_object_by_stable_id(latest_stable).unwrap();
        assert_eq!(game.object(discarded).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.player(A).unwrap().hand, vec![older]);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 6);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(
            game.turn_store.turn_history.last_card_drawn_by_player(A),
            Some(latest)
        );
        game.untap(source);
        card(&mut game, A, Zone::Hand, "Not drawn");
        assert!(
            !legal(&game, source),
            "an older draw or later non-draw arrival cannot replace the departed latest card"
        );
        card(&mut game, A, Zone::Library, "New actual draw");
        draw(&mut game, source, A, 1);
        assert!(legal(&game, source));
    }
}
#[test]
fn latest_draw_identity_never_follows_a_leave_return_incarnation_or_survives_turn_cleanup() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition);
        card(&mut game, A, Zone::Library, "Older draw");
        card(&mut game, A, Zone::Library, "Latest draw");
        draw(&mut game, source, A, 2);
        let latest = game
            .turn_store
            .turn_history
            .last_card_drawn_by_player(A)
            .unwrap();
        let departed = game.move_object_by_effect(latest, Zone::Graveyard).unwrap();
        let returned = game.move_object_by_effect(departed, Zone::Hand).unwrap();
        assert_ne!(returned, latest);
        assert_eq!(
            game.turn_store.turn_history.last_card_drawn_by_player(A),
            Some(latest)
        );
        assert!(!legal(&game, source));
        card(&mut game, A, Zone::Library, "Later successful draw");
        draw(&mut game, source, A, 1);
        assert!(legal(&game, source));
        game.next_turn();
        game.turn.priority_player = Some(A);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        assert!(
            game.turn_store
                .turn_history
                .last_card_drawn_by_player(A)
                .is_none()
        );
        assert!(!legal(&game, source));
    }
}
#[test]
fn old_object_filter_payloads_default_the_latest_draw_constraint_off() {
    let mut json = serde_json::to_value(ironsmith::target::ObjectFilter::default()).unwrap();
    json.as_object_mut().unwrap().remove("last_drawn_this_turn");
    let filter: ironsmith::target::ObjectFilter = serde_json::from_value(json).unwrap();
    assert!(filter.last_drawn_this_turn.is_none());
}

#[test]
fn latest_draw_player_scope_participates_in_generic_iterated_player_validation() {
    let filter = ironsmith::target::ObjectFilter {
        last_drawn_this_turn: Some(PlayerFilter::IteratedPlayer),
        ..Default::default()
    };
    assert!(filter.mentions_iterated_player());
}
