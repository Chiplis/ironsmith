//! Card draw/discard replacement effect matchers.

use crate::events::cause::{CauseFilter, CauseFilterRuntimeExt as _};
use crate::events::context::EventContext;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, downcast_event};
use crate::filter::PlayerFilterExt;
use crate::target::PlayerFilter;

use super::{DiscardEvent, DrawEvent};

/// Matches when a player matching the filter would draw a card.
#[derive(Debug, Clone)]
pub struct WouldDrawCardMatcher {
    pub player_filter: PlayerFilter,
}

impl WouldDrawCardMatcher {
    pub fn new(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches when you (the controller) would draw a card.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Matches when any player would draw a card.
    pub fn any_player() -> Self {
        Self::new(PlayerFilter::Any)
    }

    /// Matches when an opponent would draw a card.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl ReplacementMatcher for WouldDrawCardMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Draw {
            return false;
        }

        let Some(draw) = downcast_event::<DrawEvent>(event) else {
            return false;
        };

        self.player_filter
            .matches_player(draw.player, &ctx.filter_ctx)
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::You => "When you would draw a card".to_string(),
            PlayerFilter::Any => "When any player would draw a card".to_string(),
            PlayerFilter::Opponent => "When an opponent would draw a card".to_string(),
            _ => "When a player would draw a card".to_string(),
        }
    }
}

/// Matches when a player would draw a card while that player's library is empty.
#[derive(Debug, Clone)]
pub struct WouldDrawCardWhileLibraryEmptyMatcher {
    pub player_filter: PlayerFilter,
}

impl WouldDrawCardWhileLibraryEmptyMatcher {
    pub fn new(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches when you (the controller) would draw from an empty library.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl ReplacementMatcher for WouldDrawCardWhileLibraryEmptyMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Draw {
            return false;
        }

        let Some(draw) = downcast_event::<DrawEvent>(event) else {
            return false;
        };

        self.player_filter
            .matches_player(draw.player, &ctx.filter_ctx)
            && ctx
                .game
                .player(draw.player)
                .is_some_and(|player| player.library.is_empty())
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::You => {
                "When you would draw a card while your library has no cards in it".to_string()
            }
            _ => "When a player would draw a card while their library has no cards in it"
                .to_string(),
        }
    }
}

/// Matches when a player would draw their first card each turn.
#[derive(Debug, Clone)]
pub struct WouldDrawFirstCardMatcher {
    pub player_filter: PlayerFilter,
}

impl WouldDrawFirstCardMatcher {
    pub fn new(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches when you would draw your first card this turn.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl ReplacementMatcher for WouldDrawFirstCardMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Draw {
            return false;
        }

        let Some(draw) = downcast_event::<DrawEvent>(event) else {
            return false;
        };

        if !draw.is_first_this_turn {
            return false;
        }

        self.player_filter
            .matches_player(draw.player, &ctx.filter_ctx)
    }

    fn display(&self) -> String {
        "When you would draw your first card each turn".to_string()
    }
}

/// Matches when a card would be discarded.
///
/// Supports filtering on:
/// - Which player is discarding
/// - What caused the discard (effect, cost, game rule, etc.)
/// - Properties of the source that caused the discard
#[derive(Debug, Clone)]
pub struct WouldDiscardMatcher {
    /// Filter on which player is discarding.
    pub player_filter: PlayerFilter,
    /// Filter on what caused the discard.
    pub cause_filter: CauseFilter,
}

impl WouldDiscardMatcher {
    /// Create a new discard matcher with full control over filters.
    pub fn new(player_filter: PlayerFilter, cause_filter: CauseFilter) -> Self {
        Self {
            player_filter,
            cause_filter,
        }
    }

    /// Matches when you would discard a card (from any source).
    pub fn you() -> Self {
        Self::new(PlayerFilter::You, CauseFilter::any())
    }

    /// Matches when you would discard from an effect-like source.
    /// This is what Library of Leng uses.
    pub fn you_from_effect() -> Self {
        Self::new(PlayerFilter::You, CauseFilter::effect_like())
    }

    /// Matches when any player would discard a card.
    pub fn any_player() -> Self {
        Self::new(PlayerFilter::Any, CauseFilter::any())
    }

