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
                // CR 701.56b-style repetition: "they face that choice an
                // additional time" (The Valeyard) adds one full choice per
                // such ability controlled by an opponent of the chooser.
                let additional = game
                    .battlefield
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| game.are_opponents(game.controller_of(obj), chooser))
                    .flat_map(|obj| {
                        game.calculated_characteristics_arc(obj.id)
                            .map(|calc| calc.static_abilities.to_vec())
                            .unwrap_or_default()
                    })
                    .map(|ability| ability.additional_villainous_choices_for_opponents())
                    .fold(0u32, u32::saturating_add);
                let mut outcomes = Vec::new();
                for _ in 0..=additional {
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
                        ExecutionError::Impossible(
                            "Selected villainous mode is not legal".to_string(),
                        )
                    })?;

                    outcomes.extend(super::sequence::execute_ordered_children_with_outputs(
                        game,
                        ctx,
                        &mode.effects,
                    )?);
                }

                Ok(CompletedEffectOutputs::from_children(
                    outcomes,
                    EffectOutcome::aggregate_summing_counts,
                ))
            },
        )
    }
}
