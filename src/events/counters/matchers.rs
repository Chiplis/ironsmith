//! Counter replacement effect matchers.

use crate::events::cause::CauseFilter;
use crate::events::context::EventContext;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, downcast_event};
use crate::object::CounterType;
use crate::target::ObjectFilter;

use super::PutCountersEvent;

/// Matches when counters would be put on a permanent matching the filter.
#[derive(Debug, Clone)]
pub struct WouldPutCountersMatcher {
    pub filter: ObjectFilter,
    pub counter_type: Option<CounterType>,
    pub cause_filter: CauseFilter,
}

impl WouldPutCountersMatcher {
    pub fn new(filter: ObjectFilter, counter_type: Option<CounterType>) -> Self {
        Self {
            filter,
            counter_type,
            cause_filter: CauseFilter::any(),
        }
    }

    /// Matches any counter type on any permanent.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent(), None)
    }

    /// Matches +1/+1 counters on any creature.
    pub fn plus_one_on_creature() -> Self {
        Self::new(ObjectFilter::creature(), Some(CounterType::PlusOnePlusOne))
    }

    /// Restrict this matcher to counters put by causes matching the filter.
    pub fn with_cause_filter(mut self, cause_filter: CauseFilter) -> Self {
        self.cause_filter = cause_filter;
        self
    }
}

impl ReplacementMatcher for WouldPutCountersMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::PutCounters {
            return false;
        }

        let Some(put_counters) = downcast_event::<PutCountersEvent>(event) else {
            return false;
        };

        // Check counter type if specified
        if let Some(required_type) = &self.counter_type
            && put_counters.counter_type != *required_type
        {
            return false;
        }

        if !self.cause_filter.matches(
            &put_counters.cause,
            ctx.game,
            put_counters.affected_player(ctx.game),
        ) {
            return false;
        }

        // Check if target matches the filter
        if let Some(obj) = ctx.game.object(put_counters.target) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        match &self.counter_type {
            Some(ct) => format!(
                "When {} counters would be put on a permanent",
                ct.description()
            ),
            None => "When counters would be put on a permanent".to_string(),
        }
    }
}

/// Matches when counters would be removed from a permanent matching the filter.
#[derive(Debug, Clone)]
pub struct WouldRemoveCountersMatcher {
    pub filter: ObjectFilter,
    pub counter_type: Option<CounterType>,
}

impl WouldRemoveCountersMatcher {
    pub fn new(filter: ObjectFilter, counter_type: Option<CounterType>) -> Self {
        Self {
            filter,
            counter_type,
        }
    }

    /// Matches any counter type on any permanent.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent(), None)
    }
}

impl ReplacementMatcher for WouldRemoveCountersMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::RemoveCounters {
            return false;
        }

        let Some(remove_counters) = downcast_event::<super::RemoveCountersEvent>(event) else {
            return false;
        };

        // Check counter type if specified
        if let Some(required_type) = &self.counter_type
            && remove_counters.counter_type != *required_type
        {
            return false;
        }

        // Check if target matches the filter
        if let Some(obj) = ctx.game.object(remove_counters.target) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        match &self.counter_type {
            Some(ct) => format!(
                "When {} counters would be removed from a permanent",
                ct.description()
            ),
            None => "When counters would be removed from a permanent".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cause::{CauseType, CauseTypeFilter};
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_would_put_counters_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldPutCountersMatcher::any();

        // The matcher needs an actual object in the game to match against
        // This test verifies the basic structure
        let event = PutCountersEvent::with_cause(
            ObjectId::from_raw(1),
            CounterType::PlusOnePlusOne,
            3,
            crate::events::cause::EventCause::effect(),
        );

        // Won't match because object doesn't exist in game
        assert!(!matcher.matches_event(&event, &ctx));
    }

    #[test]
    fn test_would_put_counters_with_type_filter() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldPutCountersMatcher::plus_one_on_creature();

        // Test with wrong counter type
        let event = PutCountersEvent::with_cause(
            ObjectId::from_raw(1),
            CounterType::Loyalty,
            3,
            crate::events::cause::EventCause::effect(),
        );

        // Won't match even if object existed because counter type is wrong
        assert!(!matcher.matches_event(&event, &ctx));
    }

    #[test]
    fn test_matcher_display() {
        let matcher = WouldPutCountersMatcher::any();
        assert_eq!(
            matcher.display(),
            "When counters would be put on a permanent"
        );

        let matcher = WouldPutCountersMatcher::plus_one_on_creature();
        assert_eq!(
            matcher.display(),
            "When +1/+1 counters would be put on a permanent"
        );
    }

    #[test]
    fn test_would_put_counters_with_cause_filter() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldPutCountersMatcher::any().with_cause_filter(CauseFilter {
            cause_type: Some(CauseTypeFilter::Exact(CauseType::Effect)),
            source_filter: None,
            controller_filter: None,
        });

        let event = PutCountersEvent::with_cause(
            ObjectId::from_raw(1),
            CounterType::PlusOnePlusOne,
            1,
            crate::events::cause::EventCause::from_combat_damage(ObjectId::from_raw(2), alice),
        );

        assert!(!matcher.matches_event(&event, &ctx));
    }
}
