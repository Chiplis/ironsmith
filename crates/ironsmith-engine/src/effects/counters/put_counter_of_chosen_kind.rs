//! Choose a counter kind on a target and put another counter of that kind on it.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

pub use ironsmith_core::PutCounterOfChosenKindEffect;

use super::kind_selection::{choose_binding, counter_label, kinds_on, put_kind_on_recipients};

impl EffectExecutor for PutCounterOfChosenKindEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let mut program = super::ForEachCounterKindPutOrRemoveEffect::one_kind(self.target.clone());
        program.put_only = true;
        program.execute_child_with_outputs(game, ctx)
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
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
                    let kinds = kinds_on(game, &[id]);
                    counters.extend(kinds.into_iter().map(|kind| (id, kind)));
                }
                if counters.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }
                let options = counters
                    .iter()
                    .map(|(id, kind)| {
                        let name = game
                            .object(*id)
                            .map(|object| object.name.to_string())
                            .unwrap_or_default();
                        (
                            (*id, *kind),
                            format!("{name}: {} counter", counter_label(*kind)),
                        )
                    })
                    .collect::<Vec<_>>();
                let Some((chosen_object, kind)) =
                    choose_binding(game, ctx, "Choose a counter", &options, true)
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                put_kind_on_recipients(
                    game,
                    ctx,
                    kind,
                    &recipients,
                    self.exclude_kind_object.then_some(chosen_object),
                    self.only_if_absent,
                )
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.recipients.is_target().then_some(&self.recipients)
    }

    fn target_description(&self) -> &'static str {
        "target permanent"
    }
}
