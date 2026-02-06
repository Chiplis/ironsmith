use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

/// Global counter for auto-incrementing player IDs.
static PLAYER_ID_COUNTER: AtomicU8 = AtomicU8::new(0);
/// Global counter for auto-incrementing object IDs (starts at 1, 0 is reserved).
static OBJECT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
/// Global counter for auto-incrementing card definition IDs (starts at 1, 0 is reserved).
static CARD_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Player identifier, index-based for efficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

/// Unique object identifier, monotonically increasing.
/// Never reused - when an object changes zones, it gets a new ID per MTG rule 400.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u64);

/// Stable object instance identifier used across zone changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StableId(pub ObjectId);

/// Card definition identifier, references static card data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CardId(pub u32);

impl PlayerId {
    /// Create a new player ID with auto-incrementing counter.
    pub fn new() -> Self {
        Self(PLAYER_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Create a player ID from a specific index (for when you need explicit control).
    pub fn from_index(index: u8) -> Self {
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

impl CardId {
    /// Create a new card ID with auto-incrementing counter.
    pub fn new() -> Self {
        Self(CARD_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Create a card ID from a specific value (for when you need explicit control).
    pub fn from_raw(id: u32) -> Self {
        Self(id)
    }
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
