//! Card definition for Mycosynth Lattice.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Mycosynth Lattice {6}
/// Artifact
/// All permanents are artifacts in addition to their other types.
/// All cards that aren't on the battlefield, spells, and permanents are colorless.
/// Players may spend mana as though it were mana of any color.
pub fn mycosynth_lattice() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mycosynth Lattice")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "All permanents are artifacts in addition to their other types.\n\
All cards that aren't on the battlefield, spells, and permanents are colorless.\n\
Players may spend mana as though it were mana of any color.",
        )
        .expect("Card text should be supported")
}
