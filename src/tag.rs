//! Tagging primitives for cross-effect composition.
//!
//! Tags are dynamic keys used to pass references (objects, players, counts) between
//! effects during the same spell/ability resolution.

use std::borrow::Borrow;
use std::fmt;

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
