//! Aluren: "Any player may cast creature spells with mana value 3 or less
//! without paying their mana costs and as though they had flash."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Aluren",
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

fn card(name: &str, card_type: CardType, generic: u8) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(generic)],
            vec![ManaSymbol::Green],
        ]));
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

/// Alice controls Aluren during her own combat step (not a main phase), so
/// only flash-timed spells are castable. `player` holds the test cards and
/// has priority with no mana.
fn castable_for(player: PlayerId) -> Vec<String> {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(player);
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Battlefield,
    );
    let cards: Vec<(ObjectId, String)> = [
        ("Bear (MV 3)", CardType::Creature, 2),
        ("Giant (MV 4)", CardType::Creature, 3),
        ("Growth (MV 3 sorcery)", CardType::Sorcery, 2),
    ]
    .into_iter()
    .map(|(name, card_type, generic)| {
        (
            game.create_object_from_definition(&card(name, card_type, generic), player, Zone::Hand),
            name.to_string(),
        )
    })
    .collect();
    let actions = compute_legal_actions(&game, player);
    let mut castable: Vec<String> = cards
        .into_iter()
        .filter(|(id, _)| {
            actions
                .iter()
                .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if spell_id == id))
        })
        .map(|(_, name)| name)
        .collect();
    castable.sort();
    castable
}

#[test]
fn any_player_casts_small_creatures_free_at_instant_speed() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        assert_eq!(
            castable_for(player),
            vec!["Bear (MV 3)".to_string()],
            "only the mana value 3 creature, for player {player:?}"
        );
    }
}
