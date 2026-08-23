use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith_core::DamagedBySource;

pub type RegisterDamagedBySourceZoneReplacementEffect =
    ironsmith_core::RegisterDamagedBySourceZoneReplacementEffect;

impl EffectExecutor for RegisterDamagedBySourceZoneReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            crate::events::zones::matchers::WouldDieDamagedBySourceThisTurnMatcher::new(
                self.filter.clone(),
                DamagedBySource::ThisCreature,
            ),
            ReplacementAction::ChangeDestination(self.replacement_zone),
        );

        match self.mode {
            crate::effects::ReplacementApplyMode::OneShot => {
                game.effect_store
                    .replacement_effects
                    .add_one_shot_effect(replacement);
            }
            crate::effects::ReplacementApplyMode::UntilEndOfTurn => {
                game.effect_store
                    .replacement_effects
                    .add_until_end_of_turn_effect(replacement);
            }
            crate::effects::ReplacementApplyMode::Resolution => {
                game.effect_store
                    .replacement_effects
                    .add_resolution_effect(replacement);
            }
        }

        Ok(EffectOutcome::from_status(OutcomeStatus::Succeeded))
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}
