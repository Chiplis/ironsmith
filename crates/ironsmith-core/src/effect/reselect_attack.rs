//! "You may reselect which player or permanent target attacking creature is
//! attacking." (Portal Mage): the attacking creature's controller chooses a
//! new player, planeswalker, or battle it could attack (CR 506.4, 508.1b).
//! The creature stays attacking and keeps any blocked status.
use crate::tag::TagKeyWalk;
use crate::target_model::ChooseSpec;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct ReselectAttackTargetEffect {
    /// The attacking creature or creatures whose attack is redirected.
    pub target: ChooseSpec,
    /// "reselect which player": only players may be chosen.
    pub players_only: bool,
}

impl ReselectAttackTargetEffect {
    pub fn new(target: ChooseSpec, players_only: bool) -> Self {
        Self {
            target,
            players_only,
        }
    }
}
