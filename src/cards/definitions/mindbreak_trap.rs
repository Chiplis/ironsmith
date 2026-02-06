//! Card definition for Mindbreak Trap.

use crate::alternative_cast::{AlternativeCastingMethod, TrapCondition};
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};

/// Creates the Mindbreak Trap card definition.
///
/// Mindbreak Trap {2}{U}{U}
/// Instant - Trap
/// If an opponent cast three or more spells this turn, you may pay {0}
/// rather than pay this spell's mana cost.
/// Exile any number of target spells.
pub fn mindbreak_trap() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mindbreak Trap")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .subtypes(vec![Subtype::Trap])
        .alternative_cast(AlternativeCastingMethod::MindbreakTrap {
            name: "Mindbreak Trap's trap cost",
            cost: ManaCost::new(), // {0}
            condition: TrapCondition::OpponentCastSpells { count: 3 },
        })
        .with_spell_effect(vec![Effect::exile_any_number(ChooseSpec::spell())])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mindbreak_trap_basic() {
        let card = mindbreak_trap();
        assert_eq!(card.card.name, "Mindbreak Trap");
        assert!(card.card.card_types.contains(&CardType::Instant));
        assert!(card.card.subtypes.contains(&Subtype::Trap));
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 4);
    }

    #[test]
    fn test_mindbreak_trap_has_alternative_cast() {
        let card = mindbreak_trap();
        assert_eq!(card.alternative_casts.len(), 1);

        if let AlternativeCastingMethod::MindbreakTrap {
            cost, condition, ..
        } = &card.alternative_casts[0]
        {
            assert!(cost.is_empty()); // {0}
            assert_eq!(*condition, TrapCondition::OpponentCastSpells { count: 3 });
        } else {
            panic!("Expected Trap alternative cast");
        }
    }

    #[test]
    fn test_mindbreak_trap_has_effect() {
        let card = mindbreak_trap();
        let effects = card
            .spell_effect
            .as_ref()
            .expect("Should have spell effects");
        assert_eq!(effects.len(), 1);

        // The effect should be an exile effect
        let debug_str = format!("{:?}", &effects[0]);
        assert!(debug_str.contains("Exile"));
    }
}
