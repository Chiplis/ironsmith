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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::triggers::TriggerEvent;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> crate::game_state::GameState {
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(
        game: &mut crate::game_state::GameState,
        card_id: u32,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_equipment(
        game: &mut crate::game_state::GameState,
        card_id: u32,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![crate::types::Subtype::Equipment])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn matches_creature_damaged_by_this_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, 1, "Source", alice);
        let victim = create_creature(&mut game, 2, "Victim", alice);
        game.record_creature_damaged_by_this_turn(victim, source);

        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(victim).unwrap(), &game);
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            victim,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));
        let trigger = DiesDamagedByThisTurnTrigger::by_this_creature(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_without_damage_record() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, 1, "Source", alice);
        let victim = create_creature(&mut game, 2, "Victim", alice);

        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(victim).unwrap(), &game);
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            victim,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));
        let trigger = DiesDamagedByThisTurnTrigger::by_this_creature(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn matches_creature_damaged_by_equipped_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let equipment = create_equipment(&mut game, 1, "Sword", alice);
        let equipped_creature = create_creature(&mut game, 2, "Bearer", alice);
        let victim = create_creature(&mut game, 3, "Victim", alice);

        game.object_mut(equipment).unwrap().attached_to = Some(equipped_creature);
        game.record_creature_damaged_by_this_turn(victim, equipped_creature);

        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(victim).unwrap(), &game);
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            victim,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));
        let trigger = DiesDamagedByThisTurnTrigger::by_equipped_creature(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(equipment, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }
}
