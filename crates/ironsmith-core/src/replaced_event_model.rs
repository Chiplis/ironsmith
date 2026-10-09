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
    /// "If an opponent would draw two or more cards" (Alms Collector): one
    /// whole draw instruction of at least `minimum` cards by a matching
    /// player. The instruction is proposed before its cards are drawn one at
    /// a time (CR 121.2), so the replacement replaces all of them.
    DrawInstruction { player: PlayerFilter, minimum: u32 },
    /// "If this creature would be destroyed, regenerate it." (Clergy of the
    /// Holy Nimbus): the source's own destruction is replaced by
    /// regeneration itself (CR 701.19a) — tap it, remove all damage from it,
    /// remove it from combat — every time, and "can't be regenerated" turns it
    /// off (CR 701.19c). The program is the engine's regeneration.
    SourceDestructionRegenerates,
    /// "If a permanent with a wind counter on it would untap during its
    /// controller's untap step" (Freyalise's Winds): a matching permanent's
    /// untap, optionally only the untap step's own untap (CR 502.3). The
    /// program's "it" is that permanent.
    Untap {
        object: ObjectFilter,
        during_controllers_untap_step: bool,
    },
}
