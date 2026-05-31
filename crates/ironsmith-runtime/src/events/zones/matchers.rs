//! Zone change replacement effect matchers.

use crate::events::cause::{CauseFilter, CauseFilterRuntimeExt as _};
use crate::events::context::EventContext;
use crate::events::traits::{
    EventKind, GameEventType, ReplacementMatcher, ReplacementPriority, downcast_event,
};
use crate::filter::PlayerFilterExt;
use crate::filter::{ObjectFilterExt as _, StackObjectKind};
use crate::ids::ObjectId;
use crate::target::ObjectFilter;
use crate::zone::Zone;
use ironsmith_core::DamagedBySource;

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

    fn matches_would_enter_object(&self, object_id: ObjectId, ctx: &EventContext) -> bool {
        let Some(obj) = ctx.game.object(object_id) else {
            return false;
        };

        // Evaluate against the object's prospective battlefield characteristics.
        // Replacement effects trigger before zone change is finalized, so the
        // object may still be in hand/stack/graveyard when this matcher runs.
        let mut prospective = obj.clone();
        prospective.zone = Zone::Battlefield;

        self.filter.matches(&prospective, &ctx.filter_ctx, ctx.game)
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
                zone_change
                    .objects
                    .iter()
                    .any(|&id| self.matches_would_enter_object(id, ctx))
            }
            EventKind::EnterBattlefield => {
                let Some(etb) = downcast_event::<EnterBattlefieldEvent>(event) else {
                    return false;
                };
                self.matches_would_enter_object(etb.object, ctx)
            }
            _ => false,
        }
    }

    fn display(&self) -> String {
        "When a permanent would enter the battlefield".to_string()
    }
}

/// Matches when this specific permanent would enter the battlefield.
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
        ReplacementPriority::Other
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

    fn display(&self) -> String {
        "When a creature would die".to_string()
    }
}

/// Matches when an object would die after being dealt damage by a specific source this turn.
#[derive(Debug, Clone)]
pub struct WouldDieDamagedBySourceThisTurnMatcher {
    pub filter: ObjectFilter,
    pub damaged_by: DamagedBySource,
}

impl WouldDieDamagedBySourceThisTurnMatcher {
    pub fn new(filter: ObjectFilter, damaged_by: DamagedBySource) -> Self {
        Self { filter, damaged_by }
    }

    fn resolve_damager(&self, ctx: &EventContext) -> Option<ObjectId> {
        let source = ctx.source?;
        match self.damaged_by {
            DamagedBySource::ThisCreature => Some(source),
            DamagedBySource::EquippedCreature | DamagedBySource::EnchantedCreature => ctx
                .game
                .object(source)
                .and_then(|obj| obj.attached_to.and_then(|target| target.object_id())),
        }
    }

    fn victim_matches(
        &self,
        victim_id: ObjectId,
        zc: &ZoneChangeEvent,
        ctx: &EventContext,
    ) -> bool {
        if let Some(snapshot) = zc.snapshot.as_ref()
            && snapshot.object_id == victim_id
        {
            return self
                .filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }

        ctx.game
            .object(victim_id)
            .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
    }
}

impl ReplacementMatcher for WouldDieDamagedBySourceThisTurnMatcher {
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
        let Some(damager_id) = self.resolve_damager(ctx) else {
            return false;
        };
        let damager_stable_id = ctx.game.object(damager_id).map(|obj| obj.stable_id);

        zone_change.objects.iter().any(|&victim_id| {
            let victim_stable_id = zone_change.snapshot.as_ref().and_then(|snapshot| {
                (snapshot.object_id == victim_id).then_some(snapshot.stable_id)
            });
            self.victim_matches(victim_id, zone_change, ctx)
                && ctx
                    .game
                    .turn_store
                    .turn_history
                    .creature_was_damaged_by_source_identity_this_turn(
                        victim_id,
                        victim_stable_id,
                        damager_id,
                        damager_stable_id,
                    )
        })
    }

    fn display(&self) -> String {
        let source_text = match self.damaged_by {
            DamagedBySource::ThisCreature => "this creature",
            DamagedBySource::EquippedCreature => "equipped creature",
            DamagedBySource::EnchantedCreature => "enchanted creature",
        };
        format!(
            "When {} dealt damage by {} this turn would die",
            self.filter.description(),
            source_text
        )
    }
}

/// Matches when this specific permanent would die.
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
        ReplacementPriority::Other
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

    fn display(&self) -> String {
        "When an object would be put into a graveyard".to_string()
    }
}

/// Matches when an object matching the filter would move between specific zones.
#[derive(Debug, Clone)]
pub struct WouldChangeZoneMatcher {
    pub filter: ObjectFilter,
    pub from_zone: Option<Zone>,
    pub to_zone: Option<Zone>,
    pub cause_filter: Option<CauseFilter>,
    pub require_cause_source_match: bool,
}

impl WouldChangeZoneMatcher {
    pub fn new(filter: ObjectFilter, from_zone: Option<Zone>, to_zone: Option<Zone>) -> Self {
        Self {
            filter,
            from_zone,
            to_zone,
            cause_filter: None,
            require_cause_source_match: false,
        }
    }

    pub fn with_cause_filter(mut self, cause_filter: CauseFilter) -> Self {
        self.cause_filter = Some(cause_filter);
        self
    }

    pub fn requiring_cause_source_match(mut self) -> Self {
        self.require_cause_source_match = true;
        self
    }
}

