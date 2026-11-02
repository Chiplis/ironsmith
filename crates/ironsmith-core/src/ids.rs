use crate::tag::TagKeyWalk;

use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

/// Global counter for auto-incrementing player IDs.
static PLAYER_ID_COUNTER: AtomicU8 = AtomicU8::new(0);
/// Global counter for auto-incrementing object IDs (starts at 1, 0 is reserved).
static OBJECT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
/// Global counter for auto-incrementing card definition IDs (starts at 1, 0 is reserved).
static CARD_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Snapshot of global ID counters so deterministic replays can restore identity space.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, TagKeyWalk)]
pub struct IdCountersSnapshot {
    pub player: u8,
    pub object: u64,
    pub card: u32,
}

/// Player identifier, index-based for efficiency.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TagKeyWalk)]
pub struct PlayerId(pub u8);

/// Unique object identifier, monotonically increasing.
/// Never reused - when an object changes zones, it gets a new ID per MTG rule 400.7.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TagKeyWalk)]
pub struct ObjectId(pub u64);

/// Stable object instance identifier used across zone changes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, TagKeyWalk)]
pub struct StableId(pub ObjectId);

/// Card definition identifier, references static card data.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, TagKeyWalk)]
pub struct CardId(pub u32);

impl PlayerId {
    /// Create a new player ID with auto-incrementing counter.
    pub fn new() -> Self {
        Self(PLAYER_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Create a player ID from a specific index (for when you need explicit control).
    pub const fn from_index(index: u8) -> Self {
        Self(index)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl Default for PlayerId {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectId {
    /// Create a new object ID with auto-incrementing counter.
    pub fn new() -> Self {
        Self(OBJECT_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Create an object ID from a specific value (for when you need explicit control).
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl StableId {
    /// Create a stable ID from an object ID.
    pub fn from_object_id(id: ObjectId) -> Self {
        Self(id)
    }

    /// Create a stable ID from raw object ID value.
    pub fn from_raw(id: u64) -> Self {
        Self(ObjectId::from_raw(id))
    }

    /// Access the inner object ID.
    pub fn object_id(self) -> ObjectId {
        self.0
    }
}

impl From<ObjectId> for StableId {
    fn from(value: ObjectId) -> Self {
        Self(value)
    }
}

impl From<StableId> for ObjectId {
    fn from(value: StableId) -> Self {
        value.0
    }
}

fn allocate_card_definition_id(counter: &AtomicU32) -> Result<u32, &'static str> {
    counter.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |next| {
        (next != 0).then(|| next.checked_add(1)).flatten()
    }).map_err(|_| "card definition ID namespace exhausted")
}

fn reserve_card_definition_ids(counter: &AtomicU32, last: u32) -> Result<(), &'static str> {
    let next = last.checked_add(1).filter(|next| *next < u32::MAX)
        .ok_or("explicit card definition IDs leave no generated namespace")?;
    counter.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
        (current != 0 && current < u32::MAX).then_some(current.max(next))
    }).map(|_| ()).map_err(|_| "card definition ID namespace exhausted")
}

impl CardId {
    /// Create a new card ID with auto-incrementing counter.
    pub fn new() -> Self {
        Self(allocate_card_definition_id(&CARD_ID_COUNTER).expect("card definition ID namespace exhausted"))
    }

    /// Reserve an explicit compiler input namespace before allocating dependencies.
    /// Imported artifact IDs remain lexical and must not use this reservation.
    pub fn reserve_generated_namespace_through(last: Self) -> Result<(), &'static str> {
        reserve_card_definition_ids(&CARD_ID_COUNTER, last.0)
    }

    /// Create a card ID from a specific value (for when you need explicit control).
    pub fn from_raw(id: u32) -> Self {
        Self(id)
    }
}

/// Capture current global ID counters.
pub fn snapshot_id_counters() -> IdCountersSnapshot {
    IdCountersSnapshot {
        player: PLAYER_ID_COUNTER.load(Ordering::SeqCst),
        object: OBJECT_ID_COUNTER.load(Ordering::SeqCst),
        card: CARD_ID_COUNTER.load(Ordering::SeqCst),
    }
}

/// Restore global ID counters from a snapshot.
pub fn restore_id_counters(snapshot: IdCountersSnapshot) {
    PLAYER_ID_COUNTER.store(snapshot.player, Ordering::SeqCst);
    OBJECT_ID_COUNTER.store(snapshot.object, Ordering::SeqCst);
    CARD_ID_COUNTER.store(snapshot.card, Ordering::SeqCst);
}

/// Reset runtime player/object counters while preserving card definition identity space.
///
/// This is used when starting a fresh match so gameplay object IDs are stable across
/// deterministic replays and synchronized peers, without invalidating any card
/// definitions that have already been loaded into the registry.
pub fn reset_runtime_id_counters() {
    PLAYER_ID_COUNTER.store(0, Ordering::SeqCst);
    OBJECT_ID_COUNTER.store(1, Ordering::SeqCst);
}

/// Reset all ID counters to their initial state (for testing).
/// This should only be used in tests to ensure deterministic behavior.
#[cfg(test)]
pub fn reset_id_counters() {
    PLAYER_ID_COUNTER.store(0, Ordering::SeqCst);
    OBJECT_ID_COUNTER.store(1, Ordering::SeqCst);
    CARD_ID_COUNTER.store(1, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_id_auto_increment() {
        reset_id_counters();
        let p1 = PlayerId::new();
        let p2 = PlayerId::new();
        assert_ne!(p1, p2);
        assert_eq!(p1.index(), 0);
        assert_eq!(p2.index(), 1);
    }

    #[test]
    fn test_player_id_from_index() {
        let p1 = PlayerId::from_index(5);
        let p2 = PlayerId::from_index(10);
        assert_eq!(p1.index(), 5);
        assert_eq!(p2.index(), 10);
    }

    #[test]
    fn test_object_id_auto_increment() {
        // Object IDs auto-increment, just verify they're different
        let o1 = ObjectId::new();
        let o2 = ObjectId::new();
        assert_ne!(o1, o2);
    }

    #[test]
    fn test_object_id_from_raw() {
        let o1 = ObjectId::from_raw(100);
        let o2 = ObjectId::from_raw(200);
        assert_ne!(o1, o2);
        assert_eq!(o1.0, 100);
        assert_eq!(o2.0, 200);
    }

    #[test]
    fn test_card_id_auto_increment() {
        // Card IDs auto-increment, just verify they're different
        let c1 = CardId::new();
        let c2 = CardId::new();
        assert_ne!(c1, c2);
    }

    #[test]
    fn test_card_id_from_raw() {
        let c1 = CardId::from_raw(100);
        let c2 = CardId::from_raw(200);
        assert_ne!(c1, c2);
        assert_eq!(c1.0, 100);
        assert_eq!(c2.0, 200);
    }
}

#[cfg(test)]
mod generated_card_namespace_tests {
    use super::*;

    #[test]
    fn explicit_source_and_other_face_are_reserved_without_rewinding() {
        let counter = AtomicU32::new(1);
        reserve_card_definition_ids(&counter, 2).unwrap();
        assert_eq!(allocate_card_definition_id(&counter), Ok(3));
        reserve_card_definition_ids(&counter, 1).unwrap();
        assert_eq!(allocate_card_definition_id(&counter), Ok(4));
        // A new runtime match does not reset this definition namespace.
        assert_eq!(allocate_card_definition_id(&counter), Ok(5));
        assert_eq!(CardId::from_raw(2).0, 2);
    }

    #[test]
    fn exhausted_reservation_and_allocation_fail_without_wrapping() {
        let counter = AtomicU32::new(17);
        assert!(reserve_card_definition_ids(&counter, u32::MAX).is_err());
        assert!(reserve_card_definition_ids(&counter, u32::MAX - 1).is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 17);
        let exhausted = AtomicU32::new(u32::MAX);
        assert!(allocate_card_definition_id(&exhausted).is_err());
        assert_eq!(exhausted.load(Ordering::SeqCst), u32::MAX);
    }
}
