//! Card definition for Rex, Cyber-Hound.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Rex, Cyber-Hound {1}{W}{U}
/// Legendary Artifact Creature — Robot Dog
/// As long as you own a card exiled with a brain counter, Rex has all activated abilities of
/// each card exiled with a brain counter.
/// 2/2
pub fn rex_cyber_hound() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Rex, Cyber-Hound")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Robot, Subtype::Dog])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Whenever Rex, Cyber-Hound deals combat damage to a player, you get {E}{E}, then mill two cards.\n\
{E}{E}: Exile target creature card from a graveyard. Put a brain counter on it. Activate only as a sorcery.\n\
As long as you own a card exiled with a brain counter, Rex has all activated abilities of each card exiled with a brain counter.",
        )
        .expect("Card text should be supported")
}
