//! Combat-related events.

mod creature_attacked;
mod creature_became_blocked;
mod creature_blocked;

pub use creature_attacked::CreatureAttackedEvent;
pub use creature_became_blocked::CreatureBecameBlockedEvent;
pub use creature_blocked::CreatureBlockedEvent;

// Re-export attack target from triggers module for now
pub use crate::triggers::event::AttackEventTarget;
