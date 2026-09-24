//! Goblin Welder: "{T}: Choose target artifact a player controls and target
//! artifact card in that player's graveyard. If both targets are still legal
//! as this ability resolves, that player simultaneously sacrifices the artifact
//! and returns the artifact card to the battlefield."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::events::cause::EventCause;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Goblin Welder",
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

/// Picks the given targets and records the offered legal targets per slot.
struct Pick {
    targets: Vec<ObjectId>,
    offered: Vec<Vec<Target>>,
}

impl DecisionMaker for Pick {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.offered = ctx.requirements.iter().map(|r| r.legal_targets.clone()).collect();
        self.targets.iter().map(|id| Target::Object(*id)).collect()
    }
}

fn artifact(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

struct Board {
    game: GameState,
    welder: ObjectId,
    bob_rock: ObjectId,
    bob_relic: ObjectId,
    alice_relic: ObjectId,
}

fn board() -> Board {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let welder = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(welder);
    let bob_rock = game.create_object_from_definition(&artifact("Bob Rock"), bob, Zone::Battlefield);
    let bob_relic = game.create_object_from_definition(&artifact("Bob Relic"), bob, Zone::Graveyard);
    let alice_relic = game.create_object_from_definition(&artifact("Alice Relic"), alice, Zone::Graveyard);
    Board {
        game,
        welder,
        bob_rock,
        bob_relic,
        alice_relic,
    }
}

/// Activates with `targets`; returns whether the ability reached the stack.
fn activate(board: &mut Board, dm: &mut Pick) -> bool {
    let alice = PlayerId::from_index(0);
    let action = compute_legal_actions(&board.game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == board.welder))
        .expect("activatable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(board.game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut board.game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if !board.game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut board.game,
            &mut queue,
            &mut state,
            &ctx,
            dm,
        );
    }
    board.game.stack.len() == 1
}

fn zone_of(game: &GameState, name: &str) -> Option<Zone> {
    game.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name.as_str() == name)
        .map(|o| o.zone)
}

#[test]
fn swaps_an_artifact_for_an_artifact_card_in_its_controllers_graveyard() {
    let mut board = board();
    let bob = PlayerId::from_index(1);
    let mut dm = Pick {
        targets: vec![board.bob_rock, board.bob_relic],
        offered: Vec::new(),
    };
    assert!(activate(&mut board, &mut dm), "offered {:?} rock={:?} relic={:?} alice_relic={:?}", dm.offered, board.bob_rock, board.bob_relic, board.alice_relic);
    ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).unwrap();
    assert_eq!(zone_of(&board.game, "Bob Rock"), Some(Zone::Graveyard), "sacrificed");
    assert_eq!(zone_of(&board.game, "Bob Relic"), Some(Zone::Battlefield), "returned");
    let relic = board
        .game
        .objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name.as_str() == "Bob Relic")
        .unwrap()
        .id;
    assert_eq!(board.game.controller_of_id(relic), Some(bob), "returns under that player's control");
    assert!(board.game.is_tapped(board.welder));
}

#[test]
fn the_card_must_be_in_the_artifact_controllers_graveyard() {
    let mut board = board();
    let mut dm = Pick {
        targets: vec![board.bob_rock, board.alice_relic],
        offered: Vec::new(),
    };
    assert!(!activate(&mut board, &mut dm), "Alice's graveyard card does not match Bob's artifact");
    assert_eq!(zone_of(&board.game, "Alice Relic"), Some(Zone::Graveyard));
}

#[test]
fn nothing_happens_if_either_target_becomes_illegal() {
    for remove_card in [true, false] {
        let mut board = board();
        let mut dm = Pick {
            targets: vec![board.bob_rock, board.bob_relic],
            offered: Vec::new(),
        };
        assert!(activate(&mut board, &mut dm));
        if remove_card {
            board.game.move_object(board.bob_relic, Zone::Exile, EventCause::effect());
        } else {
            board.game.move_object(board.bob_rock, Zone::Hand, EventCause::effect());
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).unwrap();
        if remove_card {
            assert_eq!(zone_of(&board.game, "Bob Rock"), Some(Zone::Battlefield), "not sacrificed");
        } else {
            assert_eq!(zone_of(&board.game, "Bob Relic"), Some(Zone::Graveyard), "not returned");
        }
    }
}
