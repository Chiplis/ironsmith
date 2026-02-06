//! The Birth of Meletis card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// The Birth of Meletis - {1}{W}
/// Enchantment — Saga
/// (As this Saga enters and after your draw step, add a lore counter.
/// Sacrifice after III.)
/// I — Search your library for a basic Plains card, reveal it, put it into your hand,
///     then shuffle.
/// II — Create a 0/4 colorless Wall artifact creature token with defender.
/// III — You gain 2 life.
pub fn the_birth_of_meletis() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "The Birth of Meletis")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Saga])
        .saga(3)
        .parse_text(
            "I — Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle.\n\
             II — Create a 0/4 colorless Wall artifact creature token with defender.\n\
             III — You gain 2 life.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_the_birth_of_meletis() {
        let def = the_birth_of_meletis();
        assert_eq!(def.name(), "The Birth of Meletis");
        assert!(def.card.is_enchantment());
        assert!(def.card.subtypes.contains(&Subtype::Saga));
        assert_eq!(def.max_saga_chapter, Some(3));

        // Should have 3 chapter abilities (now using Trigger struct)
        let chapter_abilities: Vec<_> = def
            .abilities
            .iter()
            .filter(|a| {
                matches!(&a.kind, AbilityKind::Triggered(t)
                if t.trigger.display().contains("Chapter"))
            })
            .collect();
        assert_eq!(chapter_abilities.len(), 3);
    }

    #[test]
    fn test_chapter_triggers() {
        let def = the_birth_of_meletis();

        // Verify each chapter is present (now using Trigger struct)
        // The saga has 3 chapters, and we check that chapter triggers exist
        let chapter_triggers: Vec<_> = def
            .abilities
            .iter()
            .filter(|a| {
                matches!(&a.kind, AbilityKind::Triggered(t)
                if t.trigger.display().contains("Chapter"))
            })
            .collect();
        assert_eq!(chapter_triggers.len(), 3, "Should have 3 chapter triggers");
    }
}
