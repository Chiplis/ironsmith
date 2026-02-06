//! "At the beginning of [player]'s draw step" trigger.

use crate::events::EventKind;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires at the beginning of a player's draw step.
///
/// Used by cards that care about draw step timing.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfDrawStepTrigger {
    /// Which player's draw step triggers this ability.
    pub player: PlayerFilter,
}

impl BeginningOfDrawStepTrigger {
    /// Create a new draw step trigger for the specified player.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create a draw step trigger for your draw step.
    pub fn your_draw_step() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl TriggerMatcher for BeginningOfDrawStepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfDrawStep {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        player_filter_matches(&self.player, player, ctx)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of your draw step".to_string(),
            PlayerFilter::Any => "At the beginning of each player's draw step".to_string(),
            _ => format!("At the beginning of {:?}'s draw step", self.player),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

fn player_filter_matches(filter: &PlayerFilter, player: PlayerId, ctx: &TriggerContext) -> bool {
    match filter {
        PlayerFilter::You => player == ctx.controller,
        PlayerFilter::Opponent => player != ctx.controller,
        PlayerFilter::Any => true,
        PlayerFilter::Specific(id) => player == *id,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfDrawStepEvent;
    use crate::game_state::GameState;
    use crate::ids::ObjectId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_matches_own_draw_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfDrawStepTrigger::your_draw_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfDrawStepEvent::new(alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_draw_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfDrawStepTrigger::your_draw_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfDrawStepEvent::new(bob));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = BeginningOfDrawStepTrigger::your_draw_step();
        assert!(trigger.display().contains("draw step"));
    }
}
