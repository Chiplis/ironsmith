//! Card definition for Toph, the First Metalbender.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Toph, the First Metalbender {1}{R}{G}{W}
/// Legendary Creature — Human Warrior Ally
/// Nontoken artifacts you control are lands in addition to their other types.
/// 3/3
pub fn toph_the_first_metalbender() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Toph, the First Metalbender")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::White],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Warrior, Subtype::Ally])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Nontoken artifacts you control are lands in addition to their other types. \
(They don't gain the ability to {T} for mana.)\n\
At the beginning of your end step, earthbend 2. (Target land you control becomes a 0/0 \
creature with haste that's still a land. Put two +1/+1 counters on it. When it dies or \
is exiled, return it to the battlefield tapped.)",
        )
        .expect("Card text should be supported")
}
