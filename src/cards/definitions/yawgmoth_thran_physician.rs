//! Yawgmoth, Thran Physician card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Yawgmoth, Thran Physician - {2}{B}{B}
/// Legendary Creature — Human Cleric (2/4)
/// Protection from Humans
/// Pay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature
/// and draw a card.
/// {B}{B}, Discard a card: Proliferate.
///
/// This card is essential for various combo decks involving undying creatures.
pub fn yawgmoth_thran_physician() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Yawgmoth, Thran Physician")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(
            "Protection from Humans\n\
             Pay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.\n\
             {B}{B}, Discard a card: Proliferate."
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{AbilityKind, ProtectionFrom};

    #[test]
    fn test_yawgmoth_basic_properties() {
        let def = yawgmoth_thran_physician();
        assert_eq!(def.name(), "Yawgmoth, Thran Physician");

        // Should be legendary creature
        assert!(def.card.supertypes.contains(&Supertype::Legendary));
        assert!(def.card.card_types.contains(&CardType::Creature));

        // Should be Human Cleric
        assert!(def.card.subtypes.contains(&Subtype::Human));
        assert!(def.card.subtypes.contains(&Subtype::Cleric));

        // Should have 2/4 P/T
        assert_eq!(
            def.card
                .power_toughness
                .as_ref()
                .unwrap()
                .power
                .base_value(),
            2
        );
        assert_eq!(
            def.card
                .power_toughness
                .as_ref()
                .unwrap()
                .toughness
                .base_value(),
            4
        );

        // Mana cost should be {2}{B}{B} = 4 mana value
        assert_eq!(def.card.mana_value(), 4);
    }

    #[test]
    fn test_yawgmoth_has_protection_from_humans() {
        let def = yawgmoth_thran_physician();

        let protection = def.abilities.iter().find_map(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.protection_from()
            } else {
                None
            }
        });
        assert!(protection.is_some(), "Should have protection ability");

        // Verify it's specifically protection from Humans
        if let Some(ProtectionFrom::Permanents(filter)) = protection {
            assert!(
                filter.subtypes.contains(&Subtype::Human),
                "Protection should be from Humans"
            );
        } else {
            panic!("Expected protection from permanents");
        }
    }

    #[test]
    fn test_yawgmoth_sacrifice_ability_requires_another_creature() {
        let def = yawgmoth_thran_physician();

        // Find the sacrifice ability (has life cost in cost_effects and sacrifice in cost_effects)
        let sacrifice_ability = def
            .abilities
            .iter()
            .find(|a| {
                if let AbilityKind::Activated(act) = &a.kind {
                    // Check for life cost in cost_effects (not TotalCost)
                    act.life_cost_amount().is_some()
                } else {
                    false
                }
            })
            .expect("Should have sacrifice ability");

        if let AbilityKind::Activated(act) = &sacrifice_ability.kind {
            // Should have 3 cost effects: PayLife + ChooseObjectsEffect + SacrificeEffect
            assert_eq!(
                act.mana_cost.costs().len(),
                3,
                "Should have 3 cost effects (pay life + choose + sacrifice)"
            );

            // First effect should be LoseLifeEffect (pay_life uses LoseLifeEffect internally)
            let debug_str_0 = format!("{:?}", &act.mana_cost.costs()[0]);
            assert!(
                debug_str_0.contains("LoseLifeEffect"),
                "First cost effect should be pay life"
            );

            // Second effect should be ChooseObjectsEffect for another creature
            let debug_str_1 = format!("{:?}", &act.mana_cost.costs()[1]);
            assert!(
                debug_str_1.contains("ChooseObjectsEffect"),
                "Second cost effect should be choose"
            );
            assert!(
                debug_str_1.contains("other: true"),
                "Should require 'another' creature"
            );

            // Third effect should be SacrificeEffect
            let debug_str_2 = format!("{:?}", &act.mana_cost.costs()[2]);
            assert!(
                debug_str_2.contains("SacrificeEffect"),
                "Third cost effect should be sacrifice"
            );
        }
    }

    #[test]
    fn test_yawgmoth_sacrifice_ability_requires_life_payment() {
        let def = yawgmoth_thran_physician();

        // Find the sacrifice ability (has life cost in cost_effects)
        let sacrifice_ability = def
            .abilities
            .iter()
            .find(|a| {
                if let AbilityKind::Activated(act) = &a.kind {
                    // Check for life cost in cost_effects (not TotalCost)
                    act.life_cost_amount().is_some()
                } else {
                    false
                }
            })
            .expect("Should have sacrifice ability");

        if let AbilityKind::Activated(act) = &sacrifice_ability.kind {
            // Should have life payment cost in cost_effects
            assert_eq!(
                act.life_cost_amount(),
                Some(1),
                "Sacrifice ability should cost 1 life"
            );
        }
    }

    #[test]
    fn test_yawgmoth_sacrifice_ability_draws_card() {
        let def = yawgmoth_thran_physician();

        // Find the sacrifice ability (has life cost in cost_effects)
        let sacrifice_ability = def
            .abilities
            .iter()
            .find(|a| {
                if let AbilityKind::Activated(act) = &a.kind {
                    act.life_cost_amount().is_some()
                } else {
                    false
                }
            })
            .expect("Should have sacrifice ability");

        if let AbilityKind::Activated(act) = &sacrifice_ability.kind {
            // Should have draw card effect
            let has_draw = act
                .effects
                .iter()
                .any(|e| format!("{:?}", e).contains("DrawCardsEffect"));
            assert!(has_draw, "Sacrifice ability should draw a card");
        }
    }

    #[test]
    fn test_yawgmoth_sacrifice_ability_puts_counter() {
        let def = yawgmoth_thran_physician();

        // Find the sacrifice ability (has life cost in cost_effects)
        let sacrifice_ability = def
            .abilities
            .iter()
            .find(|a| {
                if let AbilityKind::Activated(act) = &a.kind {
                    act.life_cost_amount().is_some()
                } else {
                    false
                }
            })
            .expect("Should have sacrifice ability");

        if let AbilityKind::Activated(act) = &sacrifice_ability.kind {
            // Should put -1/-1 counter on up to one target creature
            let has_counter_effect = act
                .effects
                .iter()
                .any(|e| format!("{:?}", e).contains("PutCountersEffect"));
            assert!(has_counter_effect, "Should have put counters effect");
        }
    }

    #[test]
    fn test_yawgmoth_proliferate_ability_requires_bb_and_discard() {
        let def = yawgmoth_thran_physician();

        // Find the proliferate ability (has mana cost and discard in cost_effects, no life cost)
        let proliferate_ability = def
            .abilities
            .iter()
            .find(|a| {
                if let AbilityKind::Activated(act) = &a.kind {
                    // Has mana cost but no life cost (distinguishes from sacrifice ability)
                    act.mana_cost.mana_cost().is_some()
                        && !act
                            .mana_cost
                            .costs()
                            .iter()
                            .any(|c| c.life_amount().is_some())
                } else {
                    false
                }
            })
            .expect("Should have proliferate ability");

        if let AbilityKind::Activated(act) = &proliferate_ability.kind {
            // Should require {B}{B} mana
            let mana_cost = act.mana_cost.mana_cost().expect("Should have mana cost");
            assert_eq!(mana_cost.mana_value(), 2, "Should cost 2 mana");

            // Should have 2 costs: mana (BB) and discard effect
            assert_eq!(
                act.mana_cost.costs().len(),
                2,
                "Should have 2 costs: mana (BB) and discard"
            );

            // Check that one of the costs contains discard effect
            let debug_str = format!("{:?}", &act.mana_cost.costs());
            assert!(
                debug_str.contains("DiscardEffect"),
                "Costs should contain discard effect"
            );
        }
    }

    #[test]
    fn test_yawgmoth_has_two_activated_abilities() {
        let def = yawgmoth_thran_physician();

        let activated_count = def
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Activated(_)))
            .count();

        assert_eq!(
            activated_count, 2,
            "Should have exactly 2 activated abilities"
        );
    }
}
