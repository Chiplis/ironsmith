//! "Roll to visit your Attractions" as an instruction (Line Cutter, Command
//! Performance): CR 701.52a — roll a six-sided die, then visit each Attraction
//! you control whose lit-up numbers include the result. The turn-based action
//! of CR 505.5b performs the same action through the same engine owner.
use crate::filter_model::PlayerFilter;
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct RollToVisitAttractionsEffect {
    /// The player who rolls and whose Attractions are visited.
    pub player: PlayerFilter,
}

impl RollToVisitAttractionsEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}
