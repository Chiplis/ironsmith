//! Zone change replacement effect matchers.

use crate::events::context::EventContext;
use crate::events::traits::{
    EventKind, GameEventType, ReplacementMatcher, ReplacementPriority, downcast_event,
};
use crate::target::ObjectFilter;
use crate::zone::Zone;

use super::{EnterBattlefieldEvent, ZoneChangeEvent};

/// Matches when an object matching the filter would enter the battlefield.
#[derive(Debug, Clone)]
pub struct WouldEnterBattlefieldMatcher {
    pub filter: ObjectFilter,
}

impl WouldEnterBattlefieldMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches any creature entering the battlefield.
    pub fn creature() -> Self {
        Self::new(ObjectFilter::creature())
    }

    /// Matches any permanent entering the battlefield.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent())
    }
}

impl ReplacementMatcher for WouldEnterBattlefieldMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        match event.event_kind() {
            EventKind::ZoneChange => {
                let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
                    return false;
                };
                if zone_change.to != Zone::Battlefield {
                    return false;
                }
                if let Some(obj) = zone_change
                    .objects
                    .first()
                    .and_then(|&id| ctx.game.object(id))
                {
                    self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                } else {
                    false
                }
            }
            EventKind::EnterBattlefield => {
                let Some(etb) = downcast_event::<EnterBattlefieldEvent>(event) else {
                    return false;
                };
                if let Some(obj) = ctx.game.object(etb.object) {
                    self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When a permanent would enter the battlefield".to_string()
    }
}

/// Matches when this specific permanent would enter the battlefield (self-replacement).
#[derive(Debug, Clone)]
pub struct ThisWouldEnterBattlefieldMatcher;

impl ReplacementMatcher for ThisWouldEnterBattlefieldMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        let object_id = match event.event_kind() {
            EventKind::ZoneChange => {
                let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
                    return false;
                };
                if zone_change.to != Zone::Battlefield {
                    return false;
                }
                let Some(&obj) = zone_change.objects.first() else {
                    return false;
                };
                obj
            }
            EventKind::EnterBattlefield => {
                let Some(etb) = downcast_event::<EnterBattlefieldEvent>(event) else {
                    return false;
                };
                etb.object
            }
            _ => return false,
        };

        ctx.source == Some(object_id)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When this permanent would enter the battlefield".to_string()
    }
}

/// Matches when an object matching the filter would die.
#[derive(Debug, Clone)]
pub struct WouldDieMatcher {
    pub filter: ObjectFilter,
}

impl WouldDieMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }

    /// Matches any creature dying.
    pub fn creature() -> Self {
        Self::new(ObjectFilter::creature())
    }

    /// Matches any permanent dying.
    pub fn any() -> Self {
        Self::new(ObjectFilter::permanent())
    }
}

impl ReplacementMatcher for WouldDieMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::ZoneChange {
            return false;
        }

        let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
            return false;
        };
        if zone_change.from != Zone::Battlefield || zone_change.to != Zone::Graveyard {
            return false;
        }
        if let Some(obj) = zone_change
            .objects
            .first()
            .and_then(|&id| ctx.game.object(id))
        {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When a creature would die".to_string()
    }
}

/// Matches when this specific permanent would die (self-replacement).
#[derive(Debug, Clone)]
pub struct ThisWouldDieMatcher;

impl ReplacementMatcher for ThisWouldDieMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        let object_id = if event.event_kind() == EventKind::ZoneChange {
            let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
                return false;
            };
            if zone_change.from != Zone::Battlefield || zone_change.to != Zone::Graveyard {
                return false;
            }
            let Some(&obj) = zone_change.objects.first() else {
                return false;
            };
            obj
        } else {
            return false;
        };

        ctx.source == Some(object_id)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When this permanent would die".to_string()
    }
}

/// Matches when an object would go to the graveyard from any zone.
#[derive(Debug, Clone)]
pub struct WouldGoToGraveyardMatcher {
    pub filter: ObjectFilter,
}

impl WouldGoToGraveyardMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl ReplacementMatcher for WouldGoToGraveyardMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::ZoneChange {
            return false;
        }

        let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
            return false;
        };
        if zone_change.to != Zone::Graveyard {
            return false;
        }
        if let Some(obj) = zone_change
            .objects
            .first()
            .and_then(|&id| ctx.game.object(id))
        {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When an object would be put into a graveyard".to_string()
    }
}

/// Matches when an object would be exiled.
#[derive(Debug, Clone)]
pub struct WouldBeExiledMatcher {
    pub filter: ObjectFilter,
}

impl WouldBeExiledMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl ReplacementMatcher for WouldBeExiledMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::ZoneChange {
            return false;
        }

        let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
            return false;
        };

        if zone_change.to != Zone::Exile {
            return false;
        }

        if let Some(obj) = zone_change
            .objects
            .first()
            .and_then(|&id| ctx.game.object(id))
        {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When an object would be exiled".to_string()
    }
}

/// Matches when this specific object would be put into a graveyard from anywhere (self-replacement).
///
/// Unlike `ThisWouldDieMatcher` which only matches battlefield to graveyard,
/// this matches any zone to graveyard (e.g., library to graveyard, hand to graveyard).
/// Used by effects like Darksteel Colossus: "If this would be put into a graveyard
/// from anywhere, reveal it and shuffle it into its owner's library instead."
#[derive(Debug, Clone)]
pub struct ThisWouldGoToGraveyardMatcher;

impl ReplacementMatcher for ThisWouldGoToGraveyardMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        let object_id = if event.event_kind() == EventKind::ZoneChange {
            let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
                return false;
            };
            if zone_change.to != Zone::Graveyard {
                return false;
            }
            let Some(&obj) = zone_change.objects.first() else {
                return false;
            };
            obj
        } else {
            return false;
        };

        ctx.source == Some(object_id)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When this permanent would be put into a graveyard from anywhere".to_string()
    }
}

