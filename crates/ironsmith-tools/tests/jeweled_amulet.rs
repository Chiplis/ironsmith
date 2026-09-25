//! Jeweled Amulet: "{1}, {T}: Put a charge counter on this artifact. Note the
//! type of mana spent to pay this activation cost. Activate only if there are
//! no charge counters on this artifact. {T}, Remove a charge counter from this
//! artifact: Add one mana of this artifact's last noted type."
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::mana::ManaSymbol;
use ironsmith::{CounterType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Jeweled Amulet",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

fn setup() -> (GameState, ObjectId) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let amulet = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    (game, amulet)
}

fn charge_action(game: &GameState, amulet: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, PlayerId::from_index(0))
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == amulet))
}

fn charge(game: &mut GameState, amulet: ObjectId) {
    let action = charge_action(game, amulet).expect("charge ability is activatable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
}

fn tap_for_mana(game: &mut GameState, amulet: ObjectId) {
    let alice = PlayerId::from_index(0);
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == amulet))
        .expect("mana ability is activatable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
}

fn noted_type_round_trip(paid_with: ManaSymbol) {
    let (mut game, amulet) = setup();
    let alice = PlayerId::from_index(0);
    game.player_mut(alice).unwrap().mana_pool.add(paid_with, 1);
    charge(&mut game, amulet);
    assert_eq!(game.counter_count(amulet, CounterType::Charge), 1);
    assert_eq!(
        game.player(alice).unwrap().mana_pool.total(),
        0,
        "paid {{1}}"
    );

    game.untap(amulet);
    tap_for_mana(&mut game, amulet);
    assert_eq!(
        game.counter_count(amulet, CounterType::Charge),
        0,
        "counter removed"
    );
    let pool = &game.player(alice).unwrap().mana_pool;
    assert_eq!(
        pool.amount(paid_with),
        1,
        "added one mana of the noted type"
    );
    assert_eq!(pool.total(), 1);
}

#[test]
fn stores_red_and_later_adds_red() {
    noted_type_round_trip(ManaSymbol::Red);
}

#[test]
fn stores_colorless_and_later_adds_colorless() {
    noted_type_round_trip(ManaSymbol::Colorless);
}

#[test]
fn charge_ability_only_without_charge_counters() {
    let (mut game, amulet) = setup();
    let alice = PlayerId::from_index(0);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    charge(&mut game, amulet);
    game.untap(amulet);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    assert!(
        charge_action(&game, amulet).is_none(),
        "already has a charge counter"
    );
}

#[test]
fn no_counter_no_mana_ability() {
    let (game, amulet) = setup();
    assert!(
        !compute_legal_actions(&game, PlayerId::from_index(0))
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == amulet)),
        "needs a charge counter to remove"
    );
}
