//! Tag the triggering object's snapshot for later reference.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::TagTriggeringObjectEffect;

/// Effect that tags the object that caused the trigger.
impl EffectExecutor for TagTriggeringObjectEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn is_resolution_prelude(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let event = ctx.triggering_event.as_ref().ok_or_else(|| {
            ExecutionError::UnresolvableValue("missing triggering event".to_string())
        })?;

        let historical = event
            .downcast::<crate::events::zones::ZoneChangeEvent>()
            .filter(|event| {
                event.from == crate::zone::Zone::Battlefield
                    && event.to != crate::zone::Zone::Battlefield
            })
            .map(|event| {
                ctx.get_tagged_all(ironsmith_core::ZONE_CHANGE_GROUP_TAG)
                    .filter(|group| !group.is_empty())
                    .cloned()
                    .unwrap_or_else(|| event.snapshots().to_vec())
            });
        if let Some(historical) = historical {
            ctx.set_tagged_objects(
                format!("__pre_move_history__{}", self.tag.as_str()),
                historical,
            );
        }
        let event = ctx
            .triggering_event
            .as_ref()
            .expect("triggering event checked above");

        // A typed attachment trigger has two participants. Its ordinary
        // demonstrative is the recipient, never the Aura/Equipment itself.
        if matches!(
            event.kind(),
            crate::events::EventKind::ObjectBecameAttached
                | crate::events::EventKind::ObjectBecameUnattached
        ) && let Some(recipient) = ctx
            .get_tagged_all(ironsmith_core::tag::TRIGGER_ATTACHMENT_RECIPIENT_TAG)
            .cloned()
        {
            let count = recipient.len() as i32;
            set_triggering_object_tags(ctx, self.tag.as_str(), recipient);
            return Ok(EffectOutcome::count(count));
        }

        if let Some(discard) = event.downcast::<crate::events::other::CardDiscardedEvent>() {
            let origins = ctx
                .get_tagged_all(ironsmith_core::ZONE_CHANGE_GROUP_TAG)
                .filter(|group| !group.is_empty())
                .cloned()
                .unwrap_or_else(|| discard.snapshot.iter().cloned().collect());
            let tagged = origins
                .into_iter()
                .map(|origin| {
                    discard
                        .destination(origin.object_id)
                        .and_then(|receipt| {
                            receipt
                                .object
                                .and_then(|id| game.object(id))
                                .filter(|object| {
                                    object.zone == receipt.zone
                                        && object.stable_id == origin.stable_id
                                })
                        })
                        .map(|object| {
                            ObjectSnapshot::from_object_with_calculated_characteristics(
                                object, game,
                            )
                        })
                        // Keep historical characteristics if the arrival has gone;
                        // movement helpers cannot substitute a later incarnation.
                        .unwrap_or(origin)
                })
                .collect::<Vec<_>>();
            let count = tagged.len() as i32;
            set_triggering_object_tags(ctx, self.tag.as_str(), tagged);
            return Ok(EffectOutcome::count(count));
        }

        if let Some(zone_change) = event.downcast::<crate::events::zones::ZoneChangeEvent>() {
            // CR 603.2c: a "one or more" trigger's "them" / "those cards" is
            // every object of the simultaneous event that matched it, not the
            // first event's objects (nor every object of a batch).
            if let Some(group) = ctx
                .get_tagged_all(ironsmith_core::ZONE_CHANGE_GROUP_TAG)
                .filter(|group| !group.is_empty())
                .cloned()
            {
                let tagged = group
                    .into_iter()
                    .filter_map(|snapshot| group_member_tag(game, zone_change, snapshot))
                    .collect::<Vec<_>>();
                let count = tagged.len() as i32;
                set_triggering_object_tags(ctx, self.tag.as_str(), tagged);
                return Ok(EffectOutcome::count(count));
            }
            if !zone_change.result_objects.is_empty() {
                let tagged = explicit_destination_snapshots(game, zone_change);
                if !tagged.is_empty() {
                    let count = tagged.len() as i32;
                    set_triggering_object_tags(ctx, self.tag.as_str(), tagged);
                    return Ok(EffectOutcome::count(count));
                }
                if zone_change.from == crate::zone::Zone::Battlefield
                    && let Some(snapshot) = zone_change.snapshot.as_ref()
                {
                    // Preserve the departed permanent's characteristics even
                    // after every explicit destination object has left. The
                    // old id cannot redirect object-moving effects to a later
                    // incarnation of the same card.
                    set_triggering_object_tags(ctx, self.tag.as_str(), vec![snapshot.clone()]);
                    return Ok(EffectOutcome::count(1));
                }
                set_triggering_object_tags(ctx, self.tag.as_str(), Vec::new());
                return Ok(EffectOutcome::count(0));
            }

            let Some(snapshot) = zone_change.snapshot.as_ref() else {
                return Ok(EffectOutcome::count(0));
            };
            if zone_change.from == crate::zone::Zone::Battlefield {
                if let Some(destination_id) = game
                    .find_object_by_stable_id(snapshot.stable_id)
                    .and_then(|id| {
                        game.object(id)
                            .filter(|obj| obj.zone == zone_change.to)
                            .map(|_| id)
                    })
                {
                    let mut tagged = snapshot.clone();
                    tagged.object_id = destination_id;
                    set_triggering_object_tags(ctx, self.tag.as_str(), vec![tagged]);
                    return Ok(EffectOutcome::count(1));
                }
                // A battlefield-departure trigger is allowed to use the
                // event's last-known information even when the destination
                // object is no longer available (for example, a token that
                // ceased to exist before the triggered ability resolved).
                // Keep the old object id in that case: characteristic values
                // and ownership are still authoritative, while effects that
                // require the new destination object will fail their normal
                // live-object lookup.
                set_triggering_object_tags(ctx, self.tag.as_str(), vec![snapshot.clone()]);
                return Ok(EffectOutcome::count(1));
            }

            if zone_change.to == crate::zone::Zone::Battlefield {
                // Entry events without explicit result_objects identify their
                // destination objects in objects. Preserve those incarnations:
                // a later blink of the same physical card is a different object.
                let tagged: Vec<_> = zone_change
                    .objects
                    .iter()
                    .filter_map(|&id| {
                        game.object(id)
                            .filter(|object| object.zone == zone_change.to)
                            .map(|object| {
                                ObjectSnapshot::from_object_with_calculated_characteristics(
                                    object, game,
                                )
                            })
                            .or_else(|| latest_zone_lki_snapshot(game, id, zone_change.to))
                    })
                    .collect();
                let count = tagged.len() as i32;
                set_triggering_object_tags(ctx, self.tag.as_str(), tagged);
                return Ok(EffectOutcome::count(count));
            }

            // The event's objects are its recorded arrivals when no explicit
            // result list exists. A stable-id lookup would resurrect a later
            // incarnation that left and re-entered the same destination zone.
            let tagged: Vec<_> = zone_change
                .destination_objects()
                .iter()
                .filter_map(|&id| {
                    game.object(id)
                        .filter(|object| object.zone == zone_change.to)
                        .map(|object| {
                            ObjectSnapshot::from_object_with_calculated_characteristics(
                                object, game,
                            )
                        })
                        .or_else(|| zone_change.destination_snapshot(id).cloned())
                        .or_else(|| latest_zone_lki_snapshot(game, id, zone_change.to))
                })
                .collect();
            let count = tagged.len() as i32;
            set_triggering_object_tags(ctx, self.tag.as_str(), tagged);
            return Ok(EffectOutcome::count(count));
        }

        // CR 603.2c: "whenever one or more creatures attack you, those
        // creatures ..." / "whenever you discard one or more cards, exile
        // them" name the whole matched group of the simultaneous event, not
        // the one event object that fired the trigger.
        let group_tag = if event
            .downcast::<crate::events::combat::CreatureAttackedEvent>()
            .is_some()
        {
            Some(ironsmith_core::ATTACKING_GROUP_TAG)
        } else if matches!(
            event.kind(),
            crate::events::EventKind::PermanentTapped | crate::events::EventKind::PermanentUntapped
        ) {
            Some(ironsmith_core::TAP_STATE_GROUP_TAG)
        } else if matches!(
            event.kind(),
            crate::events::EventKind::PermanentPhasedIn
                | crate::events::EventKind::PermanentPhasedOut
        ) {
            Some(ironsmith_core::tag::PHASING_GROUP_TAG)
        } else if event
            .downcast::<crate::events::other::CardDiscardedEvent>()
            .is_some()
        {
            Some(ironsmith_core::ZONE_CHANGE_GROUP_TAG)
        } else {
            None
        };
        if let Some(group) = group_tag
            .and_then(|tag| ctx.get_tagged_all(tag))
            .filter(|group| !group.is_empty())
            .cloned()
        {
            let count = group.len() as i32;
            set_triggering_object_tags(ctx, self.tag.as_str(), group);
            return Ok(EffectOutcome::count(count));
        }

        if let Some(sacrifice) = event.downcast::<crate::events::permanents::SacrificeEvent>()
            && let Some(snapshot) = sacrifice.snapshot.as_ref()
        {
            let tagged = game
                .find_object_by_stable_id(snapshot.stable_id)
                .and_then(|id| game.object(id))
                .filter(|obj| obj.zone == crate::zone::Zone::Graveyard)
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
            if let Some(tagged) = tagged {
                set_triggering_object_tags(ctx, self.tag.as_str(), vec![tagged]);
                return Ok(EffectOutcome::count(1));
            }
            set_triggering_object_tags(ctx, self.tag.as_str(), Vec::new());
            return Ok(EffectOutcome::count(0));
        }

        if let Some(keyword_action) = event.downcast::<crate::events::KeywordActionEvent>()
            && let Some(snapshots) = keyword_action.object_tags.get(&self.tag)
        {
            let tagged = snapshots.clone();
            let count = tagged.len() as i32;
            set_triggering_object_tags(ctx, self.tag.as_str(), tagged);
            return Ok(EffectOutcome::count(count));
        }

        // A player event ("whenever you gain life") or the non-object half of
        // a combined trigger ("when this enters or at the beginning of your
        // end step") has no triggering object: the prelude tags nothing, and
        // any reference to that object refers to nothing (CR 608.2c).
        let Some(object_id) = event.object_id() else {
            set_triggering_object_tags(ctx, self.tag.as_str(), Vec::new());
            return Ok(EffectOutcome::count(0));
        };

        if let Some(obj) = game.object(object_id) {
            set_triggering_object_tags(
                ctx,
                self.tag.as_str(),
                vec![ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                )],
            );
            return Ok(EffectOutcome::count(1));
        }

        if let Some(snapshot) = event.snapshot().or_else(|| {
            // A chapter-resolution event names its Saga as both object and
            // source. Its outer source LKI survives the final-chapter sacrifice.
            // Do not borrow another event's distinct damage/ability source.
            event
                .source_snapshot()
                .filter(|snapshot| snapshot.object_id == object_id)
        }) {
            // Zone-change events are handled above. For any other event (an
            // attack, a block, a tap, a discard) the event's object has left
            // since; keep its recorded identity and last known information
            // rather than rewriting it to the card's new incarnation, which is
            // a new object (CR 400.7). Combat relationships and other LKI
            // readers key off the recorded object, and a resolution that may
            // follow the card to its new zone does so through
            // `resolve_tagged_object_id`.
            set_triggering_object_tags(ctx, self.tag.as_str(), vec![snapshot.clone()]);
            return Ok(EffectOutcome::count(1));
        }

        Ok(EffectOutcome::count(0))
    }
}

