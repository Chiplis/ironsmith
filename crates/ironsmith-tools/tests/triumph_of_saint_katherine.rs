//! Triumph of Saint Katherine: "Praesidium Protectiva — When this creature is
//! put into your graveyard from the battlefield, exile it and the top six
//! cards of your library in a face-down pile. If you do, shuffle that pile
//! and put it back on top of your library."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::{CardId, StableId};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Triumph of Saint Katherine",
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

/// Alice controls Triumph with a ten-card library; it is destroyed. Returns
/// (library names top-first, exile count, Triumph's zone).
fn dies() -> (Vec<String>, usize, Zone) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    for i in 0..10 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), format!("Card {i}"))
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Library,
        );
    }
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let triumph = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let stable: StableId = game.object(triumph).unwrap().stable_id;
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(triumph, alice, &mut dm);
    ironsmith::effects::execute_effect(
        &mut game,
        &ironsmith::Effect::destroy(ChooseSpec::SpecificObject(triumph)),
        &mut ctx,
    )
    .unwrap();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1, "the dies trigger");
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    }
    let library = game
        .player(alice)
        .unwrap()
        .library
        .iter()
        .rev()
        .map(|id| game.object(*id).unwrap().name.to_string())
        .collect();
    let zone = game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone;
    (library, game.exile.len(), zone)
}

#[test]
fn it_and_the_top_six_cards_go_back_on_top() {
    let (library, exiled, zone) = dies();
    assert_eq!(zone, Zone::Library);
    assert_eq!(exiled, 0);
    assert_eq!(library.len(), 11);
    let mut top: Vec<String> = library[..7].to_vec();
    top.sort();
    let mut expected: Vec<String> = (0..6).map(|i| format!("Card {}", 9 - i)).collect();
    expected.push("Triumph of Saint Katherine".to_string());
    expected.sort();
    assert_eq!(top, expected, "the pile is the old top six plus Triumph");
    let rest: Vec<String> = library[7..].to_vec();
    assert_eq!(rest, ["Card 3", "Card 2", "Card 1", "Card 0"]);
}
