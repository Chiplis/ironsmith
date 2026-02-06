//! Card definition for Manascape Refractor.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Manascape Refractor {3}
/// Artifact
/// Manascape Refractor enters the battlefield tapped.
/// Manascape Refractor has all activated abilities of all lands on the battlefield.
/// You may spend mana as though it were mana of any color to pay the activation costs of
/// Manascape Refractor's abilities.
pub fn manascape_refractor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Manascape Refractor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Manascape Refractor enters the battlefield tapped.\n\
Manascape Refractor has all activated abilities of all lands on the battlefield.\n\
You may spend mana as though it were mana of any color to pay the activation costs of \
Manascape Refractor's abilities.",
        )
        .expect("Card text should be supported")
}
