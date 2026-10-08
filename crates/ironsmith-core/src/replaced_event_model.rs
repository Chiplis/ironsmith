//! The event a generic "instead" replacement watches (CR 614.1a). The
//! replacement program runs in place of that event, with the replaced event
//! as its context: "that much" / "that many" read the event's amount and
//! "that player" names the affected player.

use crate::tag::TagKeyWalk;
use crate::{ObjectFilter, PlayerFilter};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum ReplacedEventSpec {
    /// "If damage would be dealt to you", "If a Zombie you control would deal
    /// combat damage to a player": damage to a matching player, optionally from
    /// a matching source and optionally combat damage only.
    DamageToPlayer {
        player: PlayerFilter,
        source_filter: Option<ObjectFilter>,
        combat_only: bool,
    },
    /// "If damage would be dealt to this creature": damage to a matching
    /// permanent, optionally from a matching source and combat only.
    DamageToObject {
        target: ObjectFilter,
        source_filter: Option<ObjectFilter>,
        combat_only: bool,
    },
    /// "If an opponent would gain life": a matching player's life gain.
    LifeGain { player: PlayerFilter },
}
