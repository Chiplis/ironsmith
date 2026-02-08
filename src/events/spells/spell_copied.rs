//! Spell copied event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A spell copied event.
///
/// Triggered when a player copies a spell on the stack. Used by abilities like
/// "Whenever you cast or copy an instant or sorcery spell".
#[derive(Debug, Clone)]
pub struct SpellCopiedEvent {
    /// The copied spell object ID (on the stack)
    pub spell: ObjectId,
    /// The player who copied the spell
    pub copier: PlayerId,
}

impl SpellCopiedEvent {
    /// Create a new spell copied event.
    pub fn new(spell: ObjectId, copier: PlayerId) -> Self {
        Self { spell, copier }
    }
}

impl GameEventType for SpellCopiedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::SpellCopied
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.copier
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!("Spell copied by player {:?}", self.copier)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.spell)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.copier)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.copier)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_copied_event_creation() {
        let event = SpellCopiedEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0));
        assert_eq!(event.spell, ObjectId::from_raw(1));
        assert_eq!(event.copier, PlayerId::from_index(0));
    }

    #[test]
    fn test_spell_copied_event_kind() {
        let event = SpellCopiedEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0));
        assert_eq!(event.event_kind(), EventKind::SpellCopied);
    }

    #[test]
    fn test_spell_copied_accessors() {
        let event = SpellCopiedEvent::new(ObjectId::from_raw(42), PlayerId::from_index(1));
        assert_eq!(event.object_id(), Some(ObjectId::from_raw(42)));
        assert_eq!(event.player(), Some(PlayerId::from_index(1)));
        assert_eq!(event.controller(), Some(PlayerId::from_index(1)));
        assert!(event.snapshot().is_none());
    }
}
