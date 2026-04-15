//! Put counters effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, ExecutionFact, Value};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::event_processor::process_put_counters_with_event;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::CounterType;
use crate::target::ChooseSpec;
use std::collections::HashMap;

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
#[derive(Debug, Clone, PartialEq)]
pub struct PutCountersEffect {
    /// The type of counter to put.
    pub counter_type: CounterType,
    /// How many counters to put.
    pub count: Value,
    /// Which permanent to target.
    pub target: ChooseSpec,
    /// How many targets. None defaults to exactly 1.
    pub target_count: Option<ChoiceCount>,
    /// Whether to distribute the total counter amount among chosen targets.
    pub distributed: bool,
}

impl PutCountersEffect {
    /// Create a new put counters effect.
    pub fn new(counter_type: CounterType, count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            counter_type,
            count: count.into(),
            target,
            target_count: None,
            distributed: false,
        }
    }

    /// Create a put counters effect with target count specification.
    pub fn with_target_count(mut self, target_count: ChoiceCount) -> Self {
        self.target_count = Some(target_count);
        self
    }

    /// Mark this as a distributed-counters effect.
    pub fn with_distributed(mut self, distributed: bool) -> Self {
        self.distributed = distributed;
        self
    }

    /// Create an effect that puts +1/+1 counters on target creature.
    pub fn plus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::PlusOnePlusOne, count, target)
    }

    /// Create an effect that puts -1/-1 counters on target creature.
    pub fn minus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::MinusOneMinusOne, count, target)
    }

    /// Create an effect that puts counters on the source.
    pub fn on_source(counter_type: CounterType, count: impl Into<Value>) -> Self {
        Self::new(counter_type, count, ChooseSpec::Source)
    }
}

impl EffectExecutor for PutCountersEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle Source target specially (for abilities like level-up that target themselves).
        let target_ids = match &self.target {
            ChooseSpec::Source => vec![ctx.source],
            _ => match resolve_objects_for_effect(game, ctx, &self.target) {
                Ok(objects) if !objects.is_empty() => objects,
                _ => {
                    // No target chosen (valid for "up to" effects).
                    return Ok(EffectOutcome::resolved());
                }
            },
        };

        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;
        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let distributed_counts: Option<HashMap<ObjectId, u32>> = if self.distributed {
            let mut allocations: HashMap<ObjectId, u32> = HashMap::new();
            let target_len = target_ids.len();
            if target_len > 0 {
                for idx in 0..count {
                    let target = target_ids[(idx as usize) % target_len];
                    *allocations.entry(target).or_insert(0) += 1;
                }
            }
            Some(allocations)
        } else {
            None
        };

        let mut outcomes = Vec::with_capacity(target_ids.len());
        let mut affected_objects = Vec::new();
        for target_id in target_ids {
            let assigned_count = distributed_counts
                .as_ref()
                .and_then(|allocations| allocations.get(&target_id).copied())
                .unwrap_or(count);
            if assigned_count == 0 {
                continue;
            }
            // Process through replacement effects (e.g., Melira, Doubling Season).
            let final_count = process_put_counters_with_event(
                game,
                target_id,
                self.counter_type,
                assigned_count,
                ctx.cause.clone(),
            );
            if final_count == 0 {
                outcomes.push(EffectOutcome::prevented());
                continue;
            }

            // Use centralized method which handles counter addition, timestamp recording, and event creation.
            match game.add_counters_with_source(
                target_id,
                self.counter_type,
                final_count,
                Some(ctx.source),
                Some(ctx.controller),
            ) {
                Some(event) => {
                    affected_objects.push(target_id);
                    outcomes.push(EffectOutcome::count(final_count as i32).with_event(event))
                }
                None => outcomes.push(EffectOutcome::target_invalid()),
            }
        }

        let mut outcome = EffectOutcome::aggregate(outcomes);
        if !affected_objects.is_empty() {
            outcome = outcome.with_execution_fact(ExecutionFact::AffectedObjects(affected_objects));
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

    fn cost_description(&self) -> Option<String> {
        if matches!(self.target, ChooseSpec::Source)
            && let Value::Fixed(count) = self.count
        {
            return Some(if count == 1 {
                format!("Put a {} counter on ~", self.counter_type.description())
            } else {
                format!(
                    "Put {} {} counters on ~",
                    count,
                    self.counter_type.description()
                )
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::ExecutionFact;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
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
    fn test_put_counters_emits_affected_objects_fact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_creature_on_battlefield(&mut game, "Bear", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::executor::ResolvedTarget::Object(target)];

        let effect = PutCountersEffect::plus_one_counters(2, ChooseSpec::target_creature());
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::AffectedObjects(vec![target]))
        );
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 2);
        assert_eq!(result.events.len(), 1);
    }
}

impl CostExecutableEffect for PutCountersEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if !matches!(self.target, ChooseSpec::Source) {
            return Err(crate::effects::CostValidationError::Other(
                "put-counters cost supports only source".to_string(),
            ));
        }
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
}
