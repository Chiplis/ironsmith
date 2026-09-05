//! Small copyable owned-on-decode values used by the serialized card schema.
//!
//! The runtime historically represented user-authored names and a few compact
//! numeric sets as `&'static` values. Artifact decoding must be lifetime
//! independent, so decoded values are interned for the process lifetime while
//! preserving the cheap `Copy` semantics used throughout the engine.

use crate::tag::TagKeyWalk;

use std::fmt;
use std::ops::Deref;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub struct InternedStr(&'static str);

impl InternedStr {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl Deref for InternedStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl AsRef<str> for InternedStr {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl From<&'static str> for InternedStr {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for InternedStr {
    fn from(value: String) -> Self {
        Self(Box::leak(value.into_boxed_str()))
    }
}

impl fmt::Debug for InternedStr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Display for InternedStr {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for InternedStr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for InternedStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self(Box::leak(value.into_boxed_str())))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub struct InternedI32Slice(&'static [i32]);

impl InternedI32Slice {
    pub const fn new(values: &'static [i32]) -> Self {
        Self(values)
    }

    pub const fn as_slice(self) -> &'static [i32] {
        self.0
    }
}

impl Deref for InternedI32Slice {
    type Target = [i32];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl From<&'static [i32]> for InternedI32Slice {
    fn from(values: &'static [i32]) -> Self {
        Self::new(values)
    }
}

impl fmt::Debug for InternedI32Slice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for InternedI32Slice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for InternedI32Slice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values = <Vec<i32> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self(Box::leak(values.into_boxed_slice())))
    }
}
