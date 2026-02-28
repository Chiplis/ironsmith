//! Put counters effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_value};
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
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle Source target specially (for abilities like level-up that target themselves).
        let target_ids = match &self.target {
            ChooseSpec::Source => vec![ctx.source],
            _ => match resolve_objects_from_spec(game, &self.target, ctx) {
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
        for target_id in target_ids {
            let assigned_count = distributed_counts
                .as_ref()
                .and_then(|allocations| allocations.get(&target_id).copied())
                .unwrap_or(count);
            if assigned_count == 0 {
                continue;
            }
            // Process through replacement effects (e.g., Melira, Doubling Season).
            let final_count =
                process_put_counters_with_event(game, target_id, self.counter_type, assigned_count);
            if final_count == 0 {
                outcomes.push(EffectOutcome::from_result(EffectResult::Prevented));
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
                    outcomes.push(EffectOutcome::count(final_count as i32).with_event(event))
                }
                None => outcomes.push(EffectOutcome::from_result(EffectResult::TargetInvalid)),
            }
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
}
