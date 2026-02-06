//! "At the beginning of [player]'s end step" trigger.

use crate::events::EventKind;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires at the beginning of a player's end step.
///
/// Used by cards like Conjurer's Closet, Obzedat, and many others.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfEndStepTrigger {
    /// Which player's end step triggers this ability.
    pub player: PlayerFilter,
}

impl BeginningOfEndStepTrigger {
    /// Create a new end step trigger for the specified player.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create an end step trigger for your end step.
    pub fn your_end_step() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Create an end step trigger for each end step.
    pub fn each_end_step() -> Self {
        Self::new(PlayerFilter::Any)
    }
}

impl TriggerMatcher for BeginningOfEndStepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfEndStep {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        player_filter_matches(&self.player, player, ctx)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of your end step".to_string(),
            PlayerFilter::Any => "At the beginning of each end step".to_string(),
            PlayerFilter::Opponent => "At the beginning of each opponent's end step".to_string(),
            _ => format!("At the beginning of {:?}'s end step", self.player),
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
    use crate::events::phase::BeginningOfEndStepEvent;
    use crate::game_state::GameState;
    use crate::ids::ObjectId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_matches_own_end_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfEndStepTrigger::your_end_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfEndStepEvent::new(alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_end_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfEndStepTrigger::your_end_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfEndStepEvent::new(bob));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_each_end_step() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfEndStepTrigger::each_end_step();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event1 = TriggerEvent::new(BeginningOfEndStepEvent::new(alice));
        let event2 = TriggerEvent::new(BeginningOfEndStepEvent::new(bob));
        assert!(trigger.matches(&event1, &ctx));
        assert!(trigger.matches(&event2, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = BeginningOfEndStepTrigger::your_end_step();
        assert!(trigger.display().contains("end step"));
    }
}
