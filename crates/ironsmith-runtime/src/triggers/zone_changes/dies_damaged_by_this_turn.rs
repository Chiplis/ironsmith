//! "Whenever a creature dealt damage by [this/equipped] creature this turn dies" triggers.

use crate::events::EventKind;
use crate::events::zones::ZoneChangeEvent;
use crate::filter::ObjectFilterExt as _;
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
        zc.objects
            .iter()
            .filter(|&&victim_id| {
                let victim_stable_id = zc.snapshot.as_ref().and_then(|snapshot| {
                    (snapshot.object_id == victim_id).then_some(snapshot.stable_id)
                });
                if !self.victim_matches(victim_id, zc, ctx) {
                    return false;
                }
                match self.damager_source {
                    DamagerSource::ThisCreature => ctx
                        .game
                        .turn_store
                        .turn_history
                        .creature_was_damaged_by_source_identity_this_turn(
                            victim_id,
                            victim_stable_id,
                            ctx.source_id,
                            ctx.game.object(ctx.source_id).map(|obj| obj.stable_id),
                        ),
                    // "Equipped/enchanted creature" is read when the trigger
                    // condition is checked: damage dealt this turn by the
                    // creature this source is attached to NOW counts, even if
                    // the attachment happened after the damage (Unscythe).
                    DamagerSource::EquippedCreature | DamagerSource::EnchantedCreature => {
                        let damager = ctx
                            .game
                            .object(ctx.source_id)
                            .and_then(|obj| obj.attached_to.as_ref())
                            .and_then(|target| match target {
                                crate::object::AttachmentTarget::Object(id) => Some(*id),
                                _ => None,
                            });
                        damager.is_some_and(|damager| {
                            ctx.game
                                .turn_store
                                .turn_history
                                .creature_was_damaged_by_source_identity_this_turn(
                                    victim_id,
                                    victim_stable_id,
                                    damager,
                                    ctx.game.object(damager).map(|obj| obj.stable_id),
                                )
                        })
                    }
                }
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
        let victim = self.victim.description();
        let victim = if victim.starts_with("a ")
            || victim.starts_with("an ")
            || victim.starts_with("the ")
            || victim.starts_with("this ")
            || victim.starts_with("that ")
        {
            victim
        } else {
            format!("a {victim}")
        };
        format!(
            "Whenever {} dealt damage by {} this turn dies",
            victim, source_text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::DamageEvent;
    use crate::events::cause::EventCause;
    use crate::events::zones::ZoneChangeEvent;
    use crate::ids::{CardId, PlayerId};
    use crate::object::{AttachmentTarget, Object};
    use crate::provenance::ProvNodeId;
    use crate::target::ObjectFilter;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn create_creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Cat])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_equipment(game: &mut crate::game_state::GameState, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), "Test Equipment")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn equipped_creature_damage_trigger_uses_current_attachment() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let equipment = create_equipment(&mut game, alice);
        let original_equipped = create_creature(&mut game, "Original Equipped", alice);
        let later_equipped = create_creature(&mut game, "Later Equipped", alice);

        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            equipment,
            AttachmentTarget::Object(original_equipped),
        );
        game.record_turn_history_event(&TriggerEvent::new(
            DamageEvent::with_cause(
                original_equipped,
                crate::events::DamageTarget::Object(later_equipped),
                2,
                true,
                EventCause::combat_damage(original_equipped),
            ),
            ProvNodeId::default(),
        ));
        game.record_turn_history_event(&TriggerEvent::new(
            DamageEvent::with_cause(
                later_equipped,
                crate::events::DamageTarget::Object(original_equipped),
                2,
                true,
                EventCause::combat_damage(later_equipped),
            ),
            ProvNodeId::default(),
        ));

        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            equipment,
            AttachmentTarget::Object(later_equipped),
        );

        let trigger = DiesDamagedByThisTurnTrigger::by_equipped_creature(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(equipment, alice, &game);
        let later_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(later_equipped).unwrap(),
            &game,
        );
        let later_dies = TriggerEvent::new(
            ZoneChangeEvent::with_cause(
                later_equipped,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::from_game_rule(),
                Some(later_snapshot),
            ),
            ProvNodeId::default(),
        );
        assert!(
            !trigger.matches(&later_dies, &ctx),
            "damage dealt by a creature that is no longer equipped should not count"
        );

        let original_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(original_equipped).unwrap(),
            &game,
        );
        let original_dies = TriggerEvent::new(
            ZoneChangeEvent::with_cause(
                original_equipped,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::from_game_rule(),
                Some(original_snapshot),
            ),
            ProvNodeId::default(),
        );
        assert!(
            trigger.matches(&original_dies, &ctx),
            "damage dealt this turn by the currently equipped creature counts even if it was dealt before the Equipment was attached"
        );
    }
}
