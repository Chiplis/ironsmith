//! "This turn, instead of declaring blockers, each defending player chooses
//! any number of creatures they control and divides them into a number of
//! piles equal to the number of attacking creatures for whom that player is
//! the defending player. ... Assign each pile to a different one of those
//! attacking creatures at random. Each creature in a pile that can block the
//! creature that pile is assigned to does so." (Camouflage)
//!
//! Resolving registers the rule for this turn; the declare-blockers step then
//! asks for a pile division and derives the blocks from a random assignment
//! (see `game_loop::assign_random_pile_blocks`).

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type DeclareBlockersByRandomPilesThisTurnEffect =
    ironsmith_core::DeclareBlockersByRandomPilesThisTurnEffect;

impl EffectExecutor for DeclareBlockersByRandomPilesThisTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.declare_blockers_by_random_piles_this_turn();
        Ok(EffectOutcome::resolved())
    }
}
