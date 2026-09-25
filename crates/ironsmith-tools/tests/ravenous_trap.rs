//! Ravenous Trap: "If an opponent had three or more cards put into their
//! graveyard from anywhere this turn, you may pay {0} rather than pay this
//! spell's mana cost. Exile target player's graveyard."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerFilter, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Ravenous Trap",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(snapshot.parse_status, ironsmith_tools::ParseStatus::StrictCompiled, "{snapshot:#?}");
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

/// `milled` cards of Bob's go to his graveyard this turn (then `exiled_after`
/// of them leave it); returns whether Alice can cast the Trap with no mana.
fn free_castable(milled: i32, player: PlayerFilter) -> bool {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    for owner in [alice, bob] {
        for i in 0..5 {
            game.create_object_from_definition(
                &CardDefinitionBuilder::new(CardId::new(), format!("Card {i}"))
                    .card_types(vec![CardType::Sorcery])
                    .build(),
                owner,
                Zone::Library,
            );
        }
    }
    let trap = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Hand,
    );
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(trap, alice, &mut dm);
    ironsmith::effects::execute_effect(&mut game, &ironsmith::Effect::mill_player(milled, player), &mut ctx)
        .unwrap();
    let _ = bob;
    compute_legal_actions(&game, alice)
        .into_iter()
        .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if spell_id == trap))
}

#[test]
fn free_after_an_opponent_had_three_cards_put_into_their_graveyard() {
    assert!(free_castable(3, PlayerFilter::Specific(PlayerId::from_index(1))));
}

#[test]
fn two_cards_are_not_enough() {
    assert!(!free_castable(2, PlayerFilter::Specific(PlayerId::from_index(1))));
}

#[test]
fn your_own_graveyard_does_not_count() {
    assert!(!free_castable(3, PlayerFilter::Specific(PlayerId::from_index(0))));
}
