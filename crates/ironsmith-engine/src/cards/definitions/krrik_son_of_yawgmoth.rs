use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

pub fn krrik_son_of_yawgmoth() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "K'rrik, Son of Yawgmoth")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian, Subtype::Horror, Subtype::Minion])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Lifelink\nFor each {B} in a cost, you may pay 2 life rather than pay that mana.",
        )
        .expect("Card text should be supported")
}
