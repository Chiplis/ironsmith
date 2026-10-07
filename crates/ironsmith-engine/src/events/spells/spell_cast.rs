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
    /// Snapshot of the spell on the stack at cast time.
    pub snapshot: Option<ObjectSnapshot>,
    /// All chosen target slots at completion. None means unavailable evidence;
    /// Some(empty) proves a cast without targets. Kept after stack departure.
    pub targets: Option<Vec<Target>>,
}

impl SpellCastEvent {
    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = Some(targets);
        self
    }

    /// The publication boundary for both priority and effect-driven casts.
    /// Capture only this exact stack incarnation after the cast commits.
    pub fn from_completed_cast(spell: ObjectId, caster: PlayerId, from_zone: Zone, game: &GameState) -> Self {
        Self::try_from_completed_cast(spell, caster, from_zone, game)
            .expect("legacy cast capture requires complete characteristic evidence")
    }

    pub fn try_from_completed_cast(spell: ObjectId, caster: PlayerId, from_zone: Zone, game: &GameState)
        -> Result<Self, crate::effects::ExecutionError>
    {
        Ok(Self {
            spell, caster, from_zone,
            snapshot: game.object(spell).filter(|object| object.zone == Zone::Stack)
                .map(|object| ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)).transpose()?,
            targets: game.stack.iter().find(|entry| entry.object_id == spell && !entry.is_ability)
                .map(|entry| entry.targets.clone()),
        })
    }

    pub fn required_completed_snapshot(&self) -> Result<&ObjectSnapshot, crate::effects::ExecutionError> {
        self.snapshot.as_ref()
            .filter(|snapshot| snapshot.object_id == self.spell && snapshot.zone == Zone::Stack)
            .ok_or_else(|| crate::effects::ExecutionError::IncompleteEvidence(
                "cast quantity requires its exact completed stack snapshot".into(),
            ))
    }

    pub fn cast_quantity(&self, quantity: ironsmith_core::CastEventQuantity) -> Result<i64, crate::effects::ExecutionError> {
        use crate::effects::ExecutionError;
        use ironsmith_core::CastEventQuantity;
        let snapshot = self.required_completed_snapshot()?;
        let amount: u128 = match quantity {
            CastEventQuantity::DistinctTargets => self.targets.as_ref().ok_or_else(|| {
                ExecutionError::IncompleteEvidence("cast target quantity requires completed chosen targets".into())
            })?.iter().copied().collect::<std::collections::HashSet<_>>().len() as u128,
            CastEventQuantity::ManaSymbols(color) => {
                if snapshot.face_down { 0 } else {
                    let symbol = crate::mana::ManaSymbol::from_color(color);
                    snapshot.mana_cost.as_ref().map_or(0, |cost| {
                        cost.pips().iter().filter(|pip| pip.contains(&symbol)).count() as u128
                    })
                }
            }
            CastEventQuantity::ManaValue => {
                if snapshot.face_down { 0 } else if let Some(value) = snapshot.linked_face_mana_value {
                    u128::from(value)
                } else if let Some(cost) = &snapshot.mana_cost {
                    let x = if cost.has_x() { snapshot.x_value.ok_or_else(|| {
                        ExecutionError::IncompleteEvidence("cast mana value requires the announced X".into())
                    })? } else { 0 };
                    cost.pips().iter().map(|pip| {
                        pip.iter().map(|symbol| u128::from(match symbol {
                            crate::mana::ManaSymbol::X => x,
                            other => other.mana_value(),
                        })).max().unwrap_or(0)
                    }).sum()
                } else { 0 }
            }
        };
        i64::try_from(amount).map_err(|_| ExecutionError::ResourceLimitExceeded {
            resource: "completed spell-cast quantity", requested: amount, maximum: i64::MAX as u128,
        })
    }

    /// Create a new spell cast event.
    pub fn new(spell: ObjectId, caster: PlayerId, from_zone: Zone) -> Self {
        Self {
            spell,
            caster,
            from_zone,
            snapshot: None,
            targets: None,
        }
    }

    pub fn new_with_snapshot(
        spell: ObjectId,
        caster: PlayerId,
        from_zone: Zone,
        snapshot: ObjectSnapshot,
    ) -> Self {
        Self {
            spell,
            caster,
            from_zone,
            snapshot: Some(snapshot),
            targets: None,
        }
    }
}

impl GameEventType for SpellCastEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::SpellCast
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.caster
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!("Spell cast by player {}", self.caster.0)
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
        self.snapshot.as_ref()
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
        assert!(event.snapshot.is_none());
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
