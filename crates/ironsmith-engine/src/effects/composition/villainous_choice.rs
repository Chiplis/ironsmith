//! Villainous choice effect implementation.

use crate::decisions::{ModesSpec, make_decision, specs::ModeOption};
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type VillainousChoiceEffect = ironsmith_core::VillainousChoiceEffect<crate::effect::Effect>;

impl EffectExecutor for VillainousChoiceEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for mode in &self.modes {
            for effect in &mode.effects {
                visitor(effect);
            }
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::compound::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                if self.modes.is_empty() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }

                let chooser = crate::effects::helpers::resolve_player_filter_as_chooser(
                    game,
                    &self.player,
                    ctx,
                )?;
                let mode_options = self
                    .modes
                    .iter()
                    .enumerate()
                    .map(|(idx, mode)| ModeOption::new(idx, mode.source_text.clone()))
                    .collect();
                let spec = ModesSpec::single(ctx.source, mode_options);
                let selected = make_decision(
                    game,
                    &mut ctx.decision_maker,
                    chooser,
                    Some(ctx.source),
                    spec,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let selected_idx = selected.first().copied().ok_or_else(|| {
                    ExecutionError::Impossible("No villainous mode selected".to_string())
                })?;
                let mode = self.modes.get(selected_idx).ok_or_else(|| {
                    ExecutionError::Impossible("Selected villainous mode is not legal".to_string())
                })?;

                let outcomes = super::sequence::execute_ordered_children_with_outputs(
                    game,
                    ctx,
                    &mode.effects,
                )?;

                Ok(CompletedEffectOutputs::from_children(
                    outcomes,
                    EffectOutcome::aggregate_summing_counts,
                ))
            },
        )
    }
}
