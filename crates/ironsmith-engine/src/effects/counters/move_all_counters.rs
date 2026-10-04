//! Move all counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt;
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;
pub use ironsmith_core::MoveAllCountersEffect;

fn source_counter_snapshot(ctx: &ExecutionContext<'_>) -> Option<Vec<(CounterType, u32)>> {
    if let Some(snapshot) = ctx.source_snapshot.as_ref() {
        return Some(
            snapshot
                .counters
                .iter()
                .map(|(ct, &count)| (*ct, count))
                .collect(),
        );
    }
    ctx.triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::zones::ZoneChangeEvent>())
        .and_then(|event| event.snapshot.as_ref())
        .filter(|snapshot| snapshot.object_id == ctx.source)
        .map(|snapshot| {
            snapshot
                .counters
                .iter()
                .map(|(ct, &count)| (*ct, count))
                .collect()
        })
}

fn source_reference_uses_lki(
    ctx: &ExecutionContext<'_>,
    from_id: crate::ids::ObjectId,
    current_zone: crate::zone::Zone,
) -> bool {
    ctx.source_snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.object_id != from_id || snapshot.zone != current_zone)
}

fn tagged_counter_snapshot(
    ctx: &ExecutionContext<'_>,
    tag: &crate::tag::TagKey,
    from_id: Option<crate::ids::ObjectId>,
) -> Option<Vec<(CounterType, u32)>> {
    let snapshots = ctx.get_tagged_all(tag)?;
    let snapshot = from_id
        .and_then(|id| snapshots.iter().find(|snapshot| snapshot.object_id == id))
        .or_else(|| snapshots.first())?;
    Some(
        snapshot
            .counters
            .iter()
            .map(|(ct, &count)| (*ct, count))
            .collect(),
    )
}

