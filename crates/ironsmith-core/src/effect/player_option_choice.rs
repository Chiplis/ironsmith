//! Per-player choices among named options.
//!
//! "Each opponent chooses fame or fortune." (Seize the Spotlight), "each
//! opponent chooses money, friends, or secrets" (Master of Ceremonies) and
//! "For each player, choose friend or foe." (Battlebond's friend-or-foe
//! cycle) each make one named choice per participating player. These are not
//! votes (CR 701.38): no vote is cast, so vote triggers and additional-vote
//! effects don't apply. Choices are made in turn order starting with the
//! active player (CR 101.4), each one known to everyone as it is made.
//!
//! The players who ended up with each option are recorded as that option's
//! player set ([`player_option_choice_tag`]) for the instructions that follow
//! in the same resolution ("Each friend …", "For each player who chose fame,
//! …").

use crate::tag::{TagKey, TagKeyWalk};

use super::*;

/// Who makes the choice for each participating player.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum PlayerOptionChooser {
    /// Each participant chooses for themselves ("Each opponent chooses …").
    Participant,
    /// The effect's controller chooses for each participant ("For each
    /// player, choose friend or foe").
    Controller,
}

/// One named choice per participating player; see the module documentation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct ChoosePlayerOptionEffect {
    pub participants: PlayerFilter,
    pub options: Vec<String>,
    pub chooser: PlayerOptionChooser,
}

impl ChoosePlayerOptionEffect {
    pub fn new(
        participants: PlayerFilter,
        options: Vec<String>,
        chooser: PlayerOptionChooser,
    ) -> Self {
        Self {
            participants,
            options,
            chooser,
        }
    }
}

/// The player set recorded for one named option of a
/// [`ChoosePlayerOptionEffect`]: the players who chose it (or for whom it was
/// chosen). Option names compare case-insensitively.
pub fn player_option_choice_tag(option: &str) -> TagKey {
    TagKey::new(format!(
        "__player_option_choice__:{}",
        option.trim().to_ascii_lowercase()
    ))
}

/// "You choose how each player votes this turn." (Illusion of Choice): for
/// the rest of the turn the effect's controller makes every vote choice; each
/// player still casts their own votes (CR 701.38) and still decides whether
/// to vote an additional time.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default, TagKeyWalk)]
pub struct ControlVotesThisTurnEffect;

impl ControlVotesThisTurnEffect {
    pub const fn new() -> Self {
        Self
    }
}
