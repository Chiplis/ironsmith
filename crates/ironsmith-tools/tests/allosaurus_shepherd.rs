//! Allosaurus Shepherd: "This spell can't be countered. Green spells you
//! control can't be countered. {4}{G}{G}: Until end of turn, each Elf creature
//! you control has base power and toughness 5/5 and becomes a Dinosaur in
//! addition to its other creature types."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Allosaurus Shepherd",
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

fn creature(name: &str, subtype: Subtype) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

struct Board {
    game: GameState,
    shepherd: ObjectId,
    my_elf: ObjectId,
    my_goblin: ObjectId,
    their_elf: ObjectId,
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
    let shepherd = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let my_elf = game.create_object_from_definition(
        &creature("Llanowar Scout", Subtype::Elf),
        alice,
        Zone::Battlefield,
    );
    let my_goblin = game.create_object_from_definition(
        &creature("Goblin Scout", Subtype::Goblin),
        alice,
        Zone::Battlefield,
    );
    let their_elf = game.create_object_from_definition(
        &creature("Enemy Elf", Subtype::Elf),
        bob,
        Zone::Battlefield,
    );
    Board {
        game,
        shepherd,
        my_elf,
        my_goblin,
        their_elf,
    }
}

fn activate(board: &mut Board) {
    let alice = PlayerId::from_index(0);
    let game = &mut board.game;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 6);
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == board.shepherd))
        .expect("activatable");
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
    assert_eq!(
        game.player(alice).unwrap().mana_pool.total(),
        0,
        "paid {{4}}{{G}}{{G}}"
    );
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
}

fn is_dinosaur(game: &GameState, id: ObjectId) -> bool {
    game.calculated_subtypes(id).contains(&Subtype::Dinosaur)
}

#[test]
fn your_elves_become_5_5_dinosaurs_until_end_of_turn() {
    let mut board = board();
    activate(&mut board);
    let game = &board.game;
    for elf in [board.shepherd, board.my_elf] {
        assert_eq!(game.calculated_power(elf), Some(5));
        assert_eq!(game.calculated_toughness(elf), Some(5));
        assert!(is_dinosaur(game, elf));
        assert!(
            game.calculated_subtypes(elf).contains(&Subtype::Elf),
            "in addition to its other types"
        );
    }
    for other in [board.my_goblin, board.their_elf] {
        assert_eq!(game.calculated_power(other), Some(1));
        assert!(!is_dinosaur(game, other));
    }

    ironsmith::turn::execute_cleanup_step(&mut board.game);
    assert_eq!(board.game.calculated_power(board.my_elf), Some(1));
    assert!(!is_dinosaur(&board.game, board.my_elf));
}

#[test]
fn elves_that_arrive_after_resolution_are_unaffected() {
    let mut board = board();
    activate(&mut board);
    let alice = PlayerId::from_index(0);
    let late = board.game.create_object_from_definition(
        &creature("Late Elf", Subtype::Elf),
        alice,
        Zone::Battlefield,
    );
    assert_eq!(
        board.game.calculated_power(late),
        Some(1),
        "CR 611.2c: affected set locked in"
    );
    assert!(!is_dinosaur(&board.game, late));
}

#[test]
fn shepherd_and_green_spells_cannot_be_countered() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("CantBeCountered")
            || debug.contains("cant_be_countered")
            || debug.contains("Uncounterable"),
        "{debug}"
    );
}