/// Matches when a card would be put into a player's hand.
#[derive(Debug, Clone)]
pub struct WouldGoToHandMatcher {
    pub player_filter: crate::target::PlayerFilter,
}

impl WouldGoToHandMatcher {
    pub fn new(player_filter: crate::target::PlayerFilter) -> Self {
        Self { player_filter }
    }

    /// Matches cards going to your hand.
    pub fn you() -> Self {
        Self::new(crate::target::PlayerFilter::You)
    }

    /// Matches cards going to any player's hand.
    pub fn any_player() -> Self {
        Self::new(crate::target::PlayerFilter::Any)
    }
}

impl ReplacementMatcher for WouldGoToHandMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::ZoneChange {
            return false;
        }

        let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
            return false;
        };

        if zone_change.to != Zone::Hand {
            return false;
        }

        // Get the owner of the object to check player filter
        if let Some(obj) = zone_change
            .objects
            .first()
            .and_then(|&id| ctx.game.object(id))
        {
            self.player_filter
                .matches_player(obj.owner, &ctx.filter_ctx)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        match &self.player_filter {
            crate::target::PlayerFilter::You => {
                "When a card would be put into your hand".to_string()
            }
            crate::target::PlayerFilter::Opponent => {
                "When a card would be put into an opponent's hand".to_string()
            }
            _ => "When a card would be put into a player's hand".to_string(),
        }
    }
}

/// Matches when an object would leave the battlefield.
#[derive(Debug, Clone)]
pub struct WouldLeaveBattlefieldMatcher {
    pub filter: ObjectFilter,
}

impl WouldLeaveBattlefieldMatcher {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl ReplacementMatcher for WouldLeaveBattlefieldMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::ZoneChange {
            return false;
        }

        let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
            return false;
        };
        if zone_change.from != Zone::Battlefield {
            return false;
        }
        if let Some(obj) = zone_change
            .objects
            .first()
            .and_then(|&id| ctx.game.object(id))
        {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        "When a permanent would leave the battlefield".to_string()
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
    fn test_would_enter_battlefield_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldEnterBattlefieldMatcher::any();

        // Zone change to battlefield should match
        let event =
            ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Hand, Zone::Battlefield, None);
        // Note: This won't actually match because the object doesn't exist in the game
        // In real usage, the object would be looked up from game state
        assert!(!matcher.matches_event(&event, &ctx));

        // Zone change to graveyard should not match
        let event = ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Hand, Zone::Graveyard, None);
        assert!(!matcher.matches_event(&event, &ctx));
    }

    #[test]
    fn test_this_would_enter_battlefield_self_replacement() {
        let matcher = ThisWouldEnterBattlefieldMatcher;
        assert_eq!(matcher.priority(), ReplacementPriority::SelfReplacement);
    }

    #[test]
    fn test_this_would_die_self_replacement() {
        let matcher = ThisWouldDieMatcher;
        assert_eq!(matcher.priority(), ReplacementPriority::SelfReplacement);
    }

    #[test]
    fn test_matcher_display() {
        let matcher = WouldEnterBattlefieldMatcher::any();
        assert_eq!(
            matcher.display(),
            "When a permanent would enter the battlefield"
        );

        let matcher = WouldDieMatcher::creature();
        assert_eq!(matcher.display(), "When a creature would die");
    }

    #[test]
    fn test_this_would_go_to_graveyard_self_replacement() {
        let matcher = ThisWouldGoToGraveyardMatcher;
        assert_eq!(matcher.priority(), ReplacementPriority::SelfReplacement);
    }

    #[test]
    fn test_this_would_go_to_graveyard_matches_zone_change() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        // Create context with source set (for self-replacement matching)
        let ctx = EventContext::for_replacement_effect(alice, source_id, &game);
        let matcher = ThisWouldGoToGraveyardMatcher;

        // Zone change to graveyard for the source should match
        let event = ZoneChangeEvent::new(source_id, Zone::Library, Zone::Graveyard, None);
        assert!(matcher.matches_event(&event, &ctx));

        // Zone change to graveyard for different object should not match
        let other_id = ObjectId::from_raw(2);
        let event = ZoneChangeEvent::new(other_id, Zone::Library, Zone::Graveyard, None);
        assert!(!matcher.matches_event(&event, &ctx));

        // Zone change to exile should not match
        let event = ZoneChangeEvent::new(source_id, Zone::Library, Zone::Exile, None);
        assert!(!matcher.matches_event(&event, &ctx));
    }

    #[test]
    fn test_this_would_go_to_graveyard_display() {
        let matcher = ThisWouldGoToGraveyardMatcher;
        assert_eq!(
            matcher.display(),
            "When this permanent would be put into a graveyard from anywhere"
        );
    }

    #[test]
    fn test_would_go_to_hand_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldGoToHandMatcher::you();

        // Zone change to hand should try to match (won't match because object doesn't exist)
        let event = ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Library, Zone::Hand, None);
        // This won't match because the object doesn't exist in game
        assert!(!matcher.matches_event(&event, &ctx));

        // Zone change to battlefield should not match
        let event = ZoneChangeEvent::new(
            ObjectId::from_raw(1),
            Zone::Library,
            Zone::Battlefield,
            None,
        );
        assert!(!matcher.matches_event(&event, &ctx));
    }

    #[test]
    fn test_would_go_to_hand_display() {
        let matcher = WouldGoToHandMatcher::you();
        assert_eq!(matcher.display(), "When a card would be put into your hand");

        let matcher = WouldGoToHandMatcher::any_player();
        assert_eq!(
            matcher.display(),
            "When a card would be put into a player's hand"
        );
    }
}
