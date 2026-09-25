//! Aven Mindcensor: "Flash. Flying. If an opponent would search a library,
//! that player searches the top four cards of that library instead."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};

fn load(name: &str) -> ironsmith::cards::CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Aven Mindcensor",
    )
    .unwrap()
    .remove(0);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload);
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

fn forest() -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![Subtype::Forest])
        .build()
}

fn filler(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

/// `searcher` cracks an Evolving Wilds while Alice controls Aven Mindcensor.
/// The searcher's library (bottom-first) has a Forest at `forest_depth` cards
/// from the top (0 = top card) among sorceries. Returns whether a Forest
/// reached the battlefield.
fn crack_wilds(searcher: PlayerId, forest_depth: usize) -> bool {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = searcher;
    game.turn.priority_player = Some(searcher);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&load("Aven Mindcensor"), alice, Zone::Battlefield);
    let total = 8;
    for depth_from_top in (0..total).rev() {
        let card = if depth_from_top == forest_depth {
            forest()
        } else {
            filler(&format!("Filler {depth_from_top}"))
        };
        game.create_object_from_definition(&card, searcher, Zone::Library);
    }
    let wilds: ObjectId =
        game.create_object_from_definition(&load("Evolving Wilds"), searcher, Zone::Battlefield);
    let action = compute_legal_actions(&game, searcher)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == wilds))
        .expect("Evolving Wilds activates");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
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
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    game.battlefield.iter().any(|id| {
        game.object(*id)
            .is_some_and(|o| o.name.as_str() == "Forest")
    })
}

#[test]
fn an_opponent_only_searches_the_top_four_cards() {
    let bob = PlayerId::from_index(1);
    assert!(crack_wilds(bob, 3), "a Forest fourth from the top is found");
    assert!(
        !crack_wilds(bob, 4),
        "a Forest fifth from the top is out of reach"
    );
}

#[test]
fn the_controller_searches_normally() {
    let alice = PlayerId::from_index(0);
    assert!(crack_wilds(alice, 6), "Alice is not an opponent of herself");
}
