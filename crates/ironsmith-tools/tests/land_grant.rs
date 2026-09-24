//! Land Grant: "If you have no land cards in hand, you may reveal your hand
//! rather than pay this spell's mana cost. Search your library for a Forest
//! card, reveal that card, put it into your hand, then shuffle."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Land Grant",
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

fn card(name: &str, card_type: CardType, subtype: Option<Subtype>) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Land {
        builder = builder.supertypes(vec![Supertype::Basic]);
    }
    if let Some(subtype) = subtype {
        builder = builder.subtypes(vec![subtype]);
    }
    builder.build()
}

/// Alice has no mana; her hand holds Land Grant plus `other`.
fn setup(other: Option<(&str, CardType)>) -> (GameState, ObjectId) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&card("Forest", CardType::Land, Some(Subtype::Forest)), alice, Zone::Library);
    game.create_object_from_definition(&card("Island", CardType::Land, Some(Subtype::Island)), alice, Zone::Library);
    if let Some((name, card_type)) = other {
        game.create_object_from_definition(&card(name, card_type, None), alice, Zone::Hand);
    }
    let spell = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Hand,
    );
    (game, spell)
}

fn castable(game: &GameState, spell: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, PlayerId::from_index(0))
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
}

#[test]
fn with_no_land_cards_in_hand_it_is_cast_by_revealing_the_hand() {
    let (mut game, spell) = setup(Some(("Spare Sorcery", CardType::Sorcery)));
    let action = castable(&game, spell).expect("the reveal alternative cost needs no mana");
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
        result = ironsmith::game_loop::apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm);
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let alice = PlayerId::from_index(0);
    let mut hand: Vec<String> = game
        .player(alice)
        .unwrap()
        .hand
        .iter()
        .map(|id| game.object(*id).unwrap().name.to_string())
        .collect();
    hand.sort();
    assert_eq!(hand, vec!["Forest".to_string(), "Spare Sorcery".to_string()]);
}

#[test]
fn a_land_card_in_hand_forbids_the_alternative_cost() {
    let (game, spell) = setup(Some(("Swamp", CardType::Land)));
    assert!(castable(&game, spell).is_none(), "no mana and a land in hand");
}
