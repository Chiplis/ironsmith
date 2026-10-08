//! The event a generic "instead" replacement watches (CR 614.1a). The
//! replacement program runs in place of that event, with the replaced event
//! as its context: "that much" / "that many" read the event's amount and
//! "that player" names the affected player.

use crate::tag::TagKeyWalk;
use crate::{ObjectFilter, PlayerFilter, Zone};

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
    /// "If you would lose life": a matching player's life loss.
    LifeLoss { player: PlayerFilter },
    /// "If enchanted land would be destroyed": destruction of a matching
    /// permanent (CR 701.8).
    Destroy { target: ObjectFilter },
    /// "If this creature would die" (battlefield to graveyard, CR 700.4),
    /// "If this would be put into a graveyard from anywhere": a matching
    /// object's zone change. The program's "it" is the moving object.
    ZoneChange {
        object: ObjectFilter,
        from: Option<Zone>,
        to: Option<Zone>,
    },
}