    /// Matches when an opponent would discard a card.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent, CauseFilter::any())
    }

    /// Add a cause filter to this matcher.
    pub fn with_cause_filter(mut self, cause_filter: CauseFilter) -> Self {
        self.cause_filter = cause_filter;
        self
    }
}

impl ReplacementMatcher for WouldDiscardMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Discard {
            return false;
        }

        let Some(discard) = downcast_event::<DiscardEvent>(event) else {
            return false;
        };

        // Check player filter
        if !self
            .player_filter
            .matches_player(discard.player, &ctx.filter_ctx)
        {
            return false;
        }

        // Check cause filter
        if !self
            .cause_filter
            .matches(&discard.cause, ctx.game, discard.player)
        {
            return false;
        }

        true
    }

    fn display(&self) -> String {
        match (&self.player_filter, &self.cause_filter.cause_type) {
            (PlayerFilter::You, Some(crate::events::cause::CauseTypeFilter::EffectLike)) => {
                "When an effect causes you to discard a card".to_string()
            }
            (PlayerFilter::You, _) => "When you would discard a card".to_string(),
            (PlayerFilter::Opponent, _) => "When an opponent would discard a card".to_string(),
            (PlayerFilter::Any, _) => "When any player would discard a card".to_string(),
            _ => "When a player would discard a card".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_would_draw_card_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldDrawCardMatcher::you();

        // Alice draws - should match
        let event_alice = DrawEvent::new(alice, 1, false);
        assert!(matcher.matches_event(&event_alice, &ctx));

        // Bob draws - should not match
        let event_bob = DrawEvent::new(bob, 1, false);
        assert!(!matcher.matches_event(&event_bob, &ctx));
    }

    #[test]
    fn test_would_draw_first_card_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldDrawFirstCardMatcher::you();

        // First draw this turn - should match
        let event_first = DrawEvent::new(alice, 1, true);
        assert!(matcher.matches_event(&event_first, &ctx));

        // Not first draw - should not match
        let event_not_first = DrawEvent::new(alice, 1, false);
        assert!(!matcher.matches_event(&event_not_first, &ctx));
    }

    #[test]
    fn test_would_discard_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldDiscardMatcher::you_from_effect();

        // Discard from effect - should match
        let event_effect = DiscardEvent::with_cause(
            ObjectId::from_raw(1),
            alice,
            crate::events::cause::EventCause::effect(),
        );
        assert!(matcher.matches_event(&event_effect, &ctx));

        // Discard as cost - should not match (effect-like filter)
        let event_cost = DiscardEvent::with_cause(
            ObjectId::from_raw(1),
            alice,
            crate::events::cause::EventCause::from_cost(ObjectId::from_raw(1), alice),
        );
        assert!(!matcher.matches_event(&event_cost, &ctx));

        // Discard from game rule - should match
        let event_rule = DiscardEvent::with_cause(
            ObjectId::from_raw(1),
            alice,
            crate::events::cause::EventCause::from_game_rule(),
        );
        assert!(!matcher.matches_event(&event_rule, &ctx));
    }

    #[test]
    fn test_would_discard_matcher_any_cause() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldDiscardMatcher::you(); // any cause

        // Should match both effect and cost discards
        let event_effect = DiscardEvent::with_cause(
            ObjectId::from_raw(1),
            alice,
            crate::events::cause::EventCause::effect(),
        );
        assert!(matcher.matches_event(&event_effect, &ctx));

        let event_cost = DiscardEvent::with_cause(
            ObjectId::from_raw(1),
            alice,
            crate::events::cause::EventCause::from_cost(ObjectId::from_raw(1), alice),
        );
        assert!(matcher.matches_event(&event_cost, &ctx));
    }

    #[test]
    fn test_matcher_display() {
        let matcher = WouldDrawCardMatcher::you();
        assert_eq!(matcher.display(), "When you would draw a card");

        let matcher = WouldDiscardMatcher::you_from_effect();
        assert_eq!(
            matcher.display(),
            "When an effect causes you to discard a card"
        );

        let matcher = WouldDiscardMatcher::you();
        assert_eq!(matcher.display(), "When you would discard a card");
    }
}
