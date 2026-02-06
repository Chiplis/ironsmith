//! Bastion Protector card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Bastion Protector - {2}{W}
/// Creature — Human Soldier
/// 3/3
/// Commander creatures you control get +2/+2 and have indestructible.
pub fn bastion_protector() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Bastion Protector")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Commander creatures you control get +2/+2 and have indestructible.")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbility;
    use crate::static_abilities::StaticAbilityId;
    use crate::zone::Zone;

    #[test]
    fn test_bastion_protector_basic_properties() {
        let def = bastion_protector();

        // Check name
        assert_eq!(def.name(), "Bastion Protector");

        // Check it's a creature
        assert!(def.is_creature());

        // Check mana cost - {2}{W} = mana value 3
        assert_eq!(def.card.mana_value(), 3);

        // Check colors - should be white
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);

        // Check subtypes
        assert!(def.card.has_subtype(Subtype::Human));
        assert!(def.card.has_subtype(Subtype::Soldier));

        // Check power/toughness
        let pt = def.card.power_toughness.unwrap();
        assert_eq!(pt.power.base_value(), 3);
        assert_eq!(pt.toughness.base_value(), 3);
    }

    #[test]
    fn test_bastion_protector_has_anthem_ability() {
        let def = bastion_protector();

        // Should have 2 abilities: Anthem and GrantAbility
        assert_eq!(def.abilities.len(), 2);

        // Check that one is an Anthem
        let has_anthem = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Anthem
            } else {
                false
            }
        });
        assert!(has_anthem, "Should have an Anthem ability");
    }

    #[test]
    fn test_bastion_protector_has_grant_indestructible() {
        let def = bastion_protector();

        // Check that one ability is a GrantAbility
        let has_grant_ability = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::GrantAbility
            } else {
                false
            }
        });
        assert!(has_grant_ability, "Should have a GrantAbility ability");
    }

    #[test]
    fn test_bastion_protector_buffs_commander() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a legendary creature to be Alice's commander
        use crate::card::CardBuilder;
        let commander_card = CardBuilder::new(CardId::new(), "Test Commander")
            .card_types(vec![CardType::Creature])
            .supertypes(vec![crate::types::Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let commander_id = game.create_object_from_card(&commander_card, alice, Zone::Battlefield);

        // Set this creature as Alice's commander
        game.set_as_commander(commander_id, alice);

        // Create Bastion Protector
        let protector_def = bastion_protector();
        let _protector_id =
            game.create_object_from_definition(&protector_def, alice, Zone::Battlefield);

        // Check that the commander got +2/+2 (using calculated power which includes continuous effects)
        assert_eq!(
            game.calculated_power(commander_id),
            Some(6),
            "Commander should have 4+2=6 power"
        );
        assert_eq!(
            game.calculated_toughness(commander_id),
            Some(6),
            "Commander should have 4+2=6 toughness"
        );
    }

    #[test]
    fn test_bastion_protector_grants_indestructible_to_commander() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a legendary creature to be Alice's commander
        use crate::card::CardBuilder;
        let commander_card = CardBuilder::new(CardId::new(), "Test Commander")
            .card_types(vec![CardType::Creature])
            .supertypes(vec![crate::types::Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let commander_id = game.create_object_from_card(&commander_card, alice, Zone::Battlefield);

        // Set this creature as Alice's commander
        game.set_as_commander(commander_id, alice);

        // Create Bastion Protector
        let protector_def = bastion_protector();
        let _protector_id =
            game.create_object_from_definition(&protector_def, alice, Zone::Battlefield);

        // Apply continuous effects
        game.update_static_ability_effects();

        // Check that the commander has indestructible
        let _commander = game.object(commander_id).unwrap();
        assert!(
            game.object_has_ability(commander_id, &StaticAbility::indestructible()),
            "Commander should have indestructible"
        );
    }

    #[test]
    fn test_bastion_protector_does_not_buff_non_commanders() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a regular creature (not a commander)
        use crate::card::CardBuilder;
        let creature_card = CardBuilder::new(CardId::new(), "Regular Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        // Note: This creature is NOT set as a commander

        // Create Bastion Protector
        let protector_def = bastion_protector();
        let _protector_id =
            game.create_object_from_definition(&protector_def, alice, Zone::Battlefield);

        // Apply continuous effects
        game.update_static_ability_effects();

        // Check that the regular creature did NOT get +2/+2
        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature.power(),
            Some(2),
            "Non-commander should still have 2 power"
        );
        assert_eq!(
            creature.toughness(),
            Some(2),
            "Non-commander should still have 2 toughness"
        );

        // Check that the regular creature does NOT have indestructible
        assert!(
            !game.object_has_ability(creature_id, &StaticAbility::indestructible()),
            "Non-commander should not have indestructible"
        );
    }

    #[test]
    fn test_bastion_protector_does_not_buff_opponent_commanders() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Bob's commander
        use crate::card::CardBuilder;
        let commander_card = CardBuilder::new(CardId::new(), "Bob's Commander")
            .card_types(vec![CardType::Creature])
            .supertypes(vec![crate::types::Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let bob_commander_id =
            game.create_object_from_card(&commander_card, bob, Zone::Battlefield);

        // Set this creature as Bob's commander
        game.set_as_commander(bob_commander_id, bob);

        // Alice creates Bastion Protector
        let protector_def = bastion_protector();
        let _protector_id =
            game.create_object_from_definition(&protector_def, alice, Zone::Battlefield);

        // Apply continuous effects
        game.update_static_ability_effects();

        // Check that Bob's commander did NOT get +2/+2 (it's not controlled by Alice)
        let bob_commander = game.object(bob_commander_id).unwrap();
        assert_eq!(
            bob_commander.power(),
            Some(4),
            "Opponent's commander should still have 4 power"
        );
        assert_eq!(
            bob_commander.toughness(),
            Some(4),
            "Opponent's commander should still have 4 toughness"
        );

        // Check that Bob's commander does NOT have indestructible
        assert!(
            !game.object_has_ability(bob_commander_id, &StaticAbility::indestructible()),
            "Opponent's commander should not have indestructible from Alice's Bastion Protector"
        );
    }

    #[test]
    fn test_bastion_protector_buffs_stolen_commander() {
        // If Alice gains control of Bob's commander, Alice's Bastion Protector should buff it
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Bob's commander
        use crate::card::CardBuilder;
        let commander_card = CardBuilder::new(CardId::new(), "Bob's Commander")
            .card_types(vec![CardType::Creature])
            .supertypes(vec![crate::types::Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let bob_commander_id =
            game.create_object_from_card(&commander_card, bob, Zone::Battlefield);

        // Set this creature as Bob's commander (not Alice's!)
        game.set_as_commander(bob_commander_id, bob);

        // Alice gains control of Bob's commander (e.g., via Control Magic)
        if let Some(obj) = game.object_mut(bob_commander_id) {
            obj.controller = alice;
        }

        // Alice creates Bastion Protector
        let protector_def = bastion_protector();
        let _protector_id =
            game.create_object_from_definition(&protector_def, alice, Zone::Battlefield);

        // Check that Bob's commander (now controlled by Alice) gets +2/+2
        assert_eq!(
            game.calculated_power(bob_commander_id),
            Some(6),
            "Stolen commander should have 4+2=6 power"
        );
        assert_eq!(
            game.calculated_toughness(bob_commander_id),
            Some(6),
            "Stolen commander should have 4+2=6 toughness"
        );

        // Check that Bob's commander (now controlled by Alice) has indestructible
        assert!(
            game.object_has_ability(bob_commander_id, &StaticAbility::indestructible()),
            "Stolen commander should have indestructible"
        );
    }
}
