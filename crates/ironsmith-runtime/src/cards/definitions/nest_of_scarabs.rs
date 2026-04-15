//! Nest of Scarabs card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Nest of Scarabs - {2}{B}
/// Enchantment
/// Whenever you put one or more -1/-1 counters on a creature, create that many
/// 1/1 black Insect creature tokens.
pub fn nest_of_scarabs() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Nest of Scarabs")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you put one or more -1/-1 counters on a creature, create that many 1/1 black Insect creature tokens.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_nest_of_scarabs_basic_properties() {
        let def = nest_of_scarabs();
        assert_eq!(def.name(), "Nest of Scarabs");
        assert!(def.card.is_enchantment());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_nest_of_scarabs_trigger_shape() {
        let def = nest_of_scarabs();
        let debug = format!("{:#?}", def.abilities);
        assert!(
            debug.contains("CounterPutOnTrigger")
                && debug.contains("source_controller: Some(")
                && debug.contains("MinusOneMinusOne")
                && debug.contains("count_mode: OneOrMore"),
            "expected counter trigger with controller and one-or-more mode, got {debug}"
        );
        assert!(
            debug.contains("CreateTokenEffect")
                && debug.contains("EventValue")
                && debug.contains("Amount"),
            "expected token creation from trigger amount, got {debug}"
        );

        let triggered_count = def
            .abilities
            .iter()
            .filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
            .count();
        assert_eq!(triggered_count, 1, "Nest should have one triggered ability");
    }
}
