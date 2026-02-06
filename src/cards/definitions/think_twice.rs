//! Card definition for Think Twice.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Think Twice card definition.
///
/// Think Twice {1}{U}
/// Instant
/// Draw a card.
/// Flashback {2}{U}
pub fn think_twice() -> CardDefinition {
    let mut def = CardDefinitionBuilder::new(CardId::new(), "Think Twice")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .flashback(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .parse_text("Draw a card.")
        .expect("Card text should be supported");

    def.card.oracle_text = format!("{}\nFlashback {{2}}{{U}}", def.card.oracle_text);

    def
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_think_twice() {
        let card = think_twice();

        assert_eq!(card.card.name, "Think Twice");
        assert!(card.card.card_types.contains(&CardType::Instant));

        // Check normal mana cost is {1}{U}
        let cost = card.card.mana_cost.as_ref().expect("Should have mana cost");
        assert_eq!(cost.mana_value(), 2);

        // Check it has flashback
        assert_eq!(card.alternative_casts.len(), 1);

        // Check flashback cost is {2}{U}
        if let crate::alternative_cast::AlternativeCastingMethod::Flashback { cost } =
            &card.alternative_casts[0]
        {
            assert_eq!(cost.mana_value(), 3);
        } else {
            panic!("Expected Flashback alternative casting method");
        }

        // Check spell effect is draw 1
        let effects = card
            .spell_effect
            .as_ref()
            .expect("Should have spell effect");
        assert_eq!(effects.len(), 1);
        let debug_str = format!("{:?}", &effects[0]);
        assert!(debug_str.contains("DrawCardsEffect"));
        assert!(debug_str.contains("Fixed(1)"));
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Think Twice normally from hand.
    ///
    /// Think Twice: {1}{U} instant
    /// Draw a card.
    /// Flashback {2}{U}
    #[test]
    fn test_replay_think_twice_from_hand() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Think Twice
                "0", // Tap first Island for mana
                "0", // Tap second Island for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Think Twice"])
                .p1_battlefield(vec!["Island", "Island"])
                .p1_deck(vec!["Mountain", "Forest"]),
        );

        let alice = PlayerId::from_index(0);

        // Think Twice should be in graveyard (after resolving)
        let alice_player = game.player(alice).unwrap();
        let tt_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            tt_in_gy,
            "Think Twice should be in graveyard after resolving"
        );

        // Alice should have drawn a card
        assert_eq!(
            alice_player.hand.len(),
            1,
            "Alice should have 1 card in hand (drew from Think Twice)"
        );
    }

    /// Tests casting Think Twice with flashback from graveyard.
    ///
    /// Flashback {2}{U} - must exile after resolving
    #[test]
    fn test_replay_think_twice_flashback() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Think Twice is in graveyard, we should be able to cast it with flashback
                // Actions: 0=pass, 1=cast Think Twice (flashback), then pay mana
                "1", // Cast Think Twice from graveyard (flashback)
                "0", // Tap first Island for mana
                "0", // Tap second Island for mana
                "0", // Tap third Island for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_graveyard(vec!["Think Twice"])
                .p1_battlefield(vec!["Island", "Island", "Island"])
                .p1_deck(vec!["Mountain", "Forest"]),
        );

        let alice = PlayerId::from_index(0);

        // Think Twice should be in EXILE (flashback exiles after resolving)
        let tt_in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(tt_in_exile, "Think Twice should be exiled after flashback");

        // It should NOT be in graveyard anymore
        let alice_player = game.player(alice).unwrap();
        let tt_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            !tt_in_gy,
            "Think Twice should not be in graveyard after flashback"
        );

        // Alice should have drawn a card
        assert_eq!(
            alice_player.hand.len(),
            1,
            "Alice should have 1 card in hand (drew from Think Twice)"
        );
    }
}
