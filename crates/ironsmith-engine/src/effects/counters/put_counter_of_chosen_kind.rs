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
                    .execute_child(game, ctx)?,
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

pub use ironsmith_core::PutCounterOfKindChosenFromEffect;

impl EffectExecutor for PutCounterOfKindChosenFromEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::filter::ObjectFilterExt as _;

        // Recipients are fixed first: a declared target was chosen on the
        // stack, and "each" recipients are the matching objects now.
        let recipients = resolve_objects_for_effect(game, ctx, &self.recipients)?;

        // Every counter (object, kind) on the objects the source names.
        let filter_ctx = ctx.filter_context(game);
        let mut counters: Vec<(crate::ids::ObjectId, CounterType)> = Vec::new();
        for id in game.battlefield.iter().copied() {
            let Some(object) = game.object(id) else {
                continue;
            };
            if !self.kind_source.matches(object, &filter_ctx, game) {
                continue;
            }
            let mut kinds = object
                .counters
                .iter()
                .filter_map(|(counter_type, count)| (*count > 0).then_some(*counter_type))
                .collect::<Vec<_>>();
            kinds.sort_by_key(|counter_type| format!("{counter_type:?}"));
            counters.extend(kinds.into_iter().map(|kind| (id, kind)));
        }
        if counters.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        let options = counters
            .iter()
            .enumerate()
            .map(|(idx, (id, kind))| {
                let name = game
                    .object(*id)
                    .map(|object| object.name.to_string())
                    .unwrap_or_default();
                SelectableOption::new(idx, format!("{name}: {} counter", counter_label(*kind)))
            })
            .collect::<Vec<_>>();
        let choice_ctx = SelectOptionsContext::new(
            ctx.controller,
            Some(ctx.source),
            "Choose a counter".to_string(),
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
        let (chosen_object, kind) = choice
            .and_then(|idx| counters.get(idx).copied())
            .unwrap_or(counters[0]);

        let mut outcomes = Vec::new();
        for recipient in recipients {
            if self.exclude_kind_object && recipient == chosen_object {
                continue;
            }
            if self.only_if_absent
                && game
                    .object(recipient)
                    .is_some_and(|object| object.counters.get(&kind).copied().unwrap_or(0) > 0)
            {
                continue;
            }
            outcomes.push(
                PutCountersEffect::new(kind, 1, ChooseSpec::SpecificObject(recipient))
                    .execute_child(game, ctx)?,
            );
        }
        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.recipients.is_target().then_some(&self.recipients)
    }

    fn target_description(&self) -> &'static str {
        "target permanent"
    }
}
