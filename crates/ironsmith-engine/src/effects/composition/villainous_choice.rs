//! Villainous choice effect implementation.

use crate::decisions::{ModesSpec, make_decision, specs::ModeOption};
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, execute_effect};
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
        if self.modes.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.player, ctx)?;
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
            return Ok(EffectOutcome::count(0));
        }

        let selected_idx = selected
            .first()
            .copied()
            .ok_or_else(|| ExecutionError::Impossible("No villainous mode selected".to_string()))?;
        let mode = self.modes.get(selected_idx).ok_or_else(|| {
            ExecutionError::Impossible("Selected villainous mode is not legal".to_string())
        })?;

        let mut outcomes = Vec::new();
        for effect in &mode.effects {
            outcomes.push(execute_effect(game, effect, ctx)?);
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }
}
