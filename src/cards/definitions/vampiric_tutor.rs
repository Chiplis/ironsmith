//! Vampiric Tutor card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

/// Vampiric Tutor
/// {B}
/// Instant
/// Search your library for a card, then shuffle and put that card on top. You lose 2 life.
pub fn vampiric_tutor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Vampiric Tutor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![
            Effect::search_library(
                ObjectFilter::default(),
                Zone::Library,
                PlayerFilter::You,
                false,
            ),
            Effect::lose_life(2),
        ])
        .oracle_text(
            "Search your library for a card, then shuffle and put that card on top. You lose 2 life.",
        )
        .build()
}
