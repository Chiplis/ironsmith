//! Event context for replacement effect matching.
//!
//! This module provides the `EventContext` struct which contains all information
//! needed to evaluate whether a replacement effect matches an event.

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::target::FilterContext;

/// Context provided to replacement matchers for determining if they match an event.
///
/// Contains all the information a replacement effect needs to determine if it applies.
#[derive(Debug, Clone)]
pub struct EventContext<'a> {
    /// The controller of the replacement effect being checked.
    pub controller: PlayerId,

    /// The source object of the replacement effect (if any).
    pub source: Option<ObjectId>,

    /// Filter context for evaluating object/player filters.
    pub filter_ctx: FilterContext,

    /// Reference to the game state for additional lookups.
    pub game: &'a GameState,

    /// Isolated battlefield view for an evolving ETB proposal.
    ///
    /// Replacement matchers use this instead of the object's current old-zone
    /// characteristics when CR 614.12/614.17d prospective evaluation applies.
    pub prospective_etb_game: Option<&'a GameState>,

    /// Last known information for the event's source, when the source has left
    /// the zone it was expected to be in before event matching.
    pub event_source_snapshot: Option<&'a ObjectSnapshot>,
}

/// A matcher query context whose original and entrant worlds have complete
/// continuous-effect discovery. Construction is restricted to checked queries.
#[derive(Debug)]
pub struct PreparedEventContext<'a> {
    context: EventContext<'a>,
    entry_worlds: &'a std::collections::HashMap<ObjectId, GameState>,
}

impl<'a> std::ops::Deref for PreparedEventContext<'a> {
    type Target = EventContext<'a>;
    fn deref(&self) -> &Self::Target { &self.context }
}

impl PreparedEventContext<'_> {
    pub(crate) fn prospective_entry_world(&self, object: ObjectId) -> Option<&GameState> {
        self.context.prospective_etb_game.or_else(|| self.entry_worlds.get(&object))
    }
}

impl<'a> EventContext<'a> {
    /// Create a new event context.
    pub fn new(
        controller: PlayerId,
        source: Option<ObjectId>,
        filter_ctx: FilterContext,
        game: &'a GameState,
    ) -> Self {
        Self {
            controller,
            source,
            filter_ctx,
            game,
            prospective_etb_game: None,
            event_source_snapshot: None,
        }
    }

    pub fn with_prospective_etb_game(mut self, game: Option<&'a GameState>) -> Self {
        self.prospective_etb_game = game;
        self
    }

    /// Attach LKI for the source of the event being matched.
    pub fn with_event_source_snapshot(mut self, snapshot: Option<&'a ObjectSnapshot>) -> Self {
        self.event_source_snapshot = snapshot;
        self
    }

    /// Evaluate one entry proposal against complete original and prospective
    /// query worlds. A supplied prospective world is authoritative (for example,
    /// a batch proposal); validate it without reconstructing or dropping it.
    /// This never publishes query changes into either caller-owned world.
    pub(crate) fn with_complete_entry_query<T>(
        &self,
        event: &crate::events::EnterBattlefieldEvent,
        evaluate: impl FnOnce(&EventContext<'_>) -> T,
    ) -> Result<T, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let original = self.game.continuous_query_snapshot()?;
        let prospective = match self.prospective_etb_game {
            Some(world) => Some(world.continuous_query_snapshot()?),
            None => event.try_prospective_game_state(&original)?,
        };
        let context = EventContext {
            controller: self.controller,
            source: self.source,
            filter_ctx: self.filter_ctx.clone(),
            game: &original,
            prospective_etb_game: prospective.as_ref(),
            event_source_snapshot: self.event_source_snapshot,
        };
        Ok(evaluate(&context))
    }

