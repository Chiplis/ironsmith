/// Perform the spell form of ascend.
///
/// When this effect resolves, its controller gets the city's blessing if they
/// control ten or more permanents. Permanent cards use the corresponding
/// `StaticAbilityId::Ascend` marker instead.
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub struct AscendEffect;

impl AscendEffect {
    pub const fn new() -> Self {
        Self
    }
}
