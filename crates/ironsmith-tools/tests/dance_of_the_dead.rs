//! Dance of the Dead: "Enchant creature card in a graveyard. When this Aura
//! enters, if it's on the battlefield, it loses 'enchant creature card in a
//! graveyard' and gains 'enchant creature put onto the battlefield with this
//! Aura.' Put enchanted creature card onto the battlefield tapped under your
//! control and attach this Aura to it. When this Aura leaves the battlefield,
//! that creature's controller sacrifices it. Enchanted creature gets +1/+1 and
//! doesn't untap during its controller's untap step. At the beginning of the
//! upkeep of enchanted creature's controller, that player may pay {1}{B}. If
//! the player does, untap that creature."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::{CardId, StableId};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::AttachmentTarget;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Dance of the Dead",
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

struct Pay(bool);

impl DecisionMaker for Pay {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.0
    }
}

struct Board {
    game: GameState,
    queue: TriggerQueue,
    aura: ObjectId,
    creature: StableId,
}

/// Alice casts Dance of the Dead on Bob's 3/3 in his graveyard and the enter
/// trigger resolves.
fn reanimate() -> Board {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let fixture = CardDefinitionBuilder::new(CardId::new(), "Buried Ogre")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3))
        .build();
    let buried = game.create_object_from_definition(&fixture, bob, Zone::Graveyard);
    let creature = game.object(buried).unwrap().stable_id;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 2);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let aura_stable = game.object(hand).unwrap().stable_id;
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
        .expect("castable on a graveyard creature card");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Pay(false);
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..12 {
        if !game.stack.is_empty() || progress.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = progress else {
            break;
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{progress:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1, "enter trigger");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let aura = game.find_object_by_stable_id(aura_stable).unwrap();
    Board {
        game,
        queue,
        aura,
        creature,
    }
}

#[test]
fn returns_tapped_under_your_control_attached_with_plus_one() {
    let board = reanimate();
    let game = &board.game;
    let returned = game.find_object_by_stable_id(board.creature).unwrap();
    assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.controller_of_id(returned),
        Some(PlayerId::from_index(0))
    );
    assert!(game.is_tapped(returned), "enters tapped");
    assert_eq!(
        game.object(board.aura).unwrap().attached_to,
        Some(AttachmentTarget::Object(returned))
    );
    assert_eq!(game.calculated_power(returned), Some(4));
    assert_eq!(game.calculated_toughness(returned), Some(4));
}

#[test]
fn doesnt_untap_normally_but_upkeep_payment_untaps_it() {
    for pay in [false, true] {
        let mut board = reanimate();
        let alice = PlayerId::from_index(0);
        let returned = board.game.find_object_by_stable_id(board.creature).unwrap();
        board.game.turn.turn_number = 5;
        board.game.turn.phase = ironsmith::game_state::Phase::Beginning;
        board.game.turn.step = Some(ironsmith::game_state::Step::Untap);
        ironsmith::turn::execute_untap_step(&mut board.game);
        assert!(
            board.game.is_tapped(returned),
            "doesn't untap during untap step"
        );
        board.game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        board
            .game
            .player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        board
            .game
            .player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        for event in
            ironsmith::triggers::generate_step_trigger_events_for_active_players(&board.game)
        {
            for entry in ironsmith::triggers::check_triggers(&board.game, &event) {
                board.queue.add(entry);
            }
        }
        let mut dm = Pay(pay);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut board.game,
            &mut board.queue,
            &mut dm,
        )
        .unwrap();
        assert_eq!(board.game.stack.len(), 1, "upkeep trigger");
        ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).unwrap();
        assert_eq!(
            !board.game.is_tapped(returned),
            pay,
            "untaps only if {{1}}{{B}} is paid"
        );
        let pool = board.game.player(alice).unwrap().mana_pool.total();
        assert_eq!(pool, if pay { 0 } else { 2 });
    }
}

#[test]
fn aura_leaving_makes_the_creatures_controller_sacrifice_it() {
    let mut board = reanimate();
    let returned = board.game.find_object_by_stable_id(board.creature).unwrap();
    board
        .game
        .set_current_controller(returned, PlayerId::from_index(1));
    board
        .game
        .move_object_by_effect(board.aura, Zone::Graveyard)
        .unwrap();
    let mut dm = Pay(false);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut board.game, &mut board.queue, &mut dm)
        .unwrap();
    assert_eq!(board.game.stack.len(), 1, "leave trigger");
    ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).unwrap();
    let now = board.game.find_object_by_stable_id(board.creature).unwrap();
    assert_eq!(
        board.game.object(now).unwrap().zone,
        Zone::Graveyard,
        "sacrificed by its controller"
    );
}
