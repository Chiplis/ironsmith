//! Wall of Omens card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Wall of Omens - {1}{W}
/// Creature — Wall (0/4)
/// Defender
/// When Wall of Omens enters the battlefield, draw a card.
pub fn wall_of_omens() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Wall of Omens")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wall])
        .power_toughness(PowerToughness::fixed(0, 4))
        .parse_text("Defender\nWhen Wall of Omens enters the battlefield, draw a card.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_wall_of_omens() {
        let def = wall_of_omens();
        assert_eq!(def.name(), "Wall of Omens");

        let has_defender = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_defender()
            } else {
                false
            }
        });
        // Check ETB trigger (now using Trigger struct)
        let has_etb = def.abilities.iter().any(|a| {
            matches!(
                &a.kind,
                AbilityKind::Triggered(t) if t.trigger.display().contains("enters")
            )
        });

        assert!(has_defender);
        assert!(has_etb);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Wall of Omens ETB draw trigger.
    ///
    /// Wall of Omens: {1}{W} creature
    /// Defender
    /// When Wall of Omens enters the battlefield, draw a card.
    #[test]
    fn test_replay_wall_of_omens_etb_draw() {
        let game = run_replay_test(
            vec![
                "1", // Cast Wall of Omens
                "0", // Tap first Plains for mana
                "0", // Tap second Plains for mana (auto-passes handle resolution)
                     // Wall resolves, ETB trigger goes on stack, auto-passes resolve it
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Wall of Omens"])
                .p1_battlefield(vec!["Plains", "Plains"])
                // Note: Library is LIFO, so last card added is on top
                .p1_deck(vec!["Island", "Swamp", "Mountain"]),
        );

        let alice = PlayerId::from_index(0);

        // Wall of Omens should be on the battlefield
        assert!(
            game.battlefield_has("Wall of Omens"),
            "Wall of Omens should be on battlefield"
        );

        let alice_player = game.player(alice).unwrap();

        // Alice should have drawn a card (started with 1 in hand, cast it = 0, drew 1 = 1)
        assert_eq!(
            alice_player.hand.len(),
            1,
            "Alice should have 1 card in hand (drew from ETB)"
        );

        // Deck should have 2 cards remaining (started with 3, drew 1)
        assert_eq!(
            alice_player.library.len(),
            2,
            "Deck should have 2 cards remaining after drawing 1"
        );

        // The drawn card should be Mountain (top of deck = last added)
        let drawn_card = game.object(alice_player.hand[0]).map(|o| o.name.as_str());
        assert_eq!(
            drawn_card,
            Some("Mountain"),
            "Should have drawn Mountain from top of deck"
        );
    }
}