/// The object a "one or more" zone-change trigger's group member names now:
/// the object it became in the destination zone, keeping a departed
/// permanent's last-known characteristics (CR 603.10a, 400.7).
fn group_member_tag(
    game: &GameState,
    zone_change: &crate::events::zones::ZoneChangeEvent,
    snapshot: ObjectSnapshot,
) -> Option<ObjectSnapshot> {
    let destination = game
        .find_object_by_stable_id(snapshot.stable_id)
        .and_then(|id| game.object(id))
        .filter(|object| object.zone == zone_change.to);
    if zone_change.from == crate::zone::Zone::Battlefield {
        let mut tagged = snapshot;
        if let Some(object) = destination {
            tagged.object_id = object.id;
        }
        return Some(tagged);
    }
    destination
        .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
}

fn latest_zone_lki_snapshot(
    game: &GameState,
    object_id: crate::ids::ObjectId,
    zone: crate::zone::Zone,
) -> Option<ObjectSnapshot> {
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .rev()
        .filter_map(|record| {
            record
                .event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
        })
        .flat_map(|event| event.snapshots())
        .find(|snapshot| snapshot.object_id == object_id && snapshot.zone == zone)
        .cloned()
}

fn explicit_destination_snapshots(
    game: &GameState,
    zone_change: &crate::events::zones::ZoneChangeEvent,
) -> Vec<ObjectSnapshot> {
    let destination_objects: Vec<_> = zone_change
        .result_objects
        .iter()
        .filter_map(|&id| {
            game.object(id)
                .filter(|obj| obj.zone == zone_change.to)
                .map(|obj| (id, obj))
        })
        .collect();

    if destination_objects.is_empty() {
        return Vec::new();
    }

    if zone_change.from == crate::zone::Zone::Battlefield
        && destination_objects.len() == 1
        && let Some(snapshot) = zone_change.snapshot.as_ref()
    {
        let mut tagged = snapshot.clone();
        tagged.object_id = destination_objects[0].0;
        return vec![tagged];
    }

    destination_objects
        .into_iter()
        .map(|(_, obj)| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .collect()
}

fn set_triggering_object_tags(
    ctx: &mut ExecutionContext,
    tag: &str,
    snapshots: Vec<ObjectSnapshot>,
) {
    ctx.set_tagged_objects(tag, snapshots.clone());
    if tag == "triggering" {
        ctx.set_tagged_objects("it", snapshots.clone());
        ctx.set_tagged_objects("__it__", snapshots);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId, StableId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    #[test]
    fn test_tag_triggering_object_uses_post_zone_change_object_id() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = game.new_object_id();
        let card = make_creature_card(creature_id.0 as u32, "Nine-Lives Familiar");
        let obj = Object::from_card(creature_id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );
        let graveyard_id = game
            .move_object_by_effect(creature_id, Zone::Graveyard)
            .expect("creature should move to graveyard");
        assert_ne!(graveyard_id, creature_id);

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                creature_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(snapshot.clone()),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let effect = TagTriggeringObjectEffect::new("triggering");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));

        let tagged = ctx
            .get_tagged("triggering")
            .expect("triggering tag should be present");
        assert_eq!(tagged.object_id, graveyard_id);
        assert_eq!(tagged.stable_id, snapshot.stable_id);
    }

    #[test]
    fn test_tag_triggering_object_uses_battlefield_lki_for_etb_object_that_died() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(creature_id.0 as u32), "Blitz Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 0))
            .build();
        game.add_object(Object::from_card(
            creature_id,
            &card,
            alice,
            Zone::Battlefield,
        ));

        let battlefield_snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );
        let mut etb_snapshot = battlefield_snapshot.clone();
        etb_snapshot.zone = Zone::Stack;
        etb_snapshot.power = Some(3);

        game.move_object_by_effect(creature_id, Zone::Graveyard)
            .expect("creature should move to graveyard");
        let death_record = crate::events::RawEvent::new(
            crate::events::zones::ZoneChangeEvent::with_cause(
                creature_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(battlefield_snapshot.clone()),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.turn_store.turn_history.record_event(
            &death_record,
            Some(battlefield_snapshot.clone()),
            None,
        );

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                creature_id,
                Zone::Stack,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                Some(etb_snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let effect = TagTriggeringObjectEffect::new("triggering");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));

        let tagged = ctx
            .get_tagged("triggering")
            .expect("triggering tag should use battlefield LKI");
        assert_eq!(tagged.stable_id, battlefield_snapshot.stable_id);
        assert_eq!(tagged.zone, Zone::Battlefield);
        assert_eq!(tagged.power, Some(5));
    }

    #[test]
    fn test_tag_triggering_object_does_not_retarget_after_destination_card_left_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = game.new_object_id();
        let card = make_creature_card(creature_id.0 as u32, "Restless Returned");
        let obj = Object::from_card(creature_id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );
        let graveyard_id = game
            .move_object_by_effect(creature_id, Zone::Graveyard)
            .expect("creature should move to graveyard");
        let battlefield_id = game
            .move_object_by_effect(graveyard_id, Zone::Battlefield)
            .expect("creature should return to battlefield");
        assert_ne!(battlefield_id, graveyard_id);

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_results(
                creature_id,
                vec![graveyard_id],
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let effect = TagTriggeringObjectEffect::new("triggering");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        let tagged = ctx
            .get_tagged("triggering")
            .expect("departure LKI remains available");
        assert_eq!(tagged.object_id, creature_id);
        assert_eq!(tagged.power, Some(1));
        assert_ne!(tagged.object_id, battlefield_id);
        assert!(game.object(tagged.object_id).is_none());
    }

    #[test]
    fn test_tag_triggering_object_keeps_battlefield_lki_without_destination_object() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(creature_id.0 as u32), "Vanished Victim")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 2))
            .build();
        game.add_object(Object::from_card(
            creature_id,
            &card,
            alice,
            Zone::Battlefield,
        ));
        let snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );

        // The event is authoritative even if its destination object is not
        // present in this resolution state.
        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                creature_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(snapshot.clone()),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let outcome = TagTriggeringObjectEffect::new("triggering")
            .execute(&mut game, &mut ctx)
            .expect("LKI tag should resolve");
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        let tagged = ctx
            .get_tagged("triggering")
            .expect("battlefield LKI should be tagged");
        assert_eq!(tagged.object_id, creature_id);
        assert_eq!(tagged.owner, alice);
        assert_eq!(tagged.power, Some(4));
        assert_eq!(tagged.zone, Zone::Battlefield);
    }

    #[test]
    fn test_tag_triggering_object_for_sacrifice_requires_card_still_in_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = game.new_object_id();
        let card = make_creature_card(creature_id.0 as u32, "Academy Rector");
        let obj = Object::from_card(creature_id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );
        let graveyard_id = game
            .move_object_by_effect(creature_id, Zone::Graveyard)
            .expect("creature should move to graveyard");
        game.move_object_by_effect(graveyard_id, Zone::Battlefield)
            .expect("creature should return to battlefield");

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::permanents::SacrificeEvent::new(creature_id, None)
                .with_snapshot(Some(snapshot), Some(alice)),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let effect = TagTriggeringObjectEffect::new("triggering");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(ctx.get_tagged("triggering").is_none());
    }

    #[test]
    fn test_tag_triggering_object_uses_all_split_meld_result_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let first = game.new_object_id();
        let first_card = make_creature_card(first.0 as u32, "Graf Rats");
        game.add_object(Object::from_card(
            first,
            &first_card,
            alice,
            Zone::Graveyard,
        ));

        let second = game.new_object_id();
        let second_card = make_creature_card(second.0 as u32, "Midnight Scavengers");
        game.add_object(Object::from_card(
            second,
            &second_card,
            alice,
            Zone::Graveyard,
        ));

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_results(
                ObjectId::from_raw(999),
                vec![first, second],
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(ObjectSnapshot {
                    chosen_subtype: None,
                    secret_chosen_subtype: None,
                    noted_life_total: None,
                    chosen_object: None,
                    object_id: ObjectId::from_raw(999),
                    stable_id: StableId::from(ObjectId::from_raw(999)),
                    kind: crate::object::ObjectKind::Card,
                    card: None,
                    controller: alice,
                    owner: alice,
                    name: "Chittering Host".to_string(),
                    first_printed_set_name: None,
                    mana_cost: None,
                    colors: crate::color::ColorSet::default(),
                    supertypes: Vec::new(),
                    card_types: vec![CardType::Creature],
                    subtypes: Vec::new(),
                    compiled_card_text: String::new(),
                    ability_labels: Vec::new(),
                    other_face: None,
                    other_face_name: None,
                    linked_face_layout: crate::card::LinkedFaceLayout::TransformLike,
                    linked_face_mana_value: None,
                    power: Some(5),
                    toughness: Some(6),
                    base_power: Some(5),
                    base_toughness: Some(6),
                    loyalty: None,
                    defense: None,
                    abilities: std::sync::Arc::new(Vec::new()),
                    aura_attach_filter: None,
                    copiable_values: crate::snapshot::CopiableValues::default(),
                    x_value: None,
                    cast_order_this_turn: None,
                    mana_spent_to_cast: crate::player::ManaPool::default(),
                    caster_mana_spent_to_cast: None,
                    mana_spent_on_x: None,
                    snow_mana_spent_to_cast: crate::player::ManaPool::default(),
                    mana_sources_spent_to_cast: Vec::new(),
                    optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
                    counters: std::collections::BTreeMap::new(),
                    is_token: false,
                    tapped: false,
                    attacking: false,
                    goaded: Some(false),
                    ring_bearer: None,
                    flipped: false,
                    face_down: false,
                    transform_count: 0,
                    attached_to: None,
                    attachments: Vec::new(),
                    attachment_snapshots: Vec::new(),
                    was_enchanted: false,
                    is_monstrous: false,
                    is_prepared: false,
                    is_commander: false,
                    zone: Zone::Battlefield,
                }),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let effect = TagTriggeringObjectEffect::new("triggering");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));

        let tagged = ctx
            .get_tagged_all("triggering")
            .expect("triggering tag should be present");
        let tagged_ids: Vec<_> = tagged.iter().map(|snapshot| snapshot.object_id).collect();
        assert_eq!(tagged_ids, vec![first, second]);
    }

    #[test]
    fn test_tag_triggering_object_preserves_lki_counters_for_battlefield_departure() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = game.new_object_id();
        let card = make_creature_card(creature_id.0 as u32, "Countered Departed");
        let mut obj = Object::from_card(creature_id, &card, alice, Zone::Battlefield);
        obj.counters
            .insert(crate::object::CounterType::PlusOnePlusOne, 2);
        game.add_object(obj);

        let snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );
        let graveyard_id = game
            .move_object_by_effect(creature_id, Zone::Graveyard)
            .expect("creature should move to graveyard");

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_results(
                creature_id,
                vec![graveyard_id],
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let effect = TagTriggeringObjectEffect::new("triggering");
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        let tagged = ctx
            .get_tagged("triggering")
            .expect("triggering tag should be present");
        assert_eq!(tagged.object_id, graveyard_id);
        assert_eq!(
            tagged
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied(),
            Some(2)
        );
    }
}
