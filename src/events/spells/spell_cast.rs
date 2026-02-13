//! Spell cast event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// A spell cast event.
///
/// Triggered when a player casts a spell. Used by abilities like
/// "Whenever you cast a spell" or "Whenever an opponent casts a spell".
#[derive(Debug, Clone)]
pub struct SpellCastEvent {
    /// The spell object ID (on the stack)
    pub spell: ObjectId,
    /// The player who cast the spell
    pub caster: PlayerId,
    /// The zone the spell was cast from.
    pub from_zone: Zone,
}

impl SpellCastEvent {
    /// Create a new spell cast event.
    pub fn new(spell: ObjectId, caster: PlayerId, from_zone: Zone) -> Self {
        Self {
            spell,
            caster,
            from_zone,
        }
    }
}

impl GameEventType for SpellCastEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::SpellCast
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.caster
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!("Spell cast by player {:?}", self.caster)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.spell)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.caster)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.caster)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_cast_event_creation() {
        let event = SpellCastEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0), Zone::Hand);
        assert_eq!(event.spell, ObjectId::from_raw(1));
        assert_eq!(event.caster, PlayerId::from_index(0));
        assert_eq!(event.from_zone, Zone::Hand);
    }

    #[test]
    fn test_spell_cast_event_kind() {
        let event = SpellCastEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0), Zone::Hand);
        assert_eq!(event.event_kind(), EventKind::SpellCast);
    }

    #[test]
    fn test_spell_cast_accessors() {
        let event = SpellCastEvent::new(
            ObjectId::from_raw(42),
            PlayerId::from_index(1),
            Zone::Graveyard,
        );
        assert_eq!(event.object_id(), Some(ObjectId::from_raw(42)));
        assert_eq!(event.player(), Some(PlayerId::from_index(1)));
        assert_eq!(event.controller(), Some(PlayerId::from_index(1)));
        assert!(event.snapshot().is_none());
    }
}
