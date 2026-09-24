//! Coveted Jewel: "When this artifact enters, draw three cards. {T}: Add three
//! mana of any one color. Whenever one or more creatures an opponent controls
//! attack you and aren't blocked, that player draws three cards and gains
//! control of this artifact. Untap it."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration, SelectFirstDecisionMaker};
use ironsmith::ids::CardId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Coveted Jewel",
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

fn bear(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

struct Board {
    game: GameState,
    jewel: ObjectId,
    attackers: Vec<ObjectId>,
    blocker: ObjectId,
}

/// Alice controls Coveted Jewel (tapped) and a blocker; it's Bob's turn and
/// he controls `attackers` Bears, each with ten cards in his library.
fn board(attackers: usize) -> Board {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 4;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    for i in 0..10 {
        game.create_object_from_definition(&bear(&format!("Library Bear {i}")), bob, Zone::Library);
    }
    let jewel = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.tap(jewel);
    let blocker = game.create_object_from_definition(&bear("Alice Bear"), alice, Zone::Battlefield);
    let attackers = (0..attackers)
        .map(|i| game.create_object_from_definition(&bear(&format!("Bob Bear {i}")), bob, Zone::Battlefield))
        .collect();
    Board {
        game,
        jewel,
        attackers,
        blocker,
    }
}

/// Bob attacks Alice with every Bear; Alice blocks the `blocked` ones with her
/// Bear. The declare-blockers triggers then resolve.
fn declare_blocks(board: &mut Board, blocked: &[usize]) {
    let alice = PlayerId::from_index(0);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    for id in &board.attackers {
        board.game.remove_summoning_sickness(*id);
    }
    let attacks: Vec<AttackerDeclaration> = board
        .attackers
        .iter()
        .map(|id| AttackerDeclaration {
            creature: *id,
            target: AttackTarget::Player(alice),
        })
        .collect();
    ironsmith::game_loop::apply_attacker_declarations(&mut board.game, &mut combat, &mut queue, &attacks)
        .unwrap();
    let blocks: Vec<BlockerDeclaration> = blocked
        .iter()
        .map(|i| BlockerDeclaration {
            blocker: board.blocker,
            blocking: board.attackers[*i],
        })
        .collect();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_blocker_declarations(&mut board.game, &mut combat, &mut queue, &blocks, alice)
        .unwrap();
    let mut dm = SelectFirstDecisionMaker;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut board.game, &mut queue, &mut dm).unwrap();
    while !board.game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).unwrap();
    }
}

#[test]
fn unblocked_attackers_trigger_once_and_hand_over_the_untapped_jewel() {
    let mut board = board(2);
    declare_blocks(&mut board, &[]);
    let bob = PlayerId::from_index(1);
    assert_eq!(board.game.player(bob).unwrap().hand.len(), 3, "one trigger for the group");
    assert_eq!(board.game.controller_of_id(board.jewel), Some(bob));
    assert!(!board.game.is_tapped(board.jewel), "untapped");
}

#[test]
fn one_unblocked_attacker_is_enough() {
    let mut board = board(2);
    declare_blocks(&mut board, &[0]);
    let bob = PlayerId::from_index(1);
    assert_eq!(board.game.player(bob).unwrap().hand.len(), 3);
    assert_eq!(board.game.controller_of_id(board.jewel), Some(bob));
}

#[test]
fn fully_blocked_attack_does_not_trigger() {
    let mut board = board(1);
    declare_blocks(&mut board, &[0]);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    assert_eq!(board.game.player(bob).unwrap().hand.len(), 0);
    assert_eq!(board.game.controller_of_id(board.jewel), Some(alice));
    assert!(board.game.is_tapped(board.jewel));
}

#[test]
fn attacking_only_alices_planeswalker_does_not_trigger() {
    let mut board = board(1);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let walker = CardDefinitionBuilder::new(CardId::new(), "Alice Walker")
        .card_types(vec![CardType::Planeswalker])
        .build();
    let walker = board.game.create_object_from_definition(&walker, alice, Zone::Battlefield);
    board.game.remove_summoning_sickness(board.attackers[0]);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut board.game,
        &mut combat,
        &mut queue,
        &[AttackerDeclaration {
            creature: board.attackers[0],
            target: AttackTarget::Planeswalker(walker),
        }],
    )
    .unwrap();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_blocker_declarations(&mut board.game, &mut combat, &mut queue, &[], alice)
        .unwrap();
    let mut dm = SelectFirstDecisionMaker;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut board.game, &mut queue, &mut dm).unwrap();
    assert!(board.game.stack.is_empty(), "\"attack you\" excludes planeswalkers");
    assert_eq!(board.game.controller_of_id(board.jewel), Some(alice));
    let _ = bob;
}