/// Effect that moves ALL counters of ALL types from one creature to another.
///
/// Used by Fate Transfer: "Move all counters from target creature onto another target creature."
///
/// # Fields
///
/// * `from` - Source creature (first target)
/// * `to` - Destination creature (second target)
///
/// # Example
///
/// ```ignore
/// // Move all counters from one creature to another
/// let effect = MoveAllCountersEffect::new(
///     ChooseSpec::creature(),
///     ChooseSpec::creature(),
/// );
/// ```
impl EffectExecutor for MoveAllCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let contextual_target_pair = if matches!(self.from.base(), ChooseSpec::Object(_))
                && matches!(self.to.base(), ChooseSpec::Object(_))
                && crate::game_loop::requires_target_selection(&self.from)
                && crate::game_loop::requires_target_selection(&self.to)
            {
                match super::assigned_counter_transfer_pair(ctx) {
                    Some((from_id, to_id)) => {
                        // Endpoint relations refer to preceding roles, never
                        // to the complete set containing the candidate itself.
                        let mut filter_ctx = ctx.filter_context(game);
                        filter_ctx.target_objects.clear();
                        let from_valid = match self.from.base() {
                            ChooseSpec::Object(filter) => game
                                .object(from_id)
                                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game)),
                            _ => false,
                        };
                        if let Some(from) = game.object(from_id) {
                            filter_ctx.target_objects.push(crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(from, game));
                        }
                        let to_valid = match self.to.base() {
                            ChooseSpec::Object(filter) => game
                                .object(to_id)
                                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game)),
                            _ => false,
                        };
                        if !from_valid || !to_valid {
                            return Ok(EffectOutcome::target_invalid());
                        }
                        Some((from_id, to_id))
                    }
                    None => return Ok(EffectOutcome::target_invalid()),
                }
            } else {
                None
            };

            let to_id = if let Some((_, to_id)) = contextual_target_pair {
                to_id
            } else {
                let Some(to_id) = resolve_objects_for_effect(game, ctx, &self.to)?
                    .first()
                    .copied()
                else {
                    return Ok(EffectOutcome::target_invalid());
                };
                to_id
            };

            let from_id = if let Some((from_id, _)) = contextual_target_pair {
                Some(from_id)
            } else {
                resolve_objects_for_effect(game, ctx, &self.from)?
                    .first()
                    .copied()
            };
            let from_is_source = matches!(self.from.base(), ChooseSpec::Source);
            let from_tag = match self.from.base() {
                ChooseSpec::Tagged(tag) => Some(tag),
                _ => None,
            };
            let counters_to_move: Vec<(CounterType, u32)> = if let Some(from_id) = from_id {
                if let Some(obj) = game.object(from_id) {
                    let tagged_snapshot = from_tag.and_then(|tag| {
                        ctx.get_tagged_all(tag).and_then(|snapshots| {
                            snapshots
                                .iter()
                                .find(|snapshot| snapshot.object_id == from_id)
                                .or_else(|| snapshots.first())
                        })
                    });
                    if from_is_source && source_reference_uses_lki(ctx, from_id, obj.zone) {
                        source_counter_snapshot(ctx).unwrap_or_default()
                    } else if let Some(snapshot) = tagged_snapshot
                        && snapshot.zone != obj.zone
                    {
                        snapshot
                            .counters
                            .iter()
                            .map(|(ct, &count)| (*ct, count))
                            .collect()
                    } else {
                        obj.counters
                            .iter()
                            .map(|(ct, &count)| (*ct, count))
                            .collect()
                    }
                } else if from_is_source {
                    source_counter_snapshot(ctx).unwrap_or_default()
                } else if let Some(tag) = from_tag {
                    tagged_counter_snapshot(ctx, tag, Some(from_id)).unwrap_or_default()
                } else {
                    return Ok(EffectOutcome::target_invalid());
                }
            } else if from_is_source {
                source_counter_snapshot(ctx).unwrap_or_default()
            } else if let Some(tag) = from_tag {
                tagged_counter_snapshot(ctx, tag, None).unwrap_or_default()
            } else {
                return Ok(EffectOutcome::target_invalid());
            };

            if counters_to_move.is_empty() {
                return Ok(EffectOutcome::count(0));
            }
            // CR 122.5: counters that can't be put onto the destination aren't
            // removed from the source either.
            let counters_to_move: Vec<(CounterType, u32)> = counters_to_move
                .into_iter()
                .filter(|(counter_type, _)| {
                    super::move_destination_can_receive_counters(game, to_id, *counter_type)
                })
                .collect();
            if counters_to_move.is_empty() {
                return Ok(EffectOutcome::count(0));
            }

            // Bind live movement versus historical placement once, before
            // replacement programs can change the source's incarnation/zone.
            // A live source that later departs must not turn into an LKI placement.
            let live_source = from_id.filter(|id| {
                game.object(*id).is_some_and(|obj| {
                    if from_is_source && source_reference_uses_lki(ctx, *id, obj.zone) {
                        return false;
                    }
                    from_tag.and_then(|tag| {
                        ctx.get_tagged_all(tag).and_then(|snapshots| snapshots.iter()
                            .find(|snapshot| snapshot.object_id == *id).or_else(|| snapshots.first()))
                    }).is_none_or(|snapshot| snapshot.zone == obj.zone)
                })
            }).and_then(|id| game.object(id).map(|object| (id, object.zone)))
                .filter(|_| self.remove_from_source);
            if self.remove_from_source && live_source.is_none() {
                return Ok(EffectOutcome::count(0));
            }
            if live_source.is_some_and(|(id, _)| id == to_id) {
                return Ok(EffectOutcome::count(0));
            }
            let mut total_moved = 0i64;
            let mut outcome = EffectOutcome::count(0);
            for (counter_type, count) in counters_to_move {
                let budget = if let Some((from_id, zone)) = live_source {
                    if game.is_phased_out(from_id)
                        || !game.object(from_id).is_some_and(|object| object.zone == zone)
                        || !super::move_destination_can_receive_counters(game, to_id, counter_type)
                    {
                        continue;
                    }
                    let budget = count.min(game.counter_count(from_id, counter_type));
                    if budget == 0 { continue; }
                    let removed = super::remove_moved_counters(game, ctx, from_id, counter_type, budget)?;
                    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                    outcome = EffectOutcome::aggregate([outcome, removed]);
                    budget
                } else {
                    // CR 122.8/122.9: historical counters are placement only.
                    count
                };
                if budget == 0 { continue; }
                total_moved = total_moved.checked_add(i64::from(budget)).ok_or_else(||
                    ExecutionError::InternalError("counter movement total exceeds the supported wide count range".into()))?;
                let placed = super::put_moved_counters(game, ctx, to_id, counter_type, budget)?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                outcome = EffectOutcome::aggregate([outcome, placed]);
            }

            outcome.set_value(crate::effect::OutcomeValue::Count(total_moved));
            Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.from)
    }

    fn target_description(&self) -> &'static str {
        "creature to move counters from"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
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
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_with_multiple_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        obj.counters.insert(CounterType::MinusOneMinusOne, 2);
        game.add_object(obj);
        id
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_move_all_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_multiple_counters(&mut game, "Source Creature", alice);
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5)); // 3 + 2

        let from_obj = game.object(from_id).unwrap();
        assert!(from_obj.counters.is_empty());

        let to_obj = game.object(to_id).unwrap();
        assert_eq!(to_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3));
        assert_eq!(
            to_obj.counters.get(&CounterType::MinusOneMinusOne),
            Some(&2)
        );
    }

    #[test]
    fn test_move_all_counters_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature(&mut game, "Source Creature", alice);
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_move_all_counters_adds_to_existing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_multiple_counters(&mut game, "Source Creature", alice);

        // Target already has some counters
        let to_id = game.new_object_id();
        let card = make_creature_card(to_id.0 as u32, "Target Creature");
        let mut to_obj = Object::from_card(to_id, &card, alice, Zone::Battlefield);
        to_obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        game.add_object(to_obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5)); // 3 + 2 moved

        let to_obj = game.object(to_id).unwrap();
        assert_eq!(to_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&4)); // 1 + 3
        assert_eq!(
            to_obj.counters.get(&CounterType::MinusOneMinusOne),
            Some(&2)
        );
    }

    #[test]
    fn test_move_all_counters_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_multiple_counters(&mut game, "Source Creature", alice);
        let source = game.new_object_id();

        // Only one target provided
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(from_id)]);

        let effect = MoveAllCountersEffect::between_creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn another_recipient_is_relative_to_source_endpoint_only() {
        for same_endpoint in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let from = create_creature_with_multiple_counters(&mut game, "From", alice);
            let to = if same_endpoint { from } else {
                create_creature_with_multiple_counters(&mut game, "To", alice)
            };
            if !same_endpoint { game.object_mut(to).unwrap().counters.clear(); }
            let mut recipient = crate::target::ObjectFilter::creature();
            recipient.other = true;
            let effect = MoveAllCountersEffect::new(
                ChooseSpec::Target(Box::new(ChooseSpec::Object(crate::target::ObjectFilter::creature()))),
                ChooseSpec::Target(Box::new(ChooseSpec::Object(recipient))));
            let source = game.new_object_id();
            let mut ctx = ExecutionContext::new_default(source, alice)
                .with_targets(vec![ResolvedTarget::Object(from), ResolvedTarget::Object(to)]);
            let out = effect.execute(&mut game, &mut ctx).unwrap();
            if same_endpoint {
                assert_eq!(out.status, crate::effect::OutcomeStatus::TargetInvalid);
                assert_eq!(game.counter_count(from, CounterType::PlusOnePlusOne), 3);
                assert_eq!(game.counter_count(from, CounterType::MinusOneMinusOne), 2);
            } else {
                assert_eq!(out.count_or_zero(), 5);
                assert_eq!(game.counter_count(from, CounterType::PlusOnePlusOne), 0);
                assert_eq!(game.counter_count(from, CounterType::MinusOneMinusOne), 0);
                assert_eq!(game.counter_count(to, CounterType::PlusOnePlusOne), 3);
                assert_eq!(game.counter_count(to, CounterType::MinusOneMinusOne), 2);
            }
        }
    }

    #[test]
    fn test_move_all_counters_clone_box() {
        let effect = MoveAllCountersEffect::between_creatures();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MoveAllCountersEffect"));
    }

    #[test]
    fn source_lki_counters_move_to_target_when_source_left_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Departed Source", alice);
        let target = create_creature(&mut game, "Counter Receiver", alice);
        game.add_counters(source, CounterType::PlusOnePlusOne, 2);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source exists"),
            &game,
        );
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("move source");

        let effect =
            MoveAllCountersEffect::put_referenced(ChooseSpec::Source, ChooseSpec::SpecificObject(target));
        let mut ctx = ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
        let outcome = effect.execute(&mut game, &mut ctx).expect("move counters");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.object(target)
                .expect("target exists")
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied(),
            Some(2)
        );
    }

    #[test]
    fn tagged_lki_counters_move_to_target_when_tagged_object_left_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Departed Source", alice);
        let target = create_creature(&mut game, "Counter Receiver", alice);
        game.add_counters(source, CounterType::PlusOnePlusOne, 2);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source exists"),
            &game,
        );
        let graveyard_id = game
            .move_object_by_effect(source, Zone::Graveyard)
            .expect("move source");
        assert_ne!(source, graveyard_id);

        let tag = crate::tag::TagKey::from("triggering");
        let mut tagged_snapshot = snapshot.clone();
        tagged_snapshot.object_id = graveyard_id;
        let effect = MoveAllCountersEffect::put_referenced(
            ChooseSpec::Tagged(tag.clone()),
            ChooseSpec::SpecificObject(target),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects(tag, vec![tagged_snapshot]);
        let outcome = effect.execute(&mut game, &mut ctx).expect("move counters");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.object(target)
                .expect("target exists")
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied(),
            Some(2)
        );
    }
}

