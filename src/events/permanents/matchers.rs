//! Permanent replacement effect matchers (tap, untap, destroy, sacrifice).

use crate::events::context::EventContext;
use crate::events::traits::{
    EventKind, GameEventType, ReplacementMatcher, ReplacementPriority, downcast_event,
};
use crate::target::ObjectFilter;

use super::{DestroyEvent, SacrificeEvent, TapEvent, UntapEvent};

/// Matches when a permanent matching the filter would become tapped.
#[derive(Debug, Clone)]
pub struct WouldBecomeTappedMatcher {
    pub filter: ObjectFilter,
}

impl WouldBecomeTappedMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches any permanent becoming tapped.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent())
    }

    /// Matches any creature becoming tapped.
    pub fn creature() -> Self {
        Self::new(ObjectFilter::creature())
    }
}

impl ReplacementMatcher for WouldBecomeTappedMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::BecomeTapped {
            return false;
        }

        let Some(tap) = downcast_event::<TapEvent>(event) else {
            return false;
        };

        if let Some(obj) = ctx.game.object(tap.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When a permanent would become tapped".to_string()
    }
}

/// Matches when a permanent matching the filter would become untapped.
#[derive(Debug, Clone)]
pub struct WouldBecomeUntappedMatcher {
    pub filter: ObjectFilter,
}

impl WouldBecomeUntappedMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches any permanent becoming untapped.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent())
    }
}

impl ReplacementMatcher for WouldBecomeUntappedMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::BecomeUntapped {
            return false;
        }

        let Some(untap) = downcast_event::<UntapEvent>(event) else {
            return false;
        };

        if let Some(obj) = ctx.game.object(untap.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When a permanent would become untapped".to_string()
    }
}

/// Matches when a permanent matching the filter would be destroyed.
#[derive(Debug, Clone)]
pub struct WouldBeDestroyedMatcher {
    pub filter: ObjectFilter,
}

impl WouldBeDestroyedMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches any permanent being destroyed.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent())
    }

    /// Matches any creature being destroyed.
    pub fn creature() -> Self {
        Self::new(ObjectFilter::creature())
    }
}

impl ReplacementMatcher for WouldBeDestroyedMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Destroy {
            return false;
        }

        let Some(destroy) = downcast_event::<DestroyEvent>(event) else {
            return false;
        };

        if let Some(obj) = ctx.game.object(destroy.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When a permanent would be destroyed".to_string()
    }
}

/// Matches when this specific permanent would be destroyed (self-replacement).
#[derive(Debug, Clone)]
pub struct ThisWouldBeDestroyedMatcher;

impl ReplacementMatcher for ThisWouldBeDestroyedMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Destroy {
            return false;
        }

        let Some(destroy) = downcast_event::<DestroyEvent>(event) else {
            return false;
        };

        ctx.source == Some(destroy.permanent)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When this permanent would be destroyed".to_string()
    }
}

/// Matches when a permanent matching the filter would be sacrificed.
#[derive(Debug, Clone)]
pub struct WouldBeSacrificedMatcher {
    pub filter: ObjectFilter,
}

impl WouldBeSacrificedMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches any permanent being sacrificed.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent())
    }

    /// Matches any creature being sacrificed.
    pub fn creature() -> Self {
        Self::new(ObjectFilter::creature())
    }
}

impl ReplacementMatcher for WouldBeSacrificedMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Sacrifice {
            return false;
        }

        let Some(sacrifice) = downcast_event::<SacrificeEvent>(event) else {
            return false;
        };

        if let Some(obj) = ctx.game.object(sacrifice.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When a permanent would be sacrificed".to_string()
    }
}

/// Matches when a specific creature (protected by regeneration) would be destroyed.
///
/// Unlike `ThisWouldBeDestroyedMatcher` which uses the effect's source,
/// this matcher tracks a specific creature ID that has a regeneration shield.
/// It's used to implement regeneration as a proper replacement effect.
#[derive(Debug, Clone)]
pub struct RegenerationShieldMatcher {
    /// The object protected by this regeneration shield.
    pub protected: crate::ids::ObjectId,
}

impl RegenerationShieldMatcher {
    /// Create a matcher for a creature with a regeneration shield.
    pub fn new(protected_creature: crate::ids::ObjectId) -> Self {
        Self {
            protected: protected_creature,
        }
    }
}

impl ReplacementMatcher for RegenerationShieldMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Destroy {
            return false;
        }

        let Some(destroy) = downcast_event::<DestroyEvent>(event) else {
            return false;
        };

        // Check if the permanent being destroyed matches our protected creature
        if destroy.permanent != self.protected {
            return false;
        }

        // Verify the creature is still on the battlefield and is still a creature
        if let Some(obj) = ctx.game.object(destroy.permanent) {
            obj.is_creature() && obj.zone == crate::zone::Zone::Battlefield
        } else {
            false
        }
    }

    fn priority(&self) -> ReplacementPriority {
        // Regeneration is a self-replacement effect (it only affects the specific creature)
        ReplacementPriority::SelfReplacement
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "Regeneration shield".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_would_become_tapped_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldBecomeTappedMatcher::any();

        // The matcher needs an actual object in the game to match against
        let event = TapEvent::new(ObjectId::from_raw(1));

        // Won't match because object doesn't exist in game
        assert!(!matcher.matches_event(&event, &ctx));
    }

    #[test]
    fn test_this_would_be_destroyed_priority() {
        let matcher = ThisWouldBeDestroyedMatcher;
        assert_eq!(matcher.priority(), ReplacementPriority::SelfReplacement);
    }

    #[test]
    fn test_matcher_display() {
        let matcher = WouldBecomeTappedMatcher::any();
        assert_eq!(matcher.display(), "When a permanent would become tapped");

        let matcher = WouldBeDestroyedMatcher::any();
        assert_eq!(matcher.display(), "When a permanent would be destroyed");

        let matcher = WouldBeSacrificedMatcher::any();
        assert_eq!(matcher.display(), "When a permanent would be sacrificed");
    }
}
