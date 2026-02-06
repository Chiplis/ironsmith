//! Enter battlefield event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::zone::Zone;

/// An enter battlefield event with ETB-specific modifiers.
///
/// This is a specialized zone change event for objects entering the battlefield,
/// allowing replacement effects to modify how the permanent enters (tapped,
/// with counters, etc.).
#[derive(Debug, Clone)]
pub struct EnterBattlefieldEvent {
    /// The object entering
    pub object: ObjectId,
    /// The zone it's coming from
    pub from: Zone,
    /// Whether it enters tapped (may be modified by replacement effects)
    pub enters_tapped: bool,
    /// Counters it enters with (may be modified by replacement effects)
    pub enters_with_counters: Vec<(CounterType, u32)>,
    /// If set, the object enters as a copy of this source object.
    pub enters_as_copy_of: Option<ObjectId>,
}

impl EnterBattlefieldEvent {
    /// Create a new enter battlefield event.
    pub fn new(object: ObjectId, from: Zone) -> Self {
        Self {
            object,
            from,
            enters_tapped: false,
            enters_with_counters: Vec::new(),
            enters_as_copy_of: None,
        }
    }

    /// Create an event where the permanent enters tapped.
    pub fn tapped(object: ObjectId, from: Zone) -> Self {
        Self {
            object,
            from,
            enters_tapped: true,
            enters_with_counters: Vec::new(),
            enters_as_copy_of: None,
        }
    }

    /// Return a new event with enters_tapped set to true.
    pub fn with_tapped(&self) -> Self {
        Self {
            enters_tapped: true,
            ..self.clone()
        }
    }

    /// Return a new event with additional counters.
    pub fn with_counters(&self, counter_type: CounterType, count: u32) -> Self {
        let mut counters = self.enters_with_counters.clone();

        // Add to existing count if same type, otherwise add new entry
        if let Some((_, existing)) = counters.iter_mut().find(|(ct, _)| *ct == counter_type) {
            *existing = existing.saturating_add(count);
        } else {
            counters.push((counter_type, count));
        }

        Self {
            enters_with_counters: counters,
            ..self.clone()
        }
    }

    /// Return a new event where the object enters as a copy of `source_id`.
    pub fn with_copy_of(&self, source_id: ObjectId) -> Self {
        Self {
            enters_as_copy_of: Some(source_id),
            ..self.clone()
        }
    }

    /// Get the total count of a specific counter type.
    pub fn counter_count(&self, counter_type: CounterType) -> u32 {
        self.enters_with_counters
            .iter()
            .filter(|(ct, _)| *ct == counter_type)
            .map(|(_, count)| count)
            .sum()
    }
}

impl GameEventType for EnterBattlefieldEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::EnterBattlefield
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.object)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        let mut desc = "Enter the battlefield".to_string();
        if self.enters_tapped {
            desc.push_str(" tapped");
        }
        if !self.enters_with_counters.is_empty() {
            desc.push_str(" with counters");
        }
        desc
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_battlefield_event_creation() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand);

        assert_eq!(event.from, Zone::Hand);
        assert!(!event.enters_tapped);
        assert!(event.enters_with_counters.is_empty());
    }

    #[test]
    fn test_enter_battlefield_tapped() {
        let event = EnterBattlefieldEvent::tapped(ObjectId::from_raw(1), Zone::Hand);
        assert!(event.enters_tapped);
    }

    #[test]
    fn test_enter_battlefield_with_counters() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 3);

        assert_eq!(event.counter_count(CounterType::PlusOnePlusOne), 3);
    }

    #[test]
    fn test_enter_battlefield_with_multiple_counter_types() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 2)
            .with_counters(CounterType::Loyalty, 3);

        assert_eq!(event.counter_count(CounterType::PlusOnePlusOne), 2);
        assert_eq!(event.counter_count(CounterType::Loyalty), 3);
    }

    #[test]
    fn test_enter_battlefield_counter_stacking() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 2)
            .with_counters(CounterType::PlusOnePlusOne, 3);

        assert_eq!(event.counter_count(CounterType::PlusOnePlusOne), 5);
    }

    #[test]
    fn test_enter_battlefield_event_kind() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand);
        assert_eq!(event.event_kind(), EventKind::EnterBattlefield);
    }

    #[test]
    fn test_enter_battlefield_display() {
        let event = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand);
        assert_eq!(event.display(), "Enter the battlefield");

        let tapped_event = event.with_tapped();
        assert_eq!(tapped_event.display(), "Enter the battlefield tapped");

        let with_counters = EnterBattlefieldEvent::new(ObjectId::from_raw(1), Zone::Hand)
            .with_counters(CounterType::PlusOnePlusOne, 3);
        assert_eq!(
            with_counters.display(),
            "Enter the battlefield with counters"
        );
    }
}
