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
    /// "secretly chooses ..., then the choices are revealed": every choice
    /// is collected before any is recorded, so no participant's choice can
    /// depend on another's (Prisoner's Dilemma, Menacing Ogre).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::ops::Not::not")
    )]
    pub secret: bool,
    /// Each participant chooses a number instead of a named option ("each
    /// player secretly chooses a number"). The players who chose the highest
    /// and the lowest number are recorded ([`player_number_extreme_tag`]),
    /// and [`crate::Value::PlayersChosenNumberExtreme`] reads those numbers.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub number: Option<PlayerNumberChoice>,
    /// "an opponent guesses which number you chose" (The Toymaker's Trap):
    /// the number each participant picks is a guess at the number chosen
    /// earlier in this resolution. Guessers are recorded as right or wrong
    /// ([`player_guess_tag`]) and the guesses are read with
    /// `PlayersChosenNumberExtreme { guess: true, .. }`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::ops::Not::not")
    )]
    pub guess: bool,
    /// "a number ... that hasn't been chosen": numbers this source chose in
    /// earlier resolutions can't be chosen again. The number chosen is
    /// remembered on the source.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::ops::Not::not")
    )]
    pub exclude_source_history: bool,
    /// "an opponent ...": the controller picks one player among
    /// `participants` to make the choice.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "std::ops::Not::not")
    )]
    pub choose_one_participant: bool,
}

/// The legal range of a per-player number choice.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub struct PlayerNumberChoice {
    pub min: u32,
    /// `None` for an unbounded choice ("a number 0 or greater").
    pub max: Option<u32>,
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
            secret: false,
            number: None,
            guess: false,
            exclude_source_history: false,
            choose_one_participant: false,
        }
    }

    /// Each participant chooses a number in `range` themselves.
    pub fn numbers(participants: PlayerFilter, range: PlayerNumberChoice) -> Self {
        Self {
            participants,
            options: Vec::new(),
            chooser: PlayerOptionChooser::Participant,
            secret: false,
            number: Some(range),
            guess: false,
            exclude_source_history: false,
            choose_one_participant: false,
        }
    }

    pub fn with_secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }
}

/// Every player who took part in a [`ChoosePlayerOptionEffect`] ("each of
/// them", "those players").
pub fn player_option_participants_tag() -> TagKey {
    TagKey::new("__player_option_choice_participants__")
}

/// The guessers who guessed right (`true`) or wrong (`false`) in a guessing
/// [`ChoosePlayerOptionEffect`].
pub fn player_guess_tag(right: bool) -> TagKey {
    TagKey::new(if right {
        "__player_number_guess__:right"
    } else {
        "__player_number_guess__:wrong"
    })
}

/// The players who chose the highest (`true`) or the lowest (`false`) number
/// in a per-player number choice. Ties put every tied player in the set.
pub fn player_number_extreme_tag(highest: bool) -> TagKey {
    TagKey::new(if highest {
        "__player_number_choice__:highest"
    } else {
        "__player_number_choice__:lowest"
    })
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
