use crate::{ChooseSpec, ObjectFilter, PlayerFilter};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "attachment filters preserve the shared object-filter value model"
)]
pub enum AuraAttachmentFilter {
    Object(ObjectFilter),
    Player(PlayerFilter),
}

impl From<ObjectFilter> for AuraAttachmentFilter {
    fn from(value: ObjectFilter) -> Self {
        Self::Object(value)
    }
}

impl From<PlayerFilter> for AuraAttachmentFilter {
    fn from(value: PlayerFilter) -> Self {
        Self::Player(value)
    }
}

impl AuraAttachmentFilter {
    pub fn target_spec(&self) -> ChooseSpec {
        match self {
            Self::Object(filter) => ChooseSpec::target(ChooseSpec::Object(filter.clone())),
            Self::Player(filter) => ChooseSpec::target(ChooseSpec::Player(filter.clone())),
        }
    }
}
