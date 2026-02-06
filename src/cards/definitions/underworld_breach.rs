//! Card definition for Underworld Breach.

use crate::ability::Ability;
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::effect::Effect;
use crate::grant::GrantSpec;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;
use crate::types::CardType;

/// Creates the Underworld Breach card definition.
///
/// Underworld Breach {1}{R}
/// Enchantment
/// Each nonland card in your graveyard has escape. The escape cost is equal to
/// the card's mana cost plus exile three other cards from your graveyard.
/// At the beginning of the end step, sacrifice Underworld Breach.
pub fn underworld_breach() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Underworld Breach")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        // Static ability: Grant escape to each nonland card in your graveyard (using unified grant system)
        .with_ability(Ability::static_ability(StaticAbility::grants(
            GrantSpec::escape_to_nonland(3),
        )))
        // Triggered ability: At the beginning of the end step, sacrifice this
        // Note: Uses Trigger::beginning_of_end_step(PlayerFilter::You) which triggers on your end step
        .with_trigger(
            Trigger::beginning_of_end_step(PlayerFilter::You),
            vec![Effect::sacrifice_source()],
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{AbilityKind, TriggeredAbility};
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::grant::Grantable;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_underworld_breach() {
        let card = underworld_breach();
        assert_eq!(card.card.name, "Underworld Breach");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 2); // 1R = 2
        assert!(card.card.card_types.contains(&CardType::Enchantment));

        // Should have two abilities
        assert_eq!(card.abilities.len(), 2);

        // First ability: Grants (unified grant system)
        let grant_ability = &card.abilities[0];
        if let AbilityKind::Static(s) = &grant_ability.kind {
            assert_eq!(s.id(), StaticAbilityId::Grants);
            // Check that grant_spec returns escape with exile_count 3
            if let Some(spec) = s.grant_spec() {
                match &spec.grantable {
                    Grantable::AlternativeCast(AlternativeCastingMethod::Escape {
                        exile_count,
                        ..
                    }) => {
                        assert_eq!(*exile_count, 3);
                    }
                    _ => panic!("Expected Escape alternative cast method"),
                }
                // Filter should exclude lands
                assert!(spec.filter.excluded_card_types.contains(&CardType::Land));
            } else {
                panic!("Expected grant_spec to return Some");
            }
        } else {
            panic!("Expected Grants static ability");
        }

        // Second ability: End step sacrifice trigger (now using Trigger struct)
        let sacrifice_trigger = &card.abilities[1];
        if let AbilityKind::Triggered(TriggeredAbility {
            trigger, effects, ..
        }) = &sacrifice_trigger.kind
        {
            assert!(
                trigger.display().contains("end step"),
                "Should trigger on end step"
            );
            assert_eq!(effects.len(), 1);
            // Check effect exists
            let debug_str = format!("{:?}", &effects[0]);
            assert!(!debug_str.is_empty());
        } else {
            panic!("Expected triggered ability for sacrifice");
        }
    }
}
