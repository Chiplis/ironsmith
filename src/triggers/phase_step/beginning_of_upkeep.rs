//! "At the beginning of [player]'s upkeep" trigger.

use crate::events::EventKind;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires at the beginning of a player's upkeep step.
///
/// Used by cards like Black Market Connections, Bitterblossom, and many others.
#[derive(Debug, Clone, PartialEq)]
pub struct BeginningOfUpkeepTrigger {
    /// Which player's upkeep triggers this ability.
    pub player: PlayerFilter,
}

impl BeginningOfUpkeepTrigger {
    /// Create a new upkeep trigger for the specified player.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create an upkeep trigger for your upkeep.
    pub fn your_upkeep() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Create an upkeep trigger for each player's upkeep.
    pub fn each_upkeep() -> Self {
        Self::new(PlayerFilter::Any)
    }
}

impl TriggerMatcher for BeginningOfUpkeepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BeginningOfUpkeep {
            return false;
        }
        let Some(player) = event.player() else {
            return false;
        };
        player_filter_matches(&self.player, player, ctx)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "At the beginning of your upkeep".to_string(),
            PlayerFilter::Any => "At the beginning of each player's upkeep".to_string(),
            PlayerFilter::Opponent => "At the beginning of each opponent's upkeep".to_string(),
            _ => format!("At the beginning of {:?}'s upkeep", self.player),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

/// Check if a PlayerFilter matches a specific player in the trigger context.
fn player_filter_matches(filter: &PlayerFilter, player: PlayerId, ctx: &TriggerContext) -> bool {
    match filter {
        PlayerFilter::You => player == ctx.controller,
        PlayerFilter::Opponent => player != ctx.controller,
        PlayerFilter::Any => true,
        PlayerFilter::Specific(id) => player == *id,
        PlayerFilter::Active => player == ctx.game.turn.active_player,
        _ => true, // Default to true for complex filters evaluated at runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfDrawStepEvent;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::game_state::GameState;
    use crate::ids::ObjectId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_matches_own_upkeep() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(alice));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_upkeep() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(bob));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_each_upkeep_matches_both() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::each_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event1 = TriggerEvent::new(BeginningOfUpkeepEvent::new(alice));
        let event2 = TriggerEvent::new(BeginningOfUpkeepEvent::new(bob));
        assert!(trigger.matches(&event1, &ctx));
        assert!(trigger.matches(&event2, &ctx));
    }

    #[test]
    fn test_does_not_match_non_upkeep_events() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(BeginningOfDrawStepEvent::new(alice));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display_your_upkeep() {
        let trigger = BeginningOfUpkeepTrigger::your_upkeep();
        assert!(trigger.display().contains("your upkeep"));
    }

    #[test]
    fn test_display_each_upkeep() {
        let trigger = BeginningOfUpkeepTrigger::each_upkeep();
        assert!(trigger.display().contains("each player's upkeep"));
    }
}
