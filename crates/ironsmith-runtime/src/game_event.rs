//! Game event types shared across the event system.
//!
//! This module provides core types used by the event system:
//! - `DamageTarget` - Identifies the target of damage (player or object)
//! - `ObjectSnapshot` - A snapshot of an object's state before a zone change

use crate::ids::{ObjectId, PlayerId};

/// The target of damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageTarget {
    /// Damage to a player
    Player(PlayerId),
    /// Damage to a creature or planeswalker
    Object(ObjectId),
}

/// A snapshot of an object's state before a zone change.
/// Used for triggered abilities that check the object's characteristics
/// at the time it left its zone (e.g., "when a creature with power 4 or greater dies").
///
/// This is an alias for the unified ObjectSnapshot from the snapshot module.
/// The unified type provides all the same functionality plus additional helper methods.
pub type ObjectSnapshot = crate::snapshot::ObjectSnapshot;
