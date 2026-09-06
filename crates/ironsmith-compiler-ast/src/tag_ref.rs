//! The AST's reference identity: a symbol, with the key it was minted under.
//!
//! Two `TagRef`s are equal when they name the same symbol. The key is the
//! symbol's rendering for the runtime, which correlates tagged objects by
//! name; `From<TagRef> for TagKey` is that rendering. `Debug` prints like a
//! `TagKey`, so diagnostics and fixtures read as before.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use ironsmith_core::tag::{TagKey, TagKeyWalk};

use crate::model::symbols::{Cardinality, ObjectDomain, ReferenceRole, SymbolId};

#[derive(Clone)]
pub struct TagRef {
    pub symbol: SymbolId,
    pub key: TagKey,
}

impl TagRef {
    /// Binds `key` in the active reference scope with the default role and
    /// returns the reference; use a `CompilerReferenceTag` where one exists.
    pub fn of(key: impl Into<TagKey>) -> Self {
        crate::reference_ledger::note_minted(
            key.into(),
            ReferenceRole::Affected,
            ObjectDomain::Object,
            Cardinality::Any,
        )
    }

    pub fn key(&self) -> &TagKey {
        &self.key
    }
}

impl PartialEq for TagRef {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
    }
}

impl Eq for TagRef {}

impl Hash for TagRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.symbol.hash(state);
    }
}

impl PartialOrd for TagRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TagRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.symbol.cmp(&other.symbol)
    }
}

impl fmt::Debug for TagRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TagKey").field(&self.key.as_str()).finish()
    }
}

impl fmt::Display for TagRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key.as_str())
    }
}

impl Deref for TagRef {
    type Target = TagKey;
    fn deref(&self) -> &TagKey {
        &self.key
    }
}

impl From<TagRef> for TagKey {
    fn from(tag: TagRef) -> TagKey {
        tag.key
    }
}

impl From<&TagRef> for TagKey {
    fn from(tag: &TagRef) -> TagKey {
        tag.key.clone()
    }
}

impl PartialEq<TagKey> for TagRef {
    fn eq(&self, other: &TagKey) -> bool {
        &self.key == other
    }
}

impl TagKeyWalk for TagRef {
    fn for_each_tag_key(&self, f: &mut dyn FnMut(&TagKey)) {
        f(&self.key);
    }
    fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut TagKey)) {
        f(&mut self.key);
    }
}
