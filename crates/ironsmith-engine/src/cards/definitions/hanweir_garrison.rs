//! Hanweir Garrison card definition.

use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Hanweir Garrison - {2}{R}
/// Creature — Human Soldier (2/3)
/// Whenever Hanweir Garrison attacks, create two 1/1 red Human creature tokens
/// that are tapped and attacking.
pub fn hanweir_garrison() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Hanweir Garrison")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "Whenever Hanweir Garrison attacks, create two 1/1 red Human creature tokens that \
             are tapped and attacking.",
        )
        .expect("Hanweir Garrison text should be supported")
}
