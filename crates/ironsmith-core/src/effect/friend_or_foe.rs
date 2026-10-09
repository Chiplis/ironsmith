//! "For each player, choose friend or foe." (Battlebond): as the spell
//! resolves its controller designates every player, themself included, as a
//! friend or a foe (a choice made on resolution, CR 608.2d). The two sets are
//! recorded as tagged player groups that later "Each friend ..." /
//! "Each foe ..." instructions iterate.
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct ChooseFriendsOrFoesEffect {
    pub friends_tag: crate::tag::TagKey,
    pub foes_tag: crate::tag::TagKey,
}

impl ChooseFriendsOrFoesEffect {
    pub fn new(
        friends_tag: impl Into<crate::tag::TagKey>,
        foes_tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            friends_tag: friends_tag.into(),
            foes_tag: foes_tag.into(),
        }
    }
}