#[cfg(test)]
mod same_object_snapshot_placement_tests {
    use super::*;
    #[test]
    fn departed_tagged_snapshot_can_place_on_the_current_object_without_moving() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Snapshot placement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let old = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.object_mut(old).unwrap().counters.insert(CounterType::Charge, 2);
        let mut snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(old).unwrap(), &game);
        let current = game.move_object_by_effect(old, crate::zone::Zone::Graveyard).unwrap();
        assert_ne!(old, current);
        snapshot.object_id = current;
        let tag = crate::tag::TagKey::from("departed-counter-source");
        let effect = crate::effect::Effect::new(MoveAllCountersEffect::put_referenced(
            ChooseSpec::Tagged(tag.clone()), ChooseSpec::SpecificObject(current)));
        let mut ctx = ExecutionContext::new_default(old, alice);
        ctx.set_tagged_objects(tag, vec![snapshot]);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(current, CounterType::Charge), 2,
            "placing former counters from a departed snapshot is not a same-object move");
        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), 1);
    }
}

#[cfg(test)]
mod actual_move_departed_source_tests {
    use super::*;
    fn departed(tagged: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Actual counter move source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let destination = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 2);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let departed = game.move_object_by_effect(source, crate::zone::Zone::Graveyard).unwrap();
        assert_ne!(source, departed);
        let tag = crate::tag::TagKey::from("actual-departed-move-source");
        let from = if tagged { ChooseSpec::Tagged(tag.clone()) } else { ChooseSpec::Source };
        let effect = crate::effect::Effect::new(MoveAllCountersEffect::new(from, ChooseSpec::SpecificObject(destination)));
        let mut ctx = ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot.clone());
        if tagged { ctx.set_tagged_objects(tag, vec![snapshot]); }
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(destination, CounterType::Charge), 0,
            "an instruction to move counters cannot silently become placement of historical counters when its source leaves the expected zone");
        assert_eq!(result.count_or_zero(), 0);
        assert_eq!(result.events_of_type::<crate::events::MarkersChangedEvent>().count(), 0);
        assert_eq!(game.counter_count(departed, CounterType::Charge), 0);
    }
    #[test] fn source_move_does_not_copy_departed_counters() { departed(false); }
    #[test] fn tagged_move_does_not_copy_departed_counters() { departed(true); }
}

#[cfg(test)]
mod explicit_counter_collection_placement_tests {
    use super::*;
    #[test]
    fn placement_from_live_reference_keeps_source_and_applies_placement_replacements() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Referenced counter collection")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let target = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge, 2);
        game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
            source, alice, crate::events::counters::matchers::WouldPutCountersMatcher::any(),
            crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        let effect = crate::effect::Effect::new(MoveAllCountersEffect::put_referenced(
            ChooseSpec::Source, ChooseSpec::SpecificObject(target)));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let out = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, CounterType::Charge), 2);
        assert_eq!(game.counter_count(target, CounterType::Charge), 4);
        let markers = out.events_of_type::<crate::events::MarkersChangedEvent>().collect::<Vec<_>>();
        assert_eq!(markers.len(), 1);
        assert!(markers[0].is_added());
        assert_eq!(markers[0].amount, 4);
    }
}