    /// Prepare complete query worlds before any matcher sees the event.
    /// Each member of a multi-object entry gets its own prospective world;
    /// one entrant's new static abilities do not become pre-existing effects
    /// for its simultaneous siblings. A supplied world remains authoritative.
    pub(crate) fn with_complete_query<T>(
        &self,
        event: &dyn crate::events::GameEventType,
        evaluate: impl FnOnce(&PreparedEventContext<'_>) -> T,
    ) -> Result<T, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let original = self.game.continuous_query_snapshot()?;
        let supplied = self.prospective_etb_game
            .map(GameState::continuous_query_snapshot).transpose()?;
        let mut worlds = std::collections::HashMap::new();
        let entry = crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event);
        if supplied.is_none() {
            if let Some(entry) = entry {
                if let Some(world) = entry.try_prospective_game_state(&original)? {
                    worlds.insert(entry.object, world);
                }
            } else if let Some(change) = crate::events::downcast_event::<crate::events::ZoneChangeEvent>(event)
                && change.to == crate::zone::Zone::Battlefield
            {
                for &object in &change.objects {
                    if worlds.contains_key(&object) { continue; }
                    let proposed = crate::events::EnterBattlefieldEvent::new(object, change.from);
                    if let Some(world) = proposed.try_prospective_game_state(&original)? {
                        worlds.insert(object, world);
                    }
                }
            }
        }
        let context = EventContext {
            controller: self.controller,
            source: self.source,
            filter_ctx: self.filter_ctx.clone(),
            game: &original,
            prospective_etb_game: supplied.as_ref().or_else(|| entry.and_then(|entry| worlds.get(&entry.object))),
            event_source_snapshot: self.event_source_snapshot,
        };
        Ok(evaluate(&PreparedEventContext { context, entry_worlds: &worlds }))
    }

    /// Create an event context for a replacement effect source.
    pub fn for_replacement_effect(
        controller: PlayerId,
        source: ObjectId,
        game: &'a GameState,
    ) -> Self {
        let filter_ctx = game.filter_context_for(controller, Some(source));
        Self::new(controller, Some(source), filter_ctx, game)
    }

    /// Create a minimal context when no specific source is known.
    pub fn for_controller(controller: PlayerId, game: &'a GameState) -> Self {
        let filter_ctx = game.filter_context_for(controller, None);
        Self::new(controller, None, filter_ctx, game)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_context_creation() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let controller = PlayerId::from_index(0);

        let ctx = EventContext::for_controller(controller, &game);

        assert_eq!(ctx.controller, controller);
        assert!(ctx.source.is_none());
    }
    #[test]
    fn complete_entry_query_respects_supplied_world_and_source_snapshot() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Context recipient")
            .card_types(vec![crate::types::CardType::Land]).build();
        let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Hand);
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut event = crate::events::EnterBattlefieldEvent::new(source, crate::zone::Zone::Hand);
        event.enters_with_counters.push((crate::object::CounterType::Charge, 3));
        let mut supplied = event.try_prospective_game_state(&game).unwrap().unwrap();
        supplied.object_mut(source).unwrap().counters.insert(crate::object::CounterType::Charge, 7);
        let ctx = EventContext::for_replacement_effect(bob, source, &game)
            .with_event_source_snapshot(Some(&snapshot))
            .with_prospective_etb_game(Some(&supplied));
        ctx.with_complete_entry_query(&event, |complete| {
            assert_eq!(complete.controller, bob);
            assert_eq!(complete.source, Some(source));
            assert_eq!(complete.event_source_snapshot.unwrap().object_id, source);
            assert_eq!(complete.game.object(source).unwrap().zone, crate::zone::Zone::Hand);
            assert_eq!(complete.prospective_etb_game.unwrap().object(source).unwrap()
                .counters.get(&crate::object::CounterType::Charge), Some(&7),
                "supplied world must not be rebuilt from the event's three-counter proposal");
        }).expect("finite complete context");
        assert!(game.object(source).unwrap().counters.is_empty());
        assert_eq!(supplied.object(source).unwrap().counters.get(&crate::object::CounterType::Charge), Some(&7));
    }

}
