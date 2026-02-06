//! Emrakul, the Promised End card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Emrakul, the Promised End - Legendary Creature — Eldrazi
/// {13}
/// 13/13
/// This spell costs {1} less to cast for each card type among cards in your graveyard.
/// When you cast this spell, gain control of target opponent during that player's next turn.
/// After that turn, that player takes an extra turn.
/// Flying, trample, protection from instants.
pub fn emrakul_the_promised_end() -> CardDefinition {
    let text = "Mana cost: {13}\n\
Type: Legendary Creature — Eldrazi\n\
Power/Toughness: 13/13\n\
This spell costs {1} less to cast for each card type among cards in your graveyard.\n\
When you cast this spell, gain control of target opponent during that player's next turn. After that turn, that player takes an extra turn.\n\
Flying, trample, protection from instants.";

    CardDefinitionBuilder::new(CardId::new(), "Emrakul, the Promised End")
        .parse_text(text)
        .expect("Emrakul text should be supported")
}
