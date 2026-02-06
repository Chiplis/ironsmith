//! Discard event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;

/// A discard event that can be processed through the replacement effect system.
///
/// This can be replaced by effects like Library of Leng.
#[derive(Debug, Clone)]
pub struct DiscardEvent {
    /// The card being discarded
    pub card: ObjectId,
    /// The player who owns/controls the card being discarded
    pub player: PlayerId,
    /// The destination zone (normally Graveyard, but can be replaced)
    pub destination: Zone,
    /// What caused this discard (effect, cost, game rule, etc.).
    /// Library of Leng only applies to effect-like discards, not costs.
    pub cause: EventCause,
    /// Whether type verification is required for the discard
    /// (e.g., "discard a land card" requires verification)
    pub requires_type_verification: bool,
}

impl DiscardEvent {
    /// Create a new discard event from an effect.
    pub fn from_effect(card: ObjectId, player: PlayerId) -> Self {
        Self {
            card,
            player,
            destination: Zone::Graveyard,
            cause: EventCause::default(), // Effect with no specific source
            requires_type_verification: false,
        }
    }

    /// Create a new discard event from an effect with a specific source.
    pub fn from_effect_with_source(
        card: ObjectId,
        player: PlayerId,
        source: ObjectId,
        source_controller: PlayerId,
    ) -> Self {
        Self {
            card,
            player,
            destination: Zone::Graveyard,
            cause: EventCause::from_effect(source, source_controller),
            requires_type_verification: false,
        }
    }

    /// Create a new discard event as a cost.
    pub fn as_cost(card: ObjectId, player: PlayerId) -> Self {
        Self {
            card,
            player,
            destination: Zone::Graveyard,
            cause: EventCause {
                cause_type: crate::events::cause::CauseType::Cost,
                source: None,
                source_controller: Some(player),
            },
            requires_type_verification: false,
        }
    }

    /// Create a new discard event as a cost with a specific source.
    pub fn as_cost_with_source(card: ObjectId, player: PlayerId, source: ObjectId) -> Self {
        Self {
            card,
            player,
            destination: Zone::Graveyard,
            cause: EventCause::from_cost(source, player),
            requires_type_verification: false,
        }
    }

    /// Create a new discard event from a game rule (e.g., cleanup step).
    pub fn from_game_rule(card: ObjectId, player: PlayerId) -> Self {
        Self {
            card,
            player,
            destination: Zone::Graveyard,
            cause: EventCause::from_game_rule(),
            requires_type_verification: false,
        }
    }

    /// Create a discard event with a custom cause.
    pub fn with_cause(card: ObjectId, player: PlayerId, cause: EventCause) -> Self {
        Self {
            card,
            player,
            destination: Zone::Graveyard,
            cause,
            requires_type_verification: false,
        }
    }

    /// Return a new event with a different destination zone.
    pub fn with_destination(&self, destination: Zone) -> Self {
        Self {
            destination,
            ..self.clone()
        }
    }

    /// Return a new event that requires type verification.
    pub fn with_type_verification(mut self) -> Self {
        self.requires_type_verification = true;
        self
    }
}

impl GameEventType for DiscardEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Discard
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        // Discard events don't have redirectable targets
        // The destination zone is changed via a different mechanism
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        "Discard a card".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cause::CauseType;

    #[test]
    fn test_discard_event_from_effect() {
        let event = DiscardEvent::from_effect(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert_eq!(event.cause.cause_type, CauseType::Effect);
        assert_eq!(event.destination, Zone::Graveyard);
    }

    #[test]
    fn test_discard_event_as_cost() {
        let event = DiscardEvent::as_cost(ObjectId::from_raw(1), PlayerId::from_index(0));

        assert_eq!(event.cause.cause_type, CauseType::Cost);
        assert_eq!(event.destination, Zone::Graveyard);
    }

    #[test]
    fn test_discard_event_from_game_rule() {
        let event = DiscardEvent::from_game_rule(ObjectId::from_raw(1), PlayerId::from_index(0));

        // Game rule discards are effect-like (Library of Leng applies)
        assert_eq!(event.cause.cause_type, CauseType::GameRule);
    }

    #[test]
    fn test_discard_event_with_destination() {
        let event = DiscardEvent::from_effect(ObjectId::from_raw(1), PlayerId::from_index(0));
        let changed = event.with_destination(Zone::Library);

        assert_eq!(changed.destination, Zone::Library);
    }

    #[test]
    fn test_discard_event_kind() {
        let event = DiscardEvent::from_effect(ObjectId::from_raw(1), PlayerId::from_index(0));
        assert_eq!(event.event_kind(), EventKind::Discard);
    }

    #[test]
    fn test_discard_event_display() {
        let event = DiscardEvent::from_effect(ObjectId::from_raw(1), PlayerId::from_index(0));
        assert_eq!(event.display(), "Discard a card");
    }

    #[test]
    fn test_discard_event_with_source() {
        let source = ObjectId::from_raw(99);
        let controller = PlayerId::from_index(1);
        let event = DiscardEvent::from_effect_with_source(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            source,
            controller,
        );

        assert_eq!(event.cause.source, Some(source));
        assert_eq!(event.cause.source_controller, Some(controller));
    }
}
