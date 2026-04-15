//! Card definitions for Breaking // Entering.

use super::CardDefinitionBuilder;
use crate::card::LinkedFaceLayout;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

const BREAKING_ID: u32 = 0x4252_4541;
const ENTERING_ID: u32 = 0x454E_5445;

pub fn breaking() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(BREAKING_ID), "Breaking")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .other_face(CardId::from_raw(ENTERING_ID))
        .other_face_name("Entering")
        .linked_face_layout(LinkedFaceLayout::Split)
        .has_fuse()
        .parse_text("Target player mills eight cards.")
        .expect("Breaking text should be supported")
}

pub fn entering() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(ENTERING_ID), "Entering")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .other_face(CardId::from_raw(BREAKING_ID))
        .other_face_name("Breaking")
        .linked_face_layout(LinkedFaceLayout::Split)
        .parse_text(
            "Put target creature card from a graveyard onto the battlefield under your control.",
        )
        .expect("Entering text should be supported")
}
