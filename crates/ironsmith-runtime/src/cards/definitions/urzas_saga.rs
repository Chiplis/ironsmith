//! Urza's Saga card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::{CardType, Subtype};

/// Urza's Saga
/// Enchantment Land — Urza's Saga
/// (As this Saga enters and after your draw step, add a lore counter.)
/// I — Urza's Saga gains "{T}: Add {C}."
/// II — Urza's Saga gains "{2}, {T}: Create a 0/0 colorless Construct artifact creature token
///      with 'This creature gets +1/+1 for each artifact you control.'"
/// III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the
///       battlefield, then shuffle.
///
/// Implementation notes:
/// - Chapter I: Gains tap-for-colorless mana ability
/// - Chapter II: Gains "{2}, {T}: Create a Construct" ability
/// - Chapter II: Creates a Construct token with CharacteristicDefiningPT ability that sets its
///   P/T equal to the number of artifacts the controller controls (evaluated dynamically via CDA)
/// - Chapter III: Searches for artifact with mana cost {0} or {1} - this EXCLUDES:
///   - Cards with no mana cost (like Sol Talisman which can only be cast via suspend)
///   - Cards with X in their cost (like Everflowing Chalice, Walking Ballista)
pub fn urzas_saga() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Urza's Saga")
        // No mana cost (it's a land)
        .card_types(vec![CardType::Enchantment, CardType::Land])
        .subtypes(vec![Subtype::Urzas, Subtype::Saga])
        .saga(3)
        // Chapter III: Search for artifact with mana cost {0} or {1}
        // IMPORTANT: The card says "mana cost {0} or {1}" which means:
        // - Must have a mana cost (excludes suspend-only cards like Sol Talisman)
        // - Must not have X in the cost (excludes Everflowing Chalice, Walking Ballista)
        // - Mana value must be 0 or 1
        .parse_text(
            "(As this Saga enters and after your draw step, add a lore counter.)\n\
             I — Urza's Saga gains \"{T}: Add {C}.\"\n\
             II — Urza's Saga gains \"{2}, {T}: Create a 0/0 colorless Construct artifact \
             creature token with 'This creature gets +1/+1 for each artifact you control.'\"\n\
             III — Search your library for an artifact card with mana cost {0} or {1}, \
             put it onto the battlefield, then shuffle.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardDefinition;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::object::Object;
    use crate::zone::Zone;

    #[test]
    fn test_urzas_saga() {
        let def = urzas_saga();
        assert_eq!(def.name(), "Urza's Saga");

        // Should be enchantment land
        assert!(def.card.card_types.contains(&CardType::Enchantment));
        assert!(def.card.card_types.contains(&CardType::Land));

        // Should have Urza's and Saga subtypes
        assert!(def.card.subtypes.contains(&Subtype::Urzas));
        assert!(def.card.subtypes.contains(&Subtype::Saga));

        // Should have max 3 chapters
        assert_eq!(def.max_saga_chapter, Some(3));

        // Should not start with a mana ability; it's granted by chapter I/II.
        assert!(!def.abilities.iter().any(|a| a.is_mana_ability()));
    }

    #[test]
    fn test_urzas_saga_chapter_trigger() {
        let def = urzas_saga();

        // Should have a chapter 3 trigger (now using Trigger struct)
        let has_chapter_3 = def.abilities.iter().any(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                // Check if the trigger display mentions chapter 3
                t.trigger.display().contains("Chapter")
            } else {
                false
            }
        });
        assert!(has_chapter_3, "Should have chapter 3 trigger");
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_artifact(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn construct_token_def() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Construct")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Construct])
            .power_toughness(PowerToughness::fixed(0, 0))
            .parse_text("This creature gets +1/+1 for each artifact you control.")
            .expect("Construct token text should be supported")
    }

    #[test]
    fn test_construct_token_has_scaling_static_ability() {
        let token_def = construct_token_def();

        let static_abilities: Vec<_> = token_def
            .abilities
            .iter()
            .filter_map(|a| match &a.kind {
                AbilityKind::Static(s) => Some((a, s)),
                _ => None,
            })
            .collect();
        assert_eq!(
            static_abilities.len(),
            1,
            "Token should have one static ability"
        );
        assert!(
            static_abilities[0]
                .0
                .text
                .as_deref()
                .is_some_and(|text| text.contains("artifact you control")),
            "Token should keep the artifact-scaling text"
        );
    }

    #[test]
    fn test_construct_token_static_ability_generates_continuous_effect() {
        use crate::static_ability_processor::generate_continuous_effects_from_static_abilities;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Construct token with the CDA
        let token_id = game.new_object_id();
        let token_def = construct_token_def();
        let token_obj = Object::from_token_definition(token_id, &token_def, alice);
        game.add_object(token_obj);

        // Generate continuous effects from static abilities
        let effects = generate_continuous_effects_from_static_abilities(&game);

        assert!(
            !effects.is_empty(),
            "Construct scaling ability should generate a continuous effect"
        );
    }

    #[test]
    fn test_construct_token_pt_increases_with_artifacts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Construct token with the CDA
        let token_id = game.new_object_id();
        let token_def = construct_token_def();
        let token_obj = Object::from_token_definition(token_id, &token_def, alice);
        game.add_object(token_obj);

        // Create some artifacts for Alice
        let _artifact1 = create_artifact(&mut game, "Sol Ring", alice);
        let _artifact2 = create_artifact(&mut game, "Mana Crypt", alice);
        let _artifact3 = create_artifact(&mut game, "Mox Diamond", alice);

        // The Construct token is also an artifact, so Alice controls 4 artifacts total
        // (the token + 3 other artifacts)

        // Calculate characteristics with the layer system
        let chars = game
            .calculated_characteristics(token_id)
            .expect("Token should exist");

        // The token should have P/T equal to the number of artifacts Alice controls (4)
        assert_eq!(
            chars.power,
            Some(4),
            "Construct should have power 4 (4 artifacts)"
        );
        assert_eq!(
            chars.toughness,
            Some(4),
            "Construct should have toughness 4 (4 artifacts)"
        );
    }

    #[test]
    fn test_construct_token_pt_with_no_other_artifacts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Construct token with the CDA
        let token_id = game.new_object_id();
        let token_def = construct_token_def();
        let token_obj = Object::from_token_definition(token_id, &token_def, alice);
        game.add_object(token_obj);

        // No other artifacts - just the Construct token itself

        // Calculate characteristics with the layer system
        let chars = game
            .calculated_characteristics(token_id)
            .expect("Token should exist");

        // The token should have P/T = 1 (only the Construct token is an artifact)
        assert_eq!(
            chars.power,
            Some(1),
            "Construct should have power 1 (only itself)"
        );
        assert_eq!(
            chars.toughness,
            Some(1),
            "Construct should have toughness 1 (only itself)"
        );
    }

    #[test]
    fn test_construct_token_pt_counts_only_controller_artifacts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a Construct token for Alice
        let token_id = game.new_object_id();
        let token_def = construct_token_def();
        let token_obj = Object::from_token_definition(token_id, &token_def, alice);
        game.add_object(token_obj);

        // Create artifacts for Alice
        let _alice_artifact = create_artifact(&mut game, "Sol Ring", alice);

        // Create artifacts for Bob (shouldn't count)
        let _bob_artifact1 = create_artifact(&mut game, "Bob's Sol Ring", bob);
        let _bob_artifact2 = create_artifact(&mut game, "Bob's Mana Crypt", bob);

        // Calculate characteristics with the layer system
        let chars = game
            .calculated_characteristics(token_id)
            .expect("Token should exist");

        // Alice controls 2 artifacts (token + Sol Ring), Bob's don't count
        assert_eq!(
            chars.power,
            Some(2),
            "Construct should have power 2 (Alice's artifacts only)"
        );
        assert_eq!(
            chars.toughness,
            Some(2),
            "Construct should have toughness 2 (Alice's artifacts only)"
        );
    }
}
