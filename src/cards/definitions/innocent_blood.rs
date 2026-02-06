//! Innocent Blood card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Innocent Blood - {B}
/// Sorcery
/// Each player sacrifices a creature of their choice.
pub fn innocent_blood() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Innocent Blood")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each player sacrifices a creature of their choice.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_innocent_blood_basic_properties() {
        let def = innocent_blood();
        assert_eq!(def.name(), "Innocent Blood");
        assert!(def.is_spell());
        assert!(def.card.is_sorcery());
        assert!(!def.card.is_instant());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_innocent_blood_is_black() {
        let def = innocent_blood();
        assert!(def.card.colors().contains(Color::Black));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_innocent_blood_has_spell_effect() {
        let def = innocent_blood();
        assert!(def.spell_effect.is_some());

        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);

        // Check it includes ForPlayersEffect composition
        let debug_str = format!("{:?}", &effects);
        assert!(
            debug_str.contains("ForPlayersEffect"),
            "Should include ForPlayersEffect composition"
        );
    }

    #[test]
    fn test_innocent_blood_oracle_text() {
        let def = innocent_blood();
        assert!(def.card.oracle_text.contains("sacrifices a creature"));
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests casting Innocent Blood when both players have creatures.
    ///
    /// Innocent Blood: {B} Sorcery
    /// Each player sacrifices a creature of their choice.
    #[test]
    fn test_replay_innocent_blood_casting() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Innocent Blood
                "0", // Tap Swamp for mana
                "0", // Choose Alice's creature to sacrifice
                "0", // Choose Bob's creature to sacrifice
                "0", // Pass priority after resolution
                     // Spell resolves - EachPlayerSacrificesEffect runs automatically
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Innocent Blood"])
                .p1_battlefield(vec!["Swamp", "Grizzly Bears"])
                .p2_battlefield(vec!["Llanowar Elves"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice's Grizzly Bears should be in graveyard
        let alice_player = game.player(alice).unwrap();
        let bears_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(
            bears_in_gy,
            "Grizzly Bears should be in graveyard after sacrifice"
        );

        // Bob's Llanowar Elves should be in graveyard
        let bob_player = game.player(bob).unwrap();
        let elves_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Llanowar Elves")
                .unwrap_or(false)
        });
        assert!(
            elves_in_gy,
            "Llanowar Elves should be in graveyard after sacrifice"
        );

        // Innocent Blood should be in graveyard
        let spell_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Innocent Blood")
                .unwrap_or(false)
        });
        assert!(
            spell_in_gy,
            "Innocent Blood should be in graveyard after resolving"
        );
    }
}
