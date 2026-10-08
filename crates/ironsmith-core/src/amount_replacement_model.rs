//! Amount-modifying replacements (CR 614.1a, 616.1): "twice that many",
//! "that many plus one", "half that damage, rounded down", "deals 3 damage
//! instead". One payload modifies the proposed amount of one watched event
//! kind; the event still happens, with the new amount, and every other
//! replacement keeps applying in the affected player's chosen order.

use crate::tag::TagKeyWalk;
use crate::{KeywordActionKind, ObjectFilter, PlayerFilter};

/// The event whose proposed amount an [`AmountModifierSpec`] changes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum AmountEventSpec {
    /// "If a source would deal 4 or more damage to a permanent or player"
    /// (Divine Presence), "If an instant or sorcery source would deal 3 or
    /// more damage to you" (Forethought Amulet). A player recipient matches
    /// `player` and a permanent recipient matches `object`; an absent filter
    /// matches no recipient of that kind.
    Damage {
        source_filter: Option<ObjectFilter>,
        player: Option<PlayerFilter>,
        object: Option<ObjectFilter>,
        combat_only: bool,
        /// "4 or more damage": the proposed amount must be at least this.
        minimum: Option<u32>,
    },
    /// "If an opponent would mill one or more cards" (Bruvac), "If you would
    /// scry a number of cards" (Kenessos), "each time you surveil" (Enhanced
    /// Surveillance): the magnitude of a keyword action a matching player
    /// performs (CR 701.17, 701.22, 701.25).
    KeywordAction {
        action: KeywordActionKind,
        performer: PlayerFilter,
    },
}

/// How the proposed amount changes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum AmountModifierSpec {
    /// "twice that many", "triple that damage".
    Multiply(u32),
    /// "that many plus one", "an additional two cards".
    Add(u32),
    /// "deals 3 damage ... instead".
    SetTo(u32),
    /// "half that damage, rounded down" / "rounded up".
    Half { round_up: bool },
}
