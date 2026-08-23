//! Choose a counter kind on a target and put another counter of that kind on it.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, PutCountersEffect};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

pub use ironsmith_core::PutCounterOfChosenKindEffect;

fn counter_label(counter_type: CounterType) -> String {
    format!("{counter_type:?}").to_ascii_lowercase()
}

fn choose_counter_kind(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    counter_kinds: &[CounterType],
) -> Option<CounterType> {
    let options = counter_kinds
        .iter()
        .enumerate()
        .map(|(idx, counter_type)| {
            SelectableOption::new(
                idx,
                format!("Choose {} counter", counter_label(*counter_type)),
            )
        })
        .collect::<Vec<_>>();
    let choice_ctx = SelectOptionsContext::new(
        ctx.controller,
        Some(ctx.source),
        "Choose a counter kind".to_string(),
        options,
        1,
        1,
    );
    let choice = ctx
        .decision_maker
        .decide_options(game, &choice_ctx)
        .into_iter()
        .next();
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    choice.and_then(|idx| counter_kinds.get(idx).copied())
}

impl EffectExecutor for PutCounterOfChosenKindEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if target_ids.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcomes = Vec::new();
        for target_id in target_ids {
            let mut counter_kinds = game
                .object(target_id)
                .map(|object| {
                    object
                        .counters
                        .iter()
                        .filter_map(|(counter_type, count)| (*count > 0).then_some(*counter_type))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if counter_kinds.is_empty() {
                continue;
            }
            counter_kinds.sort_by_key(|counter_type| format!("{counter_type:?}"));

            let Some(counter_type) = choose_counter_kind(game, ctx, &counter_kinds) else {
                return Ok(EffectOutcome::count(0));
            };
            outcomes.push(
                PutCountersEffect::new(counter_type, 1, ChooseSpec::SpecificObject(target_id))
                    .execute(game, ctx)?,
            );
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target permanent"
    }
}
