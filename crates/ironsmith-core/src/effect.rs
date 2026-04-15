//! Shared effect-domain metadata.
//!
//! These types describe effect identity and selection cardinality without
//! pulling in the runtime execution engine.

/// Identifier for an effect within an effect sequence.
///
/// Used to reference effects for conditional logic ("if you do" patterns).
/// Effects are labeled with `Effect::WithId` and referenced by `Effect::If`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(pub u32);

impl EffectId {
    /// Special ID used by ForEachControllerOfTaggedEffect to store the count
    /// of tagged objects for the current controller during iteration.
    pub const TAGGED_COUNT: Self = Self(u32::MAX);
}

/// Specifies how many objects/players to choose.
///
/// Used for effects like "Exile any number of target spells" (Mindbreak Trap)
/// or "Choose up to two target creatures".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceCount {
    /// Minimum number to choose (0 for "any number" or "up to").
    pub min: usize,
    /// Maximum number to choose. None means unlimited ("any number").
    pub max: Option<usize>,
    /// Whether this count came from a dynamic `X target ...` clause.
    pub dynamic_x: bool,
    /// Whether a dynamic X count is optional ("up to X") instead of exact.
    pub up_to_x: bool,
    /// Whether the chosen object(s) should be selected at random.
    pub random: bool,
}

/// Distinguishes exact, optional, and "all matching" search instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSelectionMode {
    /// "a card", "three cards", or other exact-count search phrasing.
    Exact,
    /// "up to N", "any number", or otherwise optional search phrasing.
    Optional,
    /// "all cards ..." search phrasing.
    AllMatching,
}

impl Default for ChoiceCount {
    fn default() -> Self {
        Self::exactly(1)
    }
}

impl ChoiceCount {
    /// Exactly N (the default for most effects).
    pub const fn exactly(n: usize) -> Self {
        Self {
            min: n,
            max: Some(n),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// Any number (0 or more, unlimited).
    pub const fn any_number() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// At least N (N or more, unlimited).
    pub const fn at_least(n: usize) -> Self {
        Self {
            min: n,
            max: None,
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// Up to N (0 to N).
    pub const fn up_to(n: usize) -> Self {
        Self {
            min: 0,
            max: Some(n),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// Dynamic X-target count (rendered as `X target ...`).
    pub const fn dynamic_x() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: true,
            up_to_x: false,
            random: false,
        }
    }

    /// Dynamic "up to X" count.
    pub const fn up_to_dynamic_x() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: true,
            up_to_x: true,
            random: false,
        }
    }

    /// Returns true if this is "any number" (min 0, no max).
    pub fn is_any_number(&self) -> bool {
        self.min == 0 && self.max.is_none() && !self.dynamic_x
    }

    /// Returns true if this is exactly 1.
    pub fn is_single(&self) -> bool {
        self.min == 1 && self.max == Some(1)
    }

    pub const fn is_dynamic_x(&self) -> bool {
        self.dynamic_x
    }

    pub const fn is_up_to_dynamic_x(&self) -> bool {
        self.dynamic_x && self.up_to_x
    }

    pub const fn is_random(&self) -> bool {
        self.random
    }

    pub fn at_random(mut self) -> Self {
        self.random = true;
        self
    }
}

impl From<usize> for ChoiceCount {
    fn from(value: usize) -> Self {
        ChoiceCount::exactly(value)
    }
}

impl From<i32> for ChoiceCount {
    fn from(value: i32) -> Self {
        if value <= 0 {
            ChoiceCount::exactly(0)
        } else {
            ChoiceCount::exactly(value as usize)
        }
    }
}
