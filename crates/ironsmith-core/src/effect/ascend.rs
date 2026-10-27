//! Grant a lasting player designation when a permanent-count condition holds.
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "DesignationWire"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub struct AscendEffect {
    pub minimum: u32,
    pub historic_only: bool,
    pub enduring_story: bool,
    pub descriptive: bool,
}

impl Default for AscendEffect { fn default() -> Self { Self::new() } }
impl AscendEffect {
    pub const fn new() -> Self {
        Self { minimum: 10, historic_only: false, enduring_story: false, descriptive: false }
    }
    pub const fn designation_if_permanent_count(minimum: u32, historic_only: bool, enduring_story: bool) -> Self {
        Self { minimum, historic_only, enduring_story, descriptive: true }
    }
}

// Older artifacts encoded the native unit effect as null.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum DesignationWire {
    Legacy(()),
    Count { minimum: u32, historic_only: bool, enduring_story: bool, descriptive: bool },
}
#[cfg(feature = "serde")]
impl From<DesignationWire> for AscendEffect {
    fn from(value: DesignationWire) -> Self {
        match value {
            DesignationWire::Legacy(()) => Self::new(),
            DesignationWire::Count { minimum, historic_only, enduring_story, descriptive } => Self { minimum, historic_only, enduring_story, descriptive },
        }
    }
}
