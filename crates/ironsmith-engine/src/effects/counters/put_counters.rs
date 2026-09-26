//! Put counters effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{DistributeSpec, NumberSpec, make_decision_with_fallback};
use crate::effect::{ChoiceCount, EffectOutcome, ExecutionFact, Value};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::process_put_counters_with_event_with_dm;
use crate::filter::{FilterContext, ObjectFilterExt as _};
use crate::game_state::{GameState, Target};
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
pub use ironsmith_core::PutCountersEffect;

/// Effect that puts counters on a target permanent.
///
/// Supports replacement effects like Doubling Season and Hardened Scales.
///
/// # Fields
///
/// * `counter_type` - The type of counter to put
/// * `count` - How many counters to put
/// * `target` - Which permanent to target
/// * `target_count` - How many targets (for "up to" effects)
/// * `distributed` - If true, distribute total counters among chosen targets
///
/// # Example
///
/// ```ignore
/// // Put two +1/+1 counters on target creature
/// let effect = PutCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
impl EffectExecutor for PutCountersEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Counter placement on determined objects involves no choices; defer to commit so the
        // whole each-player action lands as one batch.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle Source target specially (for abilities like level-up that target themselves).
        let target_ids = match self.target.base() {
            ChooseSpec::Source => vec![ctx.source],
            _ => match resolve_objects_for_effect(game, ctx, &self.target) {
                Ok(objects) if !objects.is_empty() => objects,
                _ => {
                    // No target chosen (valid for "up to" effects).
                    return Ok(EffectOutcome::resolved());
                }
            },
        };

        let max_count = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        let amount_is_up_to = self
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::UpTo);
        let count = if amount_is_up_to {
            let description = format!(
                "Choose how many {} counters to put",
                self.counter_type.description()
            );
            let spec = NumberSpec::up_to(ctx.source, max_count, description);
            let chosen = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
                FallbackStrategy::Maximum,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            chosen.min(max_count)
        } else {
            max_count
        };
        if count == 0 {
            let outcome = EffectOutcome::count(0);
            return Ok(if amount_is_up_to {
                outcome.with_execution_fact(ExecutionFact::ChosenNumber(0))
            } else {
                outcome
            });
        }

        // CR 601.2d / 603.3d: the controller announces how distributed
        // counters are divided (at least one per target) when the targets
        // are chosen. A share announced for a target that has since become
        // illegal is lost (CR 608.2b). Without an announced division (a
        // non-targeted "distribute among" or a trigger whose division wasn't
        // announced) the controller divides them now.
        let distributed_counts: Option<Vec<(ObjectId, u32)>> = if self.distributed {
            let announced = if self.target.is_target() {
                ctx.take_target_distribution(&self.target)
            } else {
                None
            };
            let division: Vec<(Target, u32)> = if let Some(announced) = announced {
                announced.allocations
            } else if target_ids.len() == 1 {
                vec![(Target::Object(target_ids[0]), count)]
            } else {
                let min_per_target = if self.target.is_target()
                    && count as usize >= target_ids.len()
                {
                    1
                } else {
                    0
                };
                let spec = DistributeSpec::new(
                    ctx.source,
                    count,
                    target_ids.iter().copied().map(Target::Object).collect(),
                    min_per_target,
                );
                let division = make_decision_with_fallback(
                    game,
                    &mut ctx.decision_maker,
                    ctx.controller,
                    Some(ctx.source),
                    spec,
                    FallbackStrategy::Maximum,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                division
            };
            // Keep the allocations in target order so counters land in the
            // same order on every peer; never place more than the total.
            let mut remaining = count;
            let mut allocations: Vec<(ObjectId, u32)> = Vec::new();
            for target_id in &target_ids {
                let share: u32 = division
                    .iter()
                    .filter(|(target, _)| *target == Target::Object(*target_id))
                    .map(|(_, amount)| *amount)
                    .sum();
                let share = share.min(remaining);
                remaining -= share;
                allocations.push((*target_id, share));
            }
            Some(allocations)
        } else {
            None
        };

        let mut outcomes = Vec::with_capacity(target_ids.len());
        let mut affected_objects = Vec::new();
        // Counters put on several objects by one instruction are one event
        // for "one or more ... on one or more ..." triggers (CR 603.2c).
        let mut counter_batch: Option<crate::provenance::ProvNodeId> = None;
        for target_id in target_ids {
            let assigned_count = distributed_counts
                .as_ref()
                .map(|allocations| {
                    allocations
                        .iter()
                        .find(|(id, _)| *id == target_id)
                        .map_or(0, |(_, amount)| *amount)
                })
                .unwrap_or(count);
            if assigned_count == 0 {
                continue;
            }
            // Process through replacement effects (e.g., Melira, Doubling Season).
            let final_count = if ctx.replacement.entry_counter_source == Some(target_id) {
                assigned_count
            } else {
                process_put_counters_with_event_with_dm(
                    game,
                    target_id,
                    self.counter_type,
                    assigned_count,
                    ctx.cause.clone(),
                    &mut *ctx.decision_maker,
                )
            };
            if ctx.decision_maker.awaiting_choice() {
                // A counter-replacement order choice is pending.
                return Ok(EffectOutcome::count(0));
            }
            if final_count == 0 {
                outcomes.push(EffectOutcome::prevented());
                continue;
            }

            // Use centralized method which handles counter addition, timestamp recording, and event creation.
            match game
                .add_counters_with_source(
                    target_id,
                    self.counter_type,
                    final_count,
                    Some(ctx.source),
                    Some(ctx.controller),
                )
                .map(|event| {
                    let batch = *counter_batch.get_or_insert_with(|| {
                        game.alloc_child_event_provenance(
                            ctx.provenance,
                            crate::events::EventKind::MarkersChanged,
                        )
                    });
                    event.with_simultaneous_batch(batch)
                }) {
                Some(event) => {
                    affected_objects.push(target_id);
                    outcomes.push(EffectOutcome::count(final_count as i32).with_event(event))
                }
                None => outcomes.push(EffectOutcome::target_invalid()),
            }
        }

        let mut outcome = EffectOutcome::aggregate(outcomes);
        if !affected_objects.is_empty() {
            outcome = outcome.with_affected_objects_from_game(game, affected_objects);
        }
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target for counters"
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.target_count
    }

    fn get_target_distribution_value(&self) -> Option<&Value> {
        // CR 601.2d: "distribute N counters among ... targets" is divided
        // when the targets are announced, at least one per target.
        (self.distributed && self.target.is_target()).then_some(&self.amount)
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.target.base(), ChooseSpec::Source)
            && let Value::Fixed(count) = self.amount
        {
            return Some(if count == 1 {
                format!(
                    "Put a {} counter on this source",
                    self.counter_type.description()
                )
            } else {
                format!(
                    "Put {} {} counters on this source",
                    count,
                    self.counter_type.description()
                )
            });
        }
        None
    }
}

