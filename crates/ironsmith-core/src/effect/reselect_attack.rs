//! "You may reselect which player or permanent target attacking creature is
//! attacking." (Portal Mage): the attacking creature's controller chooses a
//! new player, planeswalker, or battle it could attack (CR 506.4, 508.1b).
//! The creature stays attacking and keeps any blocked status.
//!
//! With `attacked_player`, the new attack is fixed instead of chosen: "Those
//! creatures are now attacking that player." (Portal Manipulator). A creature
//! whose controller could not attack that player keeps its attack.
use crate::tag::TagKeyWalk;
use crate::target_model::ChooseSpec;
use crate::PlayerFilter;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct ReselectAttackTargetEffect {
    /// The attacking creature or creatures whose attack is redirected.
    pub target: ChooseSpec,
    /// "reselect which player": only players may be chosen.
    pub players_only: bool,
    /// "are now attacking that player": the player each creature now
    /// attacks, with no choice (CR 506.4).
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub attacked_player: Option<PlayerFilter>,
}

impl ReselectAttackTargetEffect {
    pub fn new(target: ChooseSpec, players_only: bool) -> Self {
        Self {
            target,
            players_only,
            attacked_player: None,
        }
    }

    /// Redirect each creature's attack to this player.
    pub fn now_attacking(mut self, player: PlayerFilter) -> Self {
        self.players_only = true;
        self.attacked_player = Some(player);
        self
    }
}
