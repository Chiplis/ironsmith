//! Lightning Greaves card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Lightning Greaves - {2}
/// Artifact — Equipment
/// Equipped creature has haste and shroud. (It can't be the target of spells or abilities.)
/// Equip {0}
pub fn lightning_greaves() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Lightning Greaves")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Equipped creature has haste and shroud. (It can't be the target of spells or abilities.)\n\
             Equip {0}",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ability::{Ability, ActivationTiming};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbilityId;
    use crate::target::ChooseSpec;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        subtypes: Vec<Subtype>,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_equipment(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = lightning_greaves();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    fn attach_equipment(game: &mut GameState, equipment_id: ObjectId, creature_id: ObjectId) {
        if let Some(equipment) = game.object_mut(equipment_id) {
            equipment.attached_to = Some(creature_id);
        }
        if let Some(creature) = game.object_mut(creature_id) {
            creature.attachments.push(equipment_id);
        }
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_lightning_greaves_basic_properties() {
        let def = lightning_greaves();
        assert_eq!(def.name(), "Lightning Greaves");
        assert!(def.card.is_artifact());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_has_equipment_subtype() {
        let def = lightning_greaves();
        assert!(def.card.subtypes.contains(&Subtype::Equipment));
    }

    #[test]
    fn test_is_colorless() {
        let def = lightning_greaves();
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_has_correct_number_of_abilities() {
        let def = lightning_greaves();
        // Should have 2 abilities: equipment grant + equip
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // Equipment Grant Ability Tests
    // ========================================

    #[test]
    fn test_has_equipment_grant_ability() {
        let def = lightning_greaves();

        let grant_ability = def.abilities.iter().find(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::EquipmentGrant
            } else {
                false
            }
        });
        assert!(
            grant_ability.is_some(),
            "Should have an equipment grant ability"
        );

        if let AbilityKind::Static(s) = &grant_ability.unwrap().kind {
            if let Some(abilities) = s.equipment_grant_abilities() {
                assert_eq!(abilities.len(), 2, "Should grant 2 abilities");
                assert!(
                    abilities.iter().any(|a| a.has_haste()),
                    "Should grant haste"
                );
                assert!(
                    abilities.iter().any(|a| a.has_shroud()),
                    "Should grant shroud"
                );
            } else {
                panic!("Expected equipment grant abilities");
            }
        } else {
            panic!("Expected EquipmentGrant ability");
        }
    }

    // ========================================
    // Equip Ability Structure Tests
    // ========================================

    #[test]
    fn test_has_equip_ability() {
        let def = lightning_greaves();

        let equip_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)));
        assert!(
            equip_ability.is_some(),
            "Should have an activated ability (equip)"
        );

        if let AbilityKind::Activated(activated) = &equip_ability.unwrap().kind {
            // Verify equip cost is {0} (free)
            assert!(
                activated.mana_cost.is_free(),
                "Equip should have no cost (equip 0)"
            );

            // Verify timing is sorcery speed
            assert_eq!(
                activated.timing,
                ActivationTiming::SorcerySpeed,
                "Equip is sorcery speed"
            );

            // Verify targets creature you control
            assert_eq!(activated.choices.len(), 1);
            let target_spec = match &activated.choices[0] {
                ChooseSpec::Target(inner) => inner.as_ref(),
                other => other,
            };
            if let ChooseSpec::Object(filter) = target_spec {
                assert!(filter.card_types.contains(&CardType::Creature));
                assert!(matches!(
                    filter.controller,
                    Some(crate::target::PlayerFilter::You)
                ));
            } else {
                panic!("Equip should target an object");
            }
        } else {
            panic!("Expected activated ability");
        }
    }

    // ========================================
    // Shroud Protection Tests
    // ========================================

    #[test]
    fn test_equipped_creature_has_shroud() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature and attach the greaves
        let creature_id = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, creature_id);

        // Get calculated characteristics through the continuous effect system
        let chars = game
            .calculated_characteristics(creature_id)
            .expect("Should calculate characteristics");

        // The creature should have shroud
        assert!(
            chars.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_shroud()
                } else {
                    false
                }
            }),
            "Equipped creature should have shroud"
        );
    }

    #[test]
    fn test_equipped_creature_has_haste() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature and attach the greaves
        let creature_id = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);
        let equipment_id = create_equipment(&mut game, alice);
        attach_equipment(&mut game, equipment_id, creature_id);

        // Get calculated characteristics through the continuous effect system
        let chars = game
            .calculated_characteristics(creature_id)
            .expect("Should calculate characteristics");

        // The creature should have haste
        assert!(
            chars.abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            }),
            "Equipped creature should have haste"
        );
    }

    // ========================================
    // Equipment Movement Tests
    // ========================================

    #[test]
    fn test_abilities_move_with_equipment() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Helper to check if abilities list contains haste
        fn has_haste(abilities: &[Ability]) -> bool {
            abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_haste()
                } else {
                    false
                }
            })
        }

        // Helper to check if abilities list contains shroud
        fn has_shroud(abilities: &[Ability]) -> bool {
            abilities.iter().any(|a| {
                if let AbilityKind::Static(s) = &a.kind {
                    s.has_shroud()
                } else {
                    false
                }
            })
        }

        // Create two creatures and the equipment
        let creature1_id = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);
        let creature2_id = create_creature(&mut game, "Knight", vec![Subtype::Knight], alice);
        let equipment_id = create_equipment(&mut game, alice);

        // Attach to first creature
        attach_equipment(&mut game, equipment_id, creature1_id);

        // Verify creature1 has haste and shroud
        {
            let chars = game
                .calculated_characteristics(creature1_id)
                .expect("Should calculate characteristics");
            assert!(has_haste(&chars.abilities));
            assert!(has_shroud(&chars.abilities));
        }

        // Move equipment to creature2
        {
            let equipment = game.object_mut(equipment_id).unwrap();
            equipment.attached_to = Some(creature2_id);
        }
        {
            let creature1 = game.object_mut(creature1_id).unwrap();
            creature1.attachments.retain(|&id| id != equipment_id);
        }
        {
            let creature2 = game.object_mut(creature2_id).unwrap();
            creature2.attachments.push(equipment_id);
        }

        // Verify creature1 no longer has haste and shroud
        {
            let chars = game
                .calculated_characteristics(creature1_id)
                .expect("Should calculate characteristics");
            assert!(!has_haste(&chars.abilities));
            assert!(!has_shroud(&chars.abilities));
        }

        // Verify creature2 now has haste and shroud
        {
            let chars = game
                .calculated_characteristics(creature2_id)
                .expect("Should calculate characteristics");
            assert!(has_haste(&chars.abilities));
            assert!(has_shroud(&chars.abilities));
        }
    }

    #[test]
    fn test_unequipped_equipment_grants_nothing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create equipment but don't attach it
        let _equipment_id = create_equipment(&mut game, alice);

        // Create a creature
        let creature_id = create_creature(&mut game, "Soldier", vec![Subtype::Soldier], alice);

        // Verify creature has no haste or shroud
        let chars = game
            .calculated_characteristics(creature_id)
            .expect("Should calculate characteristics");

        assert!(!chars.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_haste()
            } else {
                false
            }
        }));
        assert!(!chars.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_shroud()
            } else {
                false
            }
        }));
    }

    // ========================================
    // Equip Cost Tests
    // ========================================

    #[test]
    fn test_equip_zero_cost() {
        let def = lightning_greaves();

        let equip_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)));

        if let Some(ability) = equip_ability {
            if let AbilityKind::Activated(activated) = &ability.kind {
                // Equip {0} should have no mana cost
                assert!(
                    activated.mana_cost.mana_cost().is_none()
                        || activated.mana_cost.mana_cost().unwrap().mana_value() == 0,
                    "Equip cost should be 0"
                );
            }
        }
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_oracle_text_contains_abilities() {
        let def = lightning_greaves();
        assert!(def.card.oracle_text.contains("haste"));
        assert!(def.card.oracle_text.contains("shroud"));
        assert!(def.card.oracle_text.contains("Equip"));
    }

    // ========================================
    // Not a Mana Ability Test
    // ========================================

    #[test]
    fn test_equip_not_mana_ability() {
        let def = lightning_greaves();
        for ability in &def.abilities {
            assert!(!ability.is_mana_ability());
        }
    }

    // ========================================
    // Functional Zone Tests
    // ========================================

    #[test]
    fn test_abilities_only_functional_on_battlefield() {
        let def = lightning_greaves();
        for ability in &def.abilities {
            assert!(
                ability.functional_zones.contains(&Zone::Battlefield),
                "Abilities should be functional on battlefield"
            );
            assert_eq!(
                ability.functional_zones.len(),
                1,
                "Abilities should only be functional on battlefield"
            );
        }
    }

    // ========================================
    // Shroud Prevents Targeting Tests (Conceptual)
    // ========================================

    #[test]
    fn test_shroud_conceptual_behavior() {
        // Note: Full targeting prevention would be tested at the targeting
        // validation level. This test verifies the shroud ability is granted.
        let def = lightning_greaves();

        if let Some(ability) = def.abilities.iter().find(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::EquipmentGrant
            } else {
                false
            }
        }) {
            if let AbilityKind::Static(s) = &ability.kind {
                if let Some(abilities) = s.equipment_grant_abilities() {
                    // Verify shroud is one of the granted abilities
                    assert!(
                        abilities.iter().any(|a| a.has_shroud()),
                        "Equipment should grant shroud"
                    );
                }
            }
        }
    }

    // ========================================
    // Haste Allows Immediate Attack Test (Conceptual)
    // ========================================

    #[test]
    fn test_haste_conceptual_behavior() {
        // Note: Full summoning sickness check would be tested at the
        // combat/attack validation level. This test verifies haste is granted.
        let def = lightning_greaves();

        if let Some(ability) = def.abilities.iter().find(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::EquipmentGrant
            } else {
                false
            }
        }) {
            if let AbilityKind::Static(s) = &ability.kind {
                if let Some(abilities) = s.equipment_grant_abilities() {
                    // Verify haste is one of the granted abilities
                    assert!(
                        abilities.iter().any(|a| a.has_haste()),
                        "Equipment should grant haste"
                    );
                }
            }
        }
    }

    /// Tests casting Lightning Greaves.
    ///
    /// Lightning Greaves: {2} artifact - equipment
    /// Equipped creature has haste and shroud.
    /// Equip {0}
    #[test]
    fn test_replay_lightning_greaves_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap Sol Ring for mana (adds 2 colorless to pool)
                "1", // Cast Lightning Greaves (now we have mana in pool)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Lightning Greaves"])
                .p1_battlefield(vec!["Sol Ring"]),
        );

        // Lightning Greaves should be on the battlefield
        assert!(
            game.battlefield_has("Lightning Greaves"),
            "Lightning Greaves should be on battlefield after casting"
        );
    }
}
