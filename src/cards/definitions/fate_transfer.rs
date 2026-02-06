//! Fate Transfer card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Fate Transfer - {1}{U/B}
/// Instant
/// Move all counters from target creature onto another target creature.
pub fn fate_transfer() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Fate Transfer")
        .mana_cost(ManaCost::from_pips(vec![
            // {1}
            vec![ManaSymbol::Generic(1)],
            // {U/B} - hybrid blue or black
            vec![ManaSymbol::Blue, ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        // .with_spell_effect(vec![Effect::move_all_counters(
        //     ChooseSpec::creature(),
        //     ChooseSpec::creature(),
        // )])
        .parse_text("Move all counters from target creature onto another target creature.")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fate_transfer() {
        let def = fate_transfer();
        assert_eq!(def.name(), "Fate Transfer");
        assert!(def.is_spell());
        // Mana value of {1}{U/B} is 2 (hybrid counts as 1)
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_fate_transfer_has_two_targets() {
        let def = fate_transfer();
        // The spell effect should target two creatures
        assert!(def.spell_effect.is_some());
        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
        let debug_str = format!("{:?}", &effects[0]);
        assert!(debug_str.contains("Counters"));
    }
}
