//! Generous Gift card definition.

#[cfg(test)]
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
#[cfg(test)]
use crate::color::ColorSet;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;
#[cfg(test)]
use crate::types::Subtype;

/// Creates a 3/3 green Elephant creature token.
#[cfg(test)]
fn elephant_token() -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), "Elephant")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elephant])
            .color_indicator(ColorSet::GREEN)
            .power_toughness(PowerToughness::fixed(3, 3))
            .token()
            .build(),
    )
}

/// Generous Gift - {2}{W}
/// Instant
/// Destroy target permanent. Its controller creates a 3/3 green Elephant creature token.
pub fn generous_gift() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Generous Gift")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target permanent. Its controller creates a 3/3 green Elephant creature token.",
        )
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
    fn test_generous_gift_basic_properties() {
        let def = generous_gift();
        assert_eq!(def.name(), "Generous Gift");
        assert!(def.is_spell());
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_generous_gift_is_white() {
        let def = generous_gift();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_generous_gift_has_spell_effects() {
        let def = generous_gift();
        assert!(def.spell_effect.is_some());

        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 2);

        // First effect is destroy
        let debug_str = format!("{:?}", &effects[0]);
        assert!(
            debug_str.contains("Destroy"),
            "First effect should be destroy"
        );

        // Second effect is create token
        let debug_str2 = format!("{:?}", &effects[1]);
        assert!(
            debug_str2.contains("CreateToken"),
            "Second effect should create tokens"
        );
    }

    #[test]
    fn test_generous_gift_oracle_text() {
        let def = generous_gift();
        assert!(def.card.oracle_text.contains("Destroy target permanent"));
        assert!(def.card.oracle_text.contains("3/3 green Elephant"));
    }

    // ========================================
    // Token Tests
    // ========================================

    #[test]
    fn test_elephant_token_properties() {
        let token = elephant_token();
        assert_eq!(token.name(), "Elephant");
        assert!(token.is_creature());
        assert!(token.card.has_subtype(Subtype::Elephant));
        assert!(token.card.colors().contains(Color::Green));

        let pt = token.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power.base_value(), 3);
        assert_eq!(pt.toughness.base_value(), 3);
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests casting Generous Gift to destroy own creature.
    ///
    /// Generous Gift: {2}{W} Instant
    /// Destroy target permanent. Its controller creates a 3/3 green Elephant creature token.
    #[test]
    fn test_replay_generous_gift_destroy_creature() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // Input sequence:
        // - "1": Cast Generous Gift (action 0 = Pass, action 1 = Cast)
        // - "0": Target Grizzly Bears (creatures listed first)
        // - "0", "0": Pay mana (2 choices needed for {2}{W} with 3 identical Plains)
        // After mana payment, both players auto-pass (no mana abilities available)
        let game = run_replay_test(
            vec![
                "1", // Cast Generous Gift
                "3", // Target Llanowar Elves
                "0", // Pay mana choice 1
                "0", // Pay mana choice 2
                "0", // Bob passes priority (has mana ability, needs explicit pass)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Generous Gift"])
                .p1_battlefield(vec!["Plains", "Plains", "Plains", "Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);

        // Grizzly Bears should be in graveyard (destroyed)
        let alice_player = game.player(alice).unwrap();
        let bears_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(bears_in_gy, "Grizzly Bears should be in graveyard");

        // Alice should have an Elephant token (since she controlled the destroyed permanent)
        assert!(
            game.battlefield_has("Elephant"),
            "Alice should have an Elephant token"
        );

        // Generous Gift should be in graveyard
        let spell_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Generous Gift")
                .unwrap_or(false)
        });
        assert!(spell_in_gy, "Generous Gift should be in graveyard");
    }

    /// Tests casting Generous Gift on an opponent's creature.
    /// The destroyed permanent's controller gets the Elephant token.
    #[test]
    fn test_replay_generous_gift_destroy_opponent_creature() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // Input sequence:
        // - "1": Cast Generous Gift
        // - "3": Target Llanowar Elves
        // - "0", "0": Pay mana
        // - "0": Bob passes priority (has Llanowar Elves mana ability available)
        // After Bob passes, Alice auto-passes, spell resolves
        let game = run_replay_test(
            vec![
                "1", // Cast Generous Gift
                "3", // Target Llanowar Elves
                "0", // Pay mana choice 1
                "0", // Pay mana choice 2
                "0", // Bob passes priority (has mana ability, needs explicit pass)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Generous Gift"])
                .p1_battlefield(vec!["Plains", "Plains", "Plains"])
                .p2_battlefield(vec!["Llanowar Elves"]),
        );

        let bob = PlayerId::from_index(1);

        // Llanowar Elves should be in Bob's graveyard (destroyed)
        let bob_player = game.player(bob).unwrap();
        let elves_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Llanowar Elves")
                .unwrap_or(false)
        });
        assert!(elves_in_gy, "Llanowar Elves should be in graveyard");

        // Bob should have an Elephant token (since he controlled the destroyed permanent)
        // Check if any Elephant on battlefield is controlled by Bob
        let elephant_for_bob = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Elephant" && o.controller == bob)
                .unwrap_or(false)
        });
        assert!(elephant_for_bob, "Bob should have an Elephant token");
    }
}