impl CostExecutableEffect for PutCountersEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if self.target.is_target() {
            return Err(crate::effects::CostValidationError::Other(
                "a cost object must be chosen, not targeted".to_string(),
            ));
        }
        match self.target.base() {
            ChooseSpec::Source => {
                if game
                    .object(source)
                    .is_some_and(|obj| obj.zone == crate::zone::Zone::Battlefield)
                {
                    Ok(())
                } else {
                    Err(crate::effects::CostValidationError::Other(
                        "source must be on the battlefield".to_string(),
                    ))
                }
            }
            ChooseSpec::Object(filter) => {
                let filter_ctx = FilterContext::new(controller).with_source(source);
                if game.battlefield.iter().copied().any(|object_id| {
                    game.object(object_id)
                        .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                }) {
                    Ok(())
                } else {
                    Err(crate::effects::CostValidationError::Other(
                        "no valid object for the counter cost".to_string(),
                    ))
                }
            }
            _ => Err(crate::effects::CostValidationError::Other(
                "put-counters cost supports only source or one chosen object".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(object);
        id
    }

    #[test]
    fn test_put_counters_emits_affected_result_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_creature_on_battlefield(&mut game, "Bear", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(target)];

        let effect = PutCountersEffect::plus_one_counters(2, ChooseSpec::target_creature());
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.affected_objects(), Some([target].as_slice()));
        let memory = result
            .affected_object_memory()
            .expect("counter target memory should be recorded");
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].object_id, target);
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 2);
        assert_eq!(result.events.len(), 1);
    }

    #[test]
    fn put_up_to_counters_uses_chosen_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_creature_on_battlefield(&mut game, "Saga Token", alice);

        let mut dm = crate::decision::NumericInputDecisionMaker::from_strs(&["1"]);
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(target)];

        let effect = PutCountersEffect::new(
            CounterType::Lore,
            Value::Fixed(3).with_surface_hint(ironsmith_core::ValueSurfaceHint::UpTo),
            ChooseSpec::target_creature(),
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("up-to counter amount should resolve");

        assert_eq!(result.as_count(), Some(1));
        assert_eq!(game.counter_count(target, CounterType::Lore), 1);
    }

    #[test]
    fn chosen_object_counter_cost_is_legal_and_does_not_collapse_to_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(700), "Hatchet")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let target = create_creature_on_battlefield(&mut game, "Cost Bear", alice);
        let effect = PutCountersEffect::new(
            CounterType::MinusOneMinusOne,
            1,
            ChooseSpec::Object(
                crate::filter::ObjectFilter::creature()
                    .you_control()
                    .in_zone(Zone::Battlefield),
            ),
        );

        assert!(
            CostExecutableEffect::can_execute_as_cost(&effect, &game, source, alice).is_ok(),
            "the chosen creature, not the source, should make the cost legal"
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("chosen counter cost should execute");

        assert_eq!(outcome.as_count(), Some(1));
        assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), 1);
        assert_eq!(game.counter_count(source, CounterType::MinusOneMinusOne), 0);
    }
}
