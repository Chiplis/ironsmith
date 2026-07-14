//! Tagging primitives for cross-effect composition.
//!
//! Tags are dynamic keys used to pass references (objects, players, counts)
//! between effects during the same spell/ability resolution.

use std::borrow::Borrow;
use std::fmt;

/// Runtime tag for cards linked as "exiled with this source object".
pub const SOURCE_EXILED_TAG: &str = "__source_exiled__";

/// One source snapshot per mana unit spent to cast the current spell.
pub const MANA_SOURCES_SPENT_TO_CAST_TAG: &str = "__mana_sources_spent_to_cast__";

/// Runtime tag for the creature sacrificed to an exploit action.
pub const EXPLOITED_TAG: &str = "exploited";

/// Runtime tag for the object whose exploit action sacrificed another object.
pub const EXPLOITER_TAG: &str = "exploiter";

/// Runtime tag for cards seen by a surveil action this turn.
pub const SURVEILLED_THIS_TURN_TAG: &str = "__surveilled_this_turn__";

/// Dynamic tag key used by the tagging system.
///
/// Using an owned key instead of `&'static str` enables tags built at runtime
/// while keeping convenient string-based APIs.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TagKey(String);

impl TagKey {
    /// Create a new tag key from any string-like value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the tag key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for TagKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for TagKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TagKey").field(&self.0).finish()
    }
}

impl fmt::Display for TagKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Borrow<str> for TagKey {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for TagKey {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl From<String> for TagKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&String> for TagKey {
    fn from(value: &String) -> Self {
        Self::new(value.clone())
    }
}
