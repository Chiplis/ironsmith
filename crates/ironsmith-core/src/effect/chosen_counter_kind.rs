//! "Choose a counter on a permanent you control. Put a counter of that kind
//! on target permanent you control if it doesn't have a counter of that kind
//! on it." (Aven Courier) / "Choose a kind of counter on a creature you
//! control. Put a counter of that kind on each other creature you control."
//! (Contractual Safeguard)
//!
//! The controller chooses one counter (an object and a kind of counter on
//! it) among the objects `kind_source` names, then puts one counter of that
//! kind on each recipient. "each other" excludes the object the counter was
//! chosen on; "if it doesn't have a counter of that kind on it" skips a
//! recipient that already has one when the counter would be put.

use crate::tag::TagKeyWalk;

use super::*;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct PutCounterOfKindChosenFromEffect {
    /// Objects whose counters may be chosen.
    pub kind_source: ObjectFilter,
    /// Recipients: a target declaration or every matching object.
    pub recipients: ChooseSpec,
    /// "each other …": not the object the counter was chosen on.
    pub exclude_kind_object: bool,
    /// "if it doesn't have a counter of that kind on it".
    pub only_if_absent: bool,
}

impl PutCounterOfKindChosenFromEffect {
    pub fn new(kind_source: ObjectFilter, recipients: ChooseSpec) -> Self {
        Self {
            kind_source,
            recipients,
            exclude_kind_object: false,
            only_if_absent: false,
        }
    }

    pub fn excluding_kind_object(mut self, exclude: bool) -> Self {
        self.exclude_kind_object = exclude;
        self
    }

    pub fn only_if_absent(mut self, only_if_absent: bool) -> Self {
        self.only_if_absent = only_if_absent;
        self
    }
}

/// "Choose a kind of counter at random that this creature doesn't have on
/// it from among flying, first strike, ..., and +1/+1. Put a counter of
/// that kind on this creature." (Crystalline Giant)
///
/// One kind is chosen uniformly at random among `kinds` (restricted to the
/// kinds the recipient has no counter of when `only_absent_kinds`), then one
/// counter of that kind is put on the recipient. With no eligible kind,
/// nothing happens.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct PutCounterOfRandomKindEffect {
    /// The fixed list of kinds the random choice is made from.
    pub kinds: Vec<CounterType>,
    /// The object that receives the counter.
    pub recipient: ChooseSpec,
    /// "that <recipient> doesn't have on it": kinds already on the
    /// recipient are not eligible.
    pub only_absent_kinds: bool,
}

impl PutCounterOfRandomKindEffect {
    pub fn new(kinds: Vec<CounterType>, recipient: ChooseSpec, only_absent_kinds: bool) -> Self {
        Self {
            kinds,
            recipient,
            only_absent_kinds,
        }
    }
}
