//! Rules module for MTG game mechanics.
//!
//! This module contains the rules logic for combat, damage, and state-based actions.

pub mod combat;
pub mod damage;
pub mod state_based;

pub use combat::{can_attack, can_block, has_vigilance, minimum_blockers, must_attack};
pub use damage::{
    DamageResult, DamageTarget, calculate_damage, calculate_trample_excess, is_lethal,
};
pub use state_based::{StateBasedAction, apply_state_based_actions, check_state_based_actions};

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Context for rules evaluation.
///
/// Provides information needed to evaluate rules like "you", the source object,
/// and the current game state.
#[derive(Debug, Clone)]
pub struct RulesContext<'a> {
    /// The current game state.
    pub game: &'a GameState,

    /// The source object (e.g., the creature attacking, the spell dealing damage).
    pub source: ObjectId,

    /// The controller of the source object.
    pub controller: PlayerId,
}

impl<'a> RulesContext<'a> {
    /// Create a new rules context.
    pub fn new(game: &'a GameState, source: ObjectId, controller: PlayerId) -> Self {
        Self {
            game,
            source,
            controller,
        }
    }

    /// Create a rules context from a game state and source object.
    ///
    /// The controller is determined from the source object.
    pub fn from_source(game: &'a GameState, source: ObjectId) -> Option<Self> {
        let obj = game.object(source)?;
        Some(Self {
            game,
            source,
            controller: obj.controller,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_context_creation() {
        let game = GameState::new(vec!["Alice".to_string()], 20);
        let ctx = RulesContext::new(&game, ObjectId::from_raw(1), PlayerId::from_index(0));

        assert_eq!(ctx.source, ObjectId::from_raw(1));
        assert_eq!(ctx.controller, PlayerId::from_index(0));
    }
}
