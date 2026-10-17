//! "You may have defending player play with their hand revealed for as long
//! as this creature remains on the battlefield." (Stromgald Spy)
//!
//! The players resolved from `player` keep their hands revealed — the cards
//! in hand now and every card that later enters it — while the effect's
//! source stays on the battlefield as the same object. Like the "play with
//! your hand revealed" statics, the reveal is public information that the
//! hidden-information (mental-poker) layer opens through the ordinary public
//! hand-view path.
use crate::tag::TagKeyWalk;
use crate::PlayerFilter;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, TagKeyWalk)]
pub enum HandRevealDuration {
    /// "for as long as this creature remains on the battlefield"
    #[default]
    WhileSourceOnBattlefield,
    /// "This turn, that player ... plays with their hand revealed" (Sen
    /// Triplets): until the turn the effect resolved in ends.
    ThisTurn,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct PlayWithHandRevealedEffect {
    /// Whose hand is revealed, resolved once as the effect resolves.
    pub player: PlayerFilter,
    /// How long the reveal lasts.
    #[cfg_attr(feature = "serde", serde(default))]
    pub duration: HandRevealDuration,
}

impl PlayWithHandRevealedEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            duration: HandRevealDuration::WhileSourceOnBattlefield,
        }
    }

    pub fn this_turn(player: PlayerFilter) -> Self {
        Self {
            player,
            duration: HandRevealDuration::ThisTurn,
        }
    }
}
