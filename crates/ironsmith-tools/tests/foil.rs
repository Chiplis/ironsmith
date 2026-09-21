use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
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
        "Foil",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
fn card(name: &str, island: bool) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![if island {
        CardType::Land
    } else {
        CardType::Sorcery
    }]);
    if island {
        builder = builder.subtypes(vec![Subtype::Island, Subtype::Swamp]);
    }
    builder.build()
}
fn setup(definition: &CardDefinition) -> (GameState, PlayerId, ObjectId, ObjectId) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let target = game.create_object_from_definition(&card("Target spell", false), bob, Zone::Stack);
    game.push_to_stack(ironsmith::game_state::StackEntry::new(target, bob));
    let foil = game.create_object_from_definition(definition, alice, Zone::Hand);
    (game, alice, foil, target)
}
fn cast(game: &mut GameState, alice: PlayerId, foil: ObjectId, method: CastingMethod) {
    let action = compute_legal_actions(game, alice).into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == foil && *casting_method == method)).expect("requested casting method must be legal");
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
        if game.stack.len() == 2 {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(
        game.stack.len(),
        2,
        "casting completes before counter resolution"
    );
}
#[test]
fn strict_snapshot_has_faithful_alternative_cost() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{:?}",
        snapshot.parse_error
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented && snapshot.parse_error.is_none());
    assert!(
        snapshot.similarity_score >= 0.99,
        "{}: {:?}",
        snapshot.similarity_score,
        snapshot.compiled_text
    );
    assert_eq!(definition().alternative_casts.len(), 1);
}
#[test]
fn alternative_requires_two_distinct_hand_cards_including_an_island() {
    let definition = definition();
    for (islands, others, expected) in [
        (0, 0, false),
        (1, 0, false),
        (0, 2, false),
        (1, 1, true),
        (2, 0, true),
    ] {
        let (mut game, alice, foil, _) = setup(&definition);
        for _ in 0..islands {
            game.create_object_from_definition(&card("Nonbasic Island", true), alice, Zone::Hand);
        }
        for _ in 0..others {
            game.create_object_from_definition(&card("Other card", false), alice, Zone::Hand);
        }
        let offered = compute_legal_actions(&game, alice).iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == foil));
        assert_eq!(
            offered, expected,
            "islands={islands}, other hand cards={others}; Foil cannot pay for itself"
        );
    }
}
#[test]
fn alternative_discards_both_cards_and_counters_at_instant_speed() {
    let (mut game, alice, foil, target) = setup(&definition());
    let target_identity = game.object(target).unwrap().stable_id;
    let other = game.create_object_from_definition(&card("Other card", false), alice, Zone::Hand);
    let spare =
        game.create_object_from_definition(&card("Spare non-Island", false), alice, Zone::Hand);
    let island =
        game.create_object_from_definition(&card("Nonbasic Island", true), alice, Zone::Hand);
    let identities = [island, other].map(|id| game.object(id).unwrap().stable_id);
    cast(&mut game, alice, foil, CastingMethod::Alternative(0));
    for identity in identities {
        let discarded = game.find_object_by_stable_id(identity).unwrap();
        assert_eq!(game.object(discarded).unwrap().zone, Zone::Graveyard);
    }
    assert_eq!(game.object(spare).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    let countered = game.find_object_by_stable_id(target_identity).unwrap();
    assert_eq!(game.object(countered).unwrap().zone, Zone::Graveyard);
    assert!(game.stack.is_empty());
}
#[test]
fn normal_mana_cost_does_not_require_discards() {
    let (mut game, alice, foil, _) = setup(&definition());
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    cast(&mut game, alice, foil, CastingMethod::Normal);
    assert!(game.player(alice).unwrap().graveyard.is_empty());
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
}