impl ReplacementMatcher for WouldChangeZoneMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::ZoneChange {
            return false;
        }

        let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
            return false;
        };

        if self.from_zone.is_some_and(|zone| zone_change.from != zone) {
            return false;
        }
        if self.to_zone.is_some_and(|zone| zone_change.to != zone) {
            return false;
        }
        if let Some(cause_filter) = &self.cause_filter
            && !cause_filter.matches(&zone_change.cause, ctx.game, ctx.controller)
        {
            return false;
        }
        if self.require_cause_source_match
            && !zone_change
                .cause
                .source
                .is_some_and(|source| zone_change.objects.contains(&source))
        {
            return false;
        }

        if zone_change.from == Zone::Stack {
            let mut filter = self.filter.clone();
            let mut filter_ctx = ctx.filter_ctx.clone();

            if filter.zone == Some(Zone::Stack) {
                filter.zone = None;
            }
            if filter.stack_kind == Some(StackObjectKind::Spell) {
                filter.stack_kind = None;
            }

            if let Some(snapshot) = zone_change.snapshot.as_ref().or(ctx.event_source_snapshot) {
                filter_ctx.caster.get_or_insert(snapshot.controller);
                return filter.matches_snapshot(snapshot, &filter_ctx, ctx.game);
            }
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

    fn display(&self) -> String {
        let from = self
            .from_zone
            .map(|zone| format!(" from {zone:?}"))
            .unwrap_or_default();
        let to = self
            .to_zone
            .map(|zone| format!(" to {zone:?}"))
            .unwrap_or_default();
        format!("When an object would change zones{from}{to}")
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

    fn display(&self) -> String {
        "When an object would be exiled".to_string()
    }
}

/// Matches when this specific object would be put into a graveyard from anywhere.
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
        ReplacementPriority::Other
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

    fn display(&self) -> String {
        "When a permanent would leave the battlefield".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::ids::{ObjectId, PlayerId};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn effect_zone_change(
        object: ObjectId,
        from: Zone,
        to: Zone,
        snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> ZoneChangeEvent {
        ZoneChangeEvent::with_cause(
            object,
            from,
            to,
            crate::events::cause::EventCause::effect(),
            snapshot,
        )
    }

    fn create_creature_in_zone(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), "Matcher Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn test_would_enter_battlefield_matcher() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(alice, &game);
        let matcher = WouldEnterBattlefieldMatcher::any();

        // Zone change to battlefield should match
        let event = effect_zone_change(ObjectId::from_raw(1), Zone::Hand, Zone::Battlefield, None);
        // Note: This won't actually match because the object doesn't exist in the game
        // In real usage, the object would be looked up from game state
        assert!(!matcher.matches_event(&event, &ctx));

        // Zone change to graveyard should not match
        let event = effect_zone_change(ObjectId::from_raw(1), Zone::Hand, Zone::Graveyard, None);
        assert!(!matcher.matches_event(&event, &ctx));
    }

    #[test]
    fn test_would_enter_battlefield_matcher_uses_prospective_battlefield_zone_for_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature_in_zone(&mut game, alice, Zone::Hand);
        let source_id = ObjectId::from_raw(9999);

        let filter = ObjectFilter::creature()
            .you_control()
            .in_zone(Zone::Battlefield);
        let matcher = WouldEnterBattlefieldMatcher::new(filter);
        let ctx = EventContext::for_replacement_effect(alice, source_id, &game);

        let zone_change = effect_zone_change(creature_id, Zone::Hand, Zone::Battlefield, None);
        assert!(
            matcher.matches_event(&zone_change, &ctx),
            "zone-change ETB matcher should evaluate the object as entering battlefield"
        );

        let etb = EnterBattlefieldEvent::new(creature_id, Zone::Hand);
        assert!(
            matcher.matches_event(&etb, &ctx),
            "ETB matcher should evaluate the object as entering battlefield"
        );
    }

    #[test]
    fn test_this_would_enter_battlefield_priority_is_other() {
        let matcher = ThisWouldEnterBattlefieldMatcher;
        assert_eq!(matcher.priority(), ReplacementPriority::Other);
    }

    #[test]
    fn test_this_would_die_priority_is_other() {
        let matcher = ThisWouldDieMatcher;
        assert_eq!(matcher.priority(), ReplacementPriority::Other);
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
    fn test_this_would_go_to_graveyard_priority_is_other() {
        let matcher = ThisWouldGoToGraveyardMatcher;
        assert_eq!(matcher.priority(), ReplacementPriority::Other);
    }

    #[test]
    fn test_this_would_go_to_graveyard_matches_zone_change() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        // Create context with source set so the matcher can compare object identity.
        let ctx = EventContext::for_replacement_effect(alice, source_id, &game);
        let matcher = ThisWouldGoToGraveyardMatcher;

        // Zone change to graveyard for the source should match
        let event = effect_zone_change(source_id, Zone::Library, Zone::Graveyard, None);
        assert!(matcher.matches_event(&event, &ctx));

        // Zone change to graveyard for different object should not match
        let other_id = ObjectId::from_raw(2);
        let event = effect_zone_change(other_id, Zone::Library, Zone::Graveyard, None);
        assert!(!matcher.matches_event(&event, &ctx));

        // Zone change to exile should not match
        let event = effect_zone_change(source_id, Zone::Library, Zone::Exile, None);
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
        let event = effect_zone_change(ObjectId::from_raw(1), Zone::Library, Zone::Hand, None);
        // This won't match because the object doesn't exist in game
        assert!(!matcher.matches_event(&event, &ctx));

        // Zone change to battlefield should not match
        let event = effect_zone_change(
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
