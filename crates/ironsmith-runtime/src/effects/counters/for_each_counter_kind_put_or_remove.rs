//! For each counter kind on target, choose put or remove one.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, PutCountersEffect, RemoveCountersEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// For each distinct counter type on the target permanent, choose to either
/// put one counter of that type on it or remove one from it.
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachCounterKindPutOrRemoveEffect {
    pub target: ChooseSpec,
    pub all_kinds: bool,
}

impl ForEachCounterKindPutOrRemoveEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            all_kinds: true,
        }
    }

    pub fn one_kind(target: ChooseSpec) -> Self {
        Self {
            target,
            all_kinds: false,
        }
    }

    fn counter_label(counter_type: CounterType) -> String {
        format!("{counter_type:?}").to_ascii_lowercase()
    }

    fn choose_counter_kind(
        &self,
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
                    format!("Choose {} counter", Self::counter_label(*counter_type)),
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
}

impl EffectExecutor for ForEachCounterKindPutOrRemoveEffect {
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

            let selected_kinds = if self.all_kinds {
                counter_kinds
            } else {
                let Some(counter_type) = self.choose_counter_kind(game, ctx, &counter_kinds) else {
                    return Ok(EffectOutcome::count(0));
                };
                vec![counter_type]
            };

            for counter_type in selected_kinds {
                let label = Self::counter_label(counter_type);
                let options = vec![
                    SelectableOption::new(0, format!("Put one {label} counter on it")),
                    SelectableOption::new(1, format!("Remove one {label} counter from it")),
                ];
                let choice_ctx = SelectOptionsContext::new(
                    ctx.controller,
                    Some(ctx.source),
                    format!("Choose for {label} counter"),
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
                    return Ok(EffectOutcome::count(0));
                }
                let Some(choice) = choice.filter(|idx| *idx <= 1) else {
                    return Ok(EffectOutcome::count(0));
                };

                let spec = ChooseSpec::SpecificObject(target_id);
                let outcome = if choice == 1 {
                    RemoveCountersEffect::new(counter_type, 1, spec).execute(game, ctx)?
                } else {
                    PutCountersEffect::new(counter_type, 1, spec).execute(game, ctx)?
                };
                outcomes.push(outcome);
            }
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }
}
