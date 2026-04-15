//! Event context for replacement effect matching.
//!
//! This module provides the `EventContext` struct which contains all information
//! needed to evaluate whether a replacement effect matches an event.

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::FilterContext;

/// Context provided to replacement matchers for determining if they match an event.
///
/// Contains all the information a replacement effect needs to determine if it applies.
#[derive(Debug, Clone)]
pub struct EventContext<'a> {
    /// The controller of the replacement effect being checked.
    pub controller: PlayerId,

    /// The source object of the replacement effect (if any).
    pub source: Option<ObjectId>,

    /// Filter context for evaluating object/player filters.
    pub filter_ctx: FilterContext,

    /// Reference to the game state for additional lookups.
    pub game: &'a GameState,
}

impl<'a> EventContext<'a> {
    /// Create a new event context.
    pub fn new(
        controller: PlayerId,
        source: Option<ObjectId>,
        filter_ctx: FilterContext,
        game: &'a GameState,
    ) -> Self {
        Self {
            controller,
            source,
            filter_ctx,
            game,
        }
    }

    /// Create an event context for a replacement effect source.
    pub fn for_replacement_effect(
        controller: PlayerId,
        source: ObjectId,
        game: &'a GameState,
    ) -> Self {
        let filter_ctx = game.filter_context_for(controller, Some(source));
        Self::new(controller, Some(source), filter_ctx, game)
    }

    /// Create a minimal context when no specific source is known.
    pub fn for_controller(controller: PlayerId, game: &'a GameState) -> Self {
        let filter_ctx = game.filter_context_for(controller, None);
        Self::new(controller, None, filter_ctx, game)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_context_creation() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let controller = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(controller, &game);

        assert_eq!(ctx.controller, controller);
        assert!(ctx.source.is_none());
    }
}
