//! Death triggers qualified by damage from a typed source filter this turn.

use crate::events::EventKind;
use crate::events::zones::ZoneChangeEvent;
use crate::events::{DamageEvent, DamageTarget};
use crate::filter::ObjectFilterExt as _;
use crate::ids::ObjectId;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// "Whenever a creature dealt damage this turn by [source filter] dies."
#[derive(Debug, Clone, PartialEq)]
pub struct DiesDamagedByFilteredSourceThisTurnTrigger {
    pub victim: ObjectFilter,
    pub damager_filter: ObjectFilter,
}

impl DiesDamagedByFilteredSourceThisTurnTrigger {
    pub fn new(victim: ObjectFilter, damager_filter: ObjectFilter) -> Self {
        Self {
            victim,
            damager_filter,
        }
    }

    fn victim_matches(
        &self,
        victim_id: ObjectId,
        event: &ZoneChangeEvent,
        ctx: &TriggerContext,
    ) -> bool {
        if let Some(snapshot) = event.snapshot.as_ref()
            && snapshot.object_id == victim_id
        {
            return self
                .victim
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }
        ctx.game
            .object(victim_id)
            .is_some_and(|object| self.victim.matches(object, &ctx.filter_ctx, ctx.game))
    }

    fn was_damaged_by_matching_source(
        &self,
        victim_id: ObjectId,
        victim_stable_id: Option<crate::ids::StableId>,
        ctx: &TriggerContext,
    ) -> bool {
        ctx.game
            .turn_store
            .turn_history
            .projected_records()
            .any(|record| {
                let Some(damage) = record.event.downcast::<DamageEvent>() else {
                    return false;
                };
                if damage.amount == 0 {
                    return false;
                }
                let target_matches = match damage.target {
                    DamageTarget::Object(target) if target == victim_id => true,
                    DamageTarget::Object(_) => victim_stable_id.is_some_and(|stable_id| {
                        damage
                            .target_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                    }),
                    DamageTarget::Player(_) => false,
                };
                if !target_matches {
                    return false;
                }

                if let Some(source_snapshot) = record.source_snapshot.as_ref() {
                    return self.damager_filter.matches_snapshot(
                        source_snapshot,
                        &ctx.filter_ctx,
                        ctx.game,
                    );
                }
                ctx.game.object(damage.source).is_some_and(|source| {
                    self.damager_filter
                        .matches(source, &ctx.filter_ctx, ctx.game)
                })
            })
    }

    fn matching_objects(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        if event.kind() != EventKind::ZoneChange {
            return 0;
        }
        let Some(zone_change) = event.downcast::<ZoneChangeEvent>() else {
            return 0;
        };
        if !zone_change.is_dies() {
            return 0;
        }
        zone_change
            .objects
            .iter()
            .filter(|&&victim_id| {
                let victim_stable_id = zone_change.snapshot.as_ref().and_then(|snapshot| {
                    (snapshot.object_id == victim_id).then_some(snapshot.stable_id)
                });
                self.victim_matches(victim_id, zone_change, ctx)
                    && self.was_damaged_by_matching_source(victim_id, victim_stable_id, ctx)
            })
            .count() as u32
    }
}

fn with_indefinite_article(text: &str) -> String {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
    {
        return text.to_string();
    }
    let article = if lower
        .chars()
        .next()
        .is_some_and(|first| matches!(first, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {text}")
}

impl TriggerMatcher for DiesDamagedByFilteredSourceThisTurnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.matching_objects(event, ctx) > 0
    }

    fn uses_snapshot(&self) -> bool {
        true
    }

    fn display(&self) -> String {
        let victim = with_indefinite_article(&self.victim.description());
        let mut source_filter = self.damager_filter.clone();
        source_filter.zone = None;
        let damager = with_indefinite_article(&source_filter.description())
            .replace(" you control", " you controlled")
            .replace(" an opponent controls", " an opponent controlled");
        format!("Whenever {victim} dealt damage this turn by {damager} dies")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::cause::EventCause;
    use crate::ids::{CardId, PlayerId};
    use crate::object::Object;
    use crate::provenance::ProvNodeId;
    use crate::snapshot::ObjectSnapshot;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        subtypes: Vec<Subtype>,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn record_damage(game: &mut crate::game_state::GameState, source: ObjectId, victim: ObjectId) {
        let target_snapshot = ObjectSnapshot::from_object(
            game.object(victim).expect("damage victim should exist"),
            game,
        );
        let event = TriggerEvent::new(
            DamageEvent::with_cause(
                source,
                DamageTarget::Object(victim),
                1,
                false,
                EventCause::effect(),
            )
            .with_target_snapshot(target_snapshot),
            ProvNodeId::default(),
        );
        game.record_turn_history_event(&event);
    }

    fn dies_event(game: &crate::game_state::GameState, victim: ObjectId) -> TriggerEvent {
        let snapshot = ObjectSnapshot::from_object(
            game.object(victim).expect("dying creature should exist"),
            game,
        );
        TriggerEvent::new(
            ZoneChangeEvent::with_cause(
                victim,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::from_game_rule(),
                Some(snapshot),
            ),
            ProvNodeId::default(),
        )
    }

    #[test]
    fn filtered_damager_history_matches_source_characteristics_and_controller_at_damage_time() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let trigger_source = creature(&mut game, "Trigger Source", alice, vec![Subtype::Demon]);
        let alice_spider = creature(&mut game, "Alice Spider", alice, vec![Subtype::Spider]);
        let alice_cat = creature(&mut game, "Alice Cat", alice, vec![Subtype::Cat]);
        let bob_spider = creature(&mut game, "Bob Spider", bob, vec![Subtype::Spider]);
        let spider_victim = creature(&mut game, "Spider Victim", bob, vec![Subtype::Bear]);
        let cat_victim = creature(&mut game, "Cat Victim", bob, vec![Subtype::Bear]);
        let opponent_victim = creature(&mut game, "Opponent Victim", bob, vec![Subtype::Bear]);

        record_damage(&mut game, alice_spider, spider_victim);
        record_damage(&mut game, alice_cat, cat_victim);
        record_damage(&mut game, bob_spider, opponent_victim);
        game.move_object_by_effect(alice_spider, Zone::Graveyard)
            .expect("matching damager should leave after its damage is recorded");

        let trigger = DiesDamagedByFilteredSourceThisTurnTrigger::new(
            ObjectFilter::creature().other(),
            ObjectFilter::default()
                .with_subtype(Subtype::Spider)
                .you_control()
                .in_zone(Zone::Battlefield),
        );
        let ctx = TriggerContext::for_source(trigger_source, alice, &game);

        assert!(
            trigger.matches(&dies_event(&game, spider_victim), &ctx),
            "a creature damaged by your Spider should match from source LKI"
        );
        assert!(
            !trigger.matches(&dies_event(&game, cat_victim), &ctx),
            "damage from your non-Spider must not match"
        );
        assert!(
            !trigger.matches(&dies_event(&game, opponent_victim), &ctx),
            "damage from an opponent's Spider must not match `you controlled`"
        );
    }
}
