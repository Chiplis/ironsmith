//! Damage replacement effect matchers.

use crate::events::context::EventContext;
use crate::events::traits::{
    EventKind, GameEventType, ReplacementMatcher, ReplacementPriority, downcast_event,
};
use crate::game_event::DamageTarget;
use crate::target::{ObjectFilter, PlayerFilter};

use super::DamageEvent;

/// Matches damage events where the target is a player matching the filter.
#[derive(Debug, Clone)]
pub struct DamageToPlayerMatcher {
    pub player_filter: PlayerFilter,
}

impl DamageToPlayerMatcher {
    pub fn new(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches damage to "you" (the controller of the replacement effect).
    pub fn to_you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Matches damage to any player.
    pub fn to_any_player() -> Self {
        Self::new(PlayerFilter::Any)
    }

    /// Matches damage to any opponent.
    pub fn to_opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl ReplacementMatcher for DamageToPlayerMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        match damage.target {
            DamageTarget::Player(player_id) => self
                .player_filter
                .matches_player(player_id, &ctx.filter_ctx),
            DamageTarget::Object(_) => false,
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::You => "When damage would be dealt to you".to_string(),
            PlayerFilter::Any => "When damage would be dealt to any player".to_string(),
            PlayerFilter::Opponent => "When damage would be dealt to an opponent".to_string(),
            _ => "When damage would be dealt to a player".to_string(),
        }
    }
}

/// Matches damage events where the target is an object matching the filter.
#[derive(Debug, Clone)]
pub struct DamageToObjectMatcher {
    pub filter: ObjectFilter,
}

impl DamageToObjectMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches damage to any creature.
    pub fn to_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }

    /// Matches damage to any permanent.
    pub fn to_permanent() -> Self {
        Self::new(ObjectFilter::permanent())
    }
}

impl ReplacementMatcher for DamageToObjectMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        match damage.target {
            DamageTarget::Object(object_id) => {
                if let Some(obj) = ctx.game.object(object_id) {
                    self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                } else {
                    false
                }
            }
            DamageTarget::Player(_) => false,
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When damage would be dealt to a permanent".to_string()
    }
}

/// Matches combat damage events.
#[derive(Debug, Clone)]
pub struct CombatDamageMatcher;

impl ReplacementMatcher for CombatDamageMatcher {
    fn matches_event(&self, event: &dyn GameEventType, _ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        damage.is_combat
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When combat damage would be dealt".to_string()
    }
}

/// Matches noncombat damage events.
#[derive(Debug, Clone)]
pub struct NoncombatDamageMatcher;

impl ReplacementMatcher for NoncombatDamageMatcher {
    fn matches_event(&self, event: &dyn GameEventType, _ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        !damage.is_combat
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When noncombat damage would be dealt".to_string()
    }
}

/// Matches damage events from a source matching the filter.
#[derive(Debug, Clone)]
pub struct DamageFromSourceMatcher {
    pub filter: ObjectFilter,
}

impl DamageFromSourceMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches damage from any creature.
    pub fn from_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }
}

impl ReplacementMatcher for DamageFromSourceMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        if let Some(obj) = ctx.game.object(damage.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When damage would be dealt by a source".to_string()
    }
}

/// Matches damage events to the source of the replacement effect (self-replacement).
#[derive(Debug, Clone)]
pub struct DamageToSelfMatcher {
    /// Whether this is a self-replacement effect.
    pub self_replacement: bool,
}

impl DamageToSelfMatcher {
    pub fn new() -> Self {
        Self {
            self_replacement: true,
        }
    }
}

impl Default for DamageToSelfMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplacementMatcher for DamageToSelfMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }

        let Some(damage) = downcast_event::<DamageEvent>(event) else {
            return false;
        };

        match damage.target {
            DamageTarget::Object(object_id) => ctx.source == Some(object_id),
            DamageTarget::Player(_) => false,
        }
    }

    fn priority(&self) -> ReplacementPriority {
        if self.self_replacement {
            ReplacementPriority::SelfReplacement
        } else {
            ReplacementPriority::Other
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When damage would be dealt to this permanent".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::ObjectId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_damage_to_player_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = DamageToPlayerMatcher::to_you();

        // Damage to Alice (the controller) should match
        let event_to_alice =
            DamageEvent::new(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, false);
        assert!(matcher.matches_event(&event_to_alice, &ctx));

        // Damage to Bob should not match "you"
        let event_to_bob =
            DamageEvent::new(ObjectId::from_raw(1), DamageTarget::Player(bob), 3, false);
        assert!(!matcher.matches_event(&event_to_bob, &ctx));
    }

    #[test]
    fn test_combat_damage_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = CombatDamageMatcher;

        let combat_damage =
            DamageEvent::new(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, true);
        assert!(matcher.matches_event(&combat_damage, &ctx));

        let noncombat_damage =
            DamageEvent::new(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, false);
        assert!(!matcher.matches_event(&noncombat_damage, &ctx));
    }

    #[test]
    fn test_noncombat_damage_matcher() {
        let game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = NoncombatDamageMatcher;

        let noncombat_damage =
            DamageEvent::new(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, false);
        assert!(matcher.matches_event(&noncombat_damage, &ctx));

        let combat_damage =
            DamageEvent::new(ObjectId::from_raw(1), DamageTarget::Player(alice), 3, true);
        assert!(!matcher.matches_event(&combat_damage, &ctx));
    }

    #[test]
    fn test_damage_to_self_matcher_priority() {
        let matcher = DamageToSelfMatcher::new();
        assert_eq!(matcher.priority(), ReplacementPriority::SelfReplacement);
    }
}
