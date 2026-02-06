//! Brightclimb Pathway // Grimclimb Pathway card definition.
//!
//! This is a Modal Double-Faced Card (MDFC). Both faces are lands.
//! When playing from hand, the player chooses which face to play.
//!
//! Note: Full MDFC support requires game-level handling for choosing which face
//! to play. This file defines both faces as separate card definitions.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

/// Brightclimb Pathway (front face)
/// Land
/// {T}: Add {W}.
pub fn brightclimb_pathway() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Brightclimb Pathway")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {W}.")
        .expect("Card text should be supported")
}

/// Grimclimb Pathway (back face)
/// Land
/// {T}: Add {B}.
pub fn grimclimb_pathway() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Grimclimb Pathway")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {B}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Brightclimb Pathway (Front Face) Tests
    // ========================================

    #[test]
    fn test_brightclimb_pathway_basic_properties() {
        let def = brightclimb_pathway();
        assert_eq!(def.name(), "Brightclimb Pathway");
        assert!(def.card.is_land());
        assert!(!def.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_brightclimb_pathway_is_colorless() {
        let def = brightclimb_pathway();
        // Lands without mana cost and no color indicator are colorless
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_brightclimb_pathway_color_identity() {
        let def = brightclimb_pathway();
        use crate::color::Color;
        // Color identity includes colors in rules text ({W})
        assert!(def.card.color_identity().contains(Color::White));
        assert!(!def.card.color_identity().contains(Color::Black));
    }

    #[test]
    fn test_brightclimb_pathway_has_mana_ability() {
        let def = brightclimb_pathway();
        assert_eq!(def.abilities.len(), 1);
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_brightclimb_pathway_taps_for_white() {
        let def = brightclimb_pathway();
        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.mana.contains(&ManaSymbol::White));
            assert!(!mana_ability.mana.contains(&ManaSymbol::Black));
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_brightclimb_pathway_requires_tap() {
        let def = brightclimb_pathway();
        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_brightclimb_pathway_is_not_basic() {
        let def = brightclimb_pathway();
        use crate::types::Supertype;
        assert!(!def.card.has_supertype(Supertype::Basic));
    }

    #[test]
    fn test_brightclimb_pathway_no_subtypes() {
        let def = brightclimb_pathway();
        assert!(def.card.subtypes.is_empty());
        // Unlike basic Plains, this doesn't have the Plains subtype
    }

    #[test]
    fn test_brightclimb_pathway_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = brightclimb_pathway();
        let land_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&land_id));

        let obj = game.object(land_id).unwrap();
        assert!(obj.is_land());
        assert_eq!(obj.abilities.len(), 1);
        assert!(obj.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_brightclimb_pathway_untapped_on_entry() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = brightclimb_pathway();
        let land_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(!game.is_tapped(land_id), "Pathway lands enter untapped");
    }

    // ========================================
    // Grimclimb Pathway (Back Face) Tests
    // ========================================

    #[test]
    fn test_grimclimb_pathway_basic_properties() {
        let def = grimclimb_pathway();
        assert_eq!(def.name(), "Grimclimb Pathway");
        assert!(def.card.is_land());
        assert!(!def.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_grimclimb_pathway_is_colorless() {
        let def = grimclimb_pathway();
        // Lands without mana cost and no color indicator are colorless
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_grimclimb_pathway_color_identity() {
        let def = grimclimb_pathway();
        use crate::color::Color;
        // Color identity includes colors in rules text ({B})
        assert!(def.card.color_identity().contains(Color::Black));
        assert!(!def.card.color_identity().contains(Color::White));
    }

    #[test]
    fn test_grimclimb_pathway_has_mana_ability() {
        let def = grimclimb_pathway();
        assert_eq!(def.abilities.len(), 1);
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_grimclimb_pathway_taps_for_black() {
        let def = grimclimb_pathway();
        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.mana.contains(&ManaSymbol::Black));
            assert!(!mana_ability.mana.contains(&ManaSymbol::White));
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_grimclimb_pathway_requires_tap() {
        let def = grimclimb_pathway();
        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_grimclimb_pathway_is_not_basic() {
        let def = grimclimb_pathway();
        use crate::types::Supertype;
        assert!(!def.card.has_supertype(Supertype::Basic));
    }

    #[test]
    fn test_grimclimb_pathway_no_subtypes() {
        let def = grimclimb_pathway();
        assert!(def.card.subtypes.is_empty());
    }

    #[test]
    fn test_grimclimb_pathway_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = grimclimb_pathway();
        let land_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&land_id));

        let obj = game.object(land_id).unwrap();
        assert!(obj.is_land());
        assert_eq!(obj.abilities.len(), 1);
        assert!(obj.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_grimclimb_pathway_untapped_on_entry() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = grimclimb_pathway();
        let land_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(!game.is_tapped(land_id), "Pathway lands enter untapped");
    }

    // ========================================
    // Comparison Tests
    // ========================================

    #[test]
    fn test_brightclimb_and_grimclimb_are_different() {
        let front = brightclimb_pathway();
        let back = grimclimb_pathway();

        assert_ne!(front.name(), back.name());

        // Front taps for white, back taps for black
        if let (AbilityKind::Mana(front_mana), AbilityKind::Mana(back_mana)) =
            (&front.abilities[0].kind, &back.abilities[0].kind)
        {
            assert!(front_mana.mana.contains(&ManaSymbol::White));
            assert!(!front_mana.mana.contains(&ManaSymbol::Black));

            assert!(back_mana.mana.contains(&ManaSymbol::Black));
            assert!(!back_mana.mana.contains(&ManaSymbol::White));
        }
    }

    #[test]
    fn test_both_faces_are_lands() {
        let front = brightclimb_pathway();
        let back = grimclimb_pathway();

        assert!(front.card.is_land());
        assert!(back.card.is_land());
    }

    #[test]
    fn test_mdfc_combined_color_identity() {
        // In Commander, MDFCs have combined color identity from both faces
        // We can compute this by taking the union
        use crate::color::Color;

        let front = brightclimb_pathway();
        let back = grimclimb_pathway();

        let front_identity = front.card.color_identity();
        let back_identity = back.card.color_identity();
        let combined = front_identity.union(back_identity);

        assert!(
            combined.contains(Color::White),
            "Combined identity should have white"
        );
        assert!(
            combined.contains(Color::Black),
            "Combined identity should have black"
        );
    }

    // ========================================
    // Rules Interaction Tests
    // ========================================

    #[test]
    fn test_pathway_not_affected_by_land_subtypes() {
        // Pathways don't have land subtypes, so effects like "destroy all Swamps"
        // wouldn't affect Grimclimb Pathway even though it produces black mana
        let back = grimclimb_pathway();
        use crate::types::Subtype;

        assert!(!back.card.has_subtype(Subtype::Swamp));
        assert!(!back.card.has_subtype(Subtype::Plains));
    }

    #[test]
    fn test_pathway_cannot_be_fetched() {
        // Fetchlands search for lands with specific subtypes (Swamp, Plains, etc.)
        // Pathways don't have these subtypes, so they can't be fetched
        let front = brightclimb_pathway();
        let back = grimclimb_pathway();

        // Neither face has any land subtypes
        assert!(front.card.subtypes.is_empty());
        assert!(back.card.subtypes.is_empty());
    }

    #[test]
    fn test_pathway_not_affected_by_blood_moon() {
        // Blood Moon turns nonbasic lands with land subtypes into Mountains
        // Since Pathways have no land subtypes, Blood Moon has no effect
        // (they remain lands that tap for their color)
        let front = brightclimb_pathway();

        // Not basic
        use crate::types::Supertype;
        assert!(!front.card.has_supertype(Supertype::Basic));

        // No land subtypes to lose
        assert!(front.card.subtypes.is_empty());
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_brightclimb_pathway_play() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Brightclimb Pathway
            ],
            ReplayTestConfig::new().p1_hand(vec!["Brightclimb Pathway"]),
        );

        assert!(
            game.battlefield_has("Brightclimb Pathway"),
            "Brightclimb Pathway should be on battlefield after playing"
        );
    }

    #[test]
    fn test_replay_grimclimb_pathway_play() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Grimclimb Pathway
            ],
            ReplayTestConfig::new().p1_hand(vec!["Grimclimb Pathway"]),
        );

        assert!(
            game.battlefield_has("Grimclimb Pathway"),
            "Grimclimb Pathway should be on battlefield after playing"
        );
    }
}
