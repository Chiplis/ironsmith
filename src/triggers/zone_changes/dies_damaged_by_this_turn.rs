//! "Whenever a creature dealt damage by [this/equipped] creature this turn dies" triggers.

use crate::events::EventKind;
use crate::events::zones::ZoneChangeEvent;
use crate::ids::ObjectId;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Which source object is used when matching "dealt damage by ... this turn".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamagerSource {
    /// The trigger source itself ("this creature").
    ThisCreature,
    /// The creature this source is attached to ("equipped creature").
    EquippedCreature,
    /// The creature this source enchants ("enchanted creature").
    EnchantedCreature,
}

/// Trigger for "Whenever a creature dealt damage by [source] this turn dies".
#[derive(Debug, Clone, PartialEq)]
pub struct DiesDamagedByThisTurnTrigger {
    /// Victim filter for the creature that dies.
    pub victim: ObjectFilter,
    /// Where to read the damager object from.
    pub damager_source: DamagerSource,
}

impl DiesDamagedByThisTurnTrigger {
    /// Create a trigger where the damager is the source object.
    pub fn by_this_creature(victim: ObjectFilter) -> Self {
        Self {
            victim,
            damager_source: DamagerSource::ThisCreature,
        }
    }

    /// Create a trigger where the damager is the creature this source is attached to.
    pub fn by_equipped_creature(victim: ObjectFilter) -> Self {
        Self {
            victim,
            damager_source: DamagerSource::EquippedCreature,
        }
    }

    /// Create a trigger where the damager is the creature this source enchants.
    pub fn by_enchanted_creature(victim: ObjectFilter) -> Self {
        Self {
            victim,
            damager_source: DamagerSource::EnchantedCreature,
        }
    }

    fn resolve_damager(&self, ctx: &TriggerContext) -> Option<ObjectId> {
        match self.damager_source {
            DamagerSource::ThisCreature => Some(ctx.source_id),
            DamagerSource::EquippedCreature | DamagerSource::EnchantedCreature => ctx
                .game
                .object(ctx.source_id)
                .and_then(|obj| obj.attached_to),
        }
    }

    fn victim_matches(
        &self,
        victim_id: ObjectId,
        zc: &ZoneChangeEvent,
        ctx: &TriggerContext,
    ) -> bool {
        if let Some(snapshot) = zc.snapshot.as_ref()
            && snapshot.object_id == victim_id
        {
            return self
                .victim
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }

        ctx.game
            .object(victim_id)
            .is_some_and(|obj| self.victim.matches(obj, &ctx.filter_ctx, ctx.game))
    }

    fn matching_objects(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        if event.kind() != EventKind::ZoneChange {
            return 0;
        }
        let Some(zc) = event.downcast::<ZoneChangeEvent>() else {
            return 0;
        };
        if !zc.is_dies() {
            return 0;
        }
        let Some(damager_id) = self.resolve_damager(ctx) else {
            return 0;
        };

        zc.objects
            .iter()
            .filter(|&&victim_id| {
                self.victim_matches(victim_id, zc, ctx)
                    && ctx
                        .game
                        .creature_was_damaged_by_source_this_turn(victim_id, damager_id)
            })
            .count() as u32
    }
}

impl TriggerMatcher for DiesDamagedByThisTurnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.matching_objects(event, ctx) > 0
    }

    fn uses_snapshot(&self) -> bool {
        true
    }

    fn display(&self) -> String {
        let source_text = match self.damager_source {
            DamagerSource::ThisCreature => "this creature",
            DamagerSource::EquippedCreature => "equipped creature",
            DamagerSource::EnchantedCreature => "enchanted creature",
        };
        format!(
            "Whenever {} dealt damage by {} this turn dies",
            self.victim.description(),
            source_text
        )
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}
