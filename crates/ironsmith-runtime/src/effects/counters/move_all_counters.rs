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
        let contextual_target_pair = if ctx.target_assignments.is_empty()
            && !ctx.targets.is_empty()
            && matches!(self.from.base(), ChooseSpec::Object(_))
            && matches!(self.to.base(), ChooseSpec::Object(_))
            && !self.from.is_target()
            && !self.to.is_target()
        {
            match ctx.resolve_two_object_targets() {
                Some((from_id, to_id)) => {
                    let filter_ctx = ctx.filter_context(game);
                    let from_valid = match self.from.base() {
                        ChooseSpec::Object(filter) => game
                            .object(from_id)
                            .is_some_and(|obj| filter.matches(obj, &filter_ctx, game)),
                        _ => false,
                    };
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

        let mut total_moved = 0u32;
        let mut outcome = EffectOutcome::count(0);

        // Move each counter type using centralized methods
        for (counter_type, count) in counters_to_move {
            // Remove from source
            let removed = if let Some(from_id) = from_id.filter(|id| {
                game.object(*id).is_some_and(|obj| {
                    if from_is_source && source_reference_uses_lki(ctx, *id, obj.zone) {
                        return false;
                    }
                    from_tag
                        .and_then(|tag| {
                            ctx.get_tagged_all(tag).and_then(|snapshots| {
                                snapshots
                                    .iter()
                                    .find(|snapshot| snapshot.object_id == *id)
                                    .or_else(|| snapshots.first())
                            })
                        })
                        .is_none_or(|snapshot| snapshot.zone == obj.zone)
                })
            }) {
                if let Some((removed, remove_event)) = game.remove_counters(
                    from_id,
                    counter_type,
                    count,
                    Some(ctx.source),
                    Some(ctx.controller),
                ) {
                    outcome = outcome.with_event(remove_event);
                    removed
                } else {
                    0
                }
            } else {
                count
            };
            if removed == 0 {
                continue;
            }
            total_moved += removed;

            // Add to destination (only the amount actually removed)
            if let Some(add_event) = game.add_counters_with_source(
                to_id,
                counter_type,
                removed,
                Some(ctx.source),
                Some(ctx.controller),
            ) {
                outcome = outcome.with_event(add_event);
            }
        }

        outcome.set_value(crate::effect::OutcomeValue::Count(total_moved as i32));
        Ok(outcome)
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
            MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(target));
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
        let effect = MoveAllCountersEffect::new(
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
