//! Life gain/loss replacement effect matchers.

use crate::events::context::EventContext;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, downcast_event};
use crate::target::PlayerFilter;

use super::{LifeGainEvent, LifeLossEvent};

/// Matches life gain events for players matching the filter.
#[derive(Debug, Clone)]
pub struct WouldGainLifeMatcher {
    pub player_filter: PlayerFilter,
}

impl WouldGainLifeMatcher {
    pub fn new(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches when you (the controller) would gain life.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Matches when any player would gain life.
    pub fn any_player() -> Self {
        Self::new(PlayerFilter::Any)
    }

    /// Matches when an opponent would gain life.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl ReplacementMatcher for WouldGainLifeMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::LifeGain {
            return false;
        }

        let Some(life_gain) = downcast_event::<LifeGainEvent>(event) else {
            return false;
        };

        self.player_filter
            .matches_player(life_gain.player, &ctx.filter_ctx)
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::You => "When you would gain life".to_string(),
            PlayerFilter::Any => "When any player would gain life".to_string(),
            PlayerFilter::Opponent => "When an opponent would gain life".to_string(),
            _ => "When a player would gain life".to_string(),
        }
    }
}

/// Matches life loss events for players matching the filter.
#[derive(Debug, Clone)]
pub struct WouldLoseLifeMatcher {
    pub player_filter: PlayerFilter,
}

impl WouldLoseLifeMatcher {
    pub fn new(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches when you (the controller) would lose life.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Matches when any player would lose life.
    pub fn any_player() -> Self {
        Self::new(PlayerFilter::Any)
    }

    /// Matches when an opponent would lose life.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl ReplacementMatcher for WouldLoseLifeMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::LifeLoss {
            return false;
        }

        let Some(life_loss) = downcast_event::<LifeLossEvent>(event) else {
            return false;
        };

        self.player_filter
            .matches_player(life_loss.player, &ctx.filter_ctx)
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::You => "When you would lose life".to_string(),
            PlayerFilter::Any => "When any player would lose life".to_string(),
            PlayerFilter::Opponent => "When an opponent would lose life".to_string(),
            _ => "When a player would lose life".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_would_gain_life_matcher_you() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldGainLifeMatcher::you();

        // Alice gains life - should match
        let event_alice = LifeGainEvent::new(alice, 5);
        assert!(matcher.matches_event(&event_alice, &ctx));

        // Bob gains life - should not match
        let event_bob = LifeGainEvent::new(bob, 5);
        assert!(!matcher.matches_event(&event_bob, &ctx));
    }

    #[test]
    fn test_would_gain_life_matcher_any() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldGainLifeMatcher::any_player();

        // Both should match
        let event_alice = LifeGainEvent::new(alice, 5);
        assert!(matcher.matches_event(&event_alice, &ctx));

        let event_bob = LifeGainEvent::new(bob, 5);
        assert!(matcher.matches_event(&event_bob, &ctx));
    }

    #[test]
    fn test_would_lose_life_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldLoseLifeMatcher::opponent();

        // Alice loses life - should not match (not opponent)
        let event_alice = LifeLossEvent::from_effect(alice, 5);
        assert!(!matcher.matches_event(&event_alice, &ctx));

        // Bob loses life - should match (is opponent)
        let event_bob = LifeLossEvent::from_effect(bob, 5);
        assert!(matcher.matches_event(&event_bob, &ctx));
    }

    #[test]
    fn test_matcher_display() {
        let matcher = WouldGainLifeMatcher::you();
        assert_eq!(matcher.display(), "When you would gain life");

        let matcher = WouldLoseLifeMatcher::any_player();
        assert_eq!(matcher.display(), "When any player would lose life");
    }
}
