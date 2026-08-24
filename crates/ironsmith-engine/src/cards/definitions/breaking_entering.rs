//! Card definitions for Breaking // Entering.

use super::CardDefinitionBuilder;
use crate::card::LinkedFaceLayout;
use crate::cards::CardDefinition;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

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
        .oracle_text("Target player mills eight cards.")
        .with_spell_effect(vec![Effect::mill_player(8, PlayerFilter::Any)])
        .build()
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
        .oracle_text(
            "Put target creature card from a graveyard onto the battlefield under your control.",
        )
        .with_spell_effect(vec![Effect::put_onto_battlefield(
            ChooseSpec::Target(Box::new(ChooseSpec::Object(
                ObjectFilter::creature().in_zone(Zone::Graveyard),
            ))),
            false,
            PlayerFilter::You,
        )])
        .build()
}
