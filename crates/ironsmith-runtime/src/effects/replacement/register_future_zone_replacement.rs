use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterFutureZoneReplacementEffect = ironsmith_core::RegisterFutureZoneReplacementEffect;

impl EffectExecutor for RegisterFutureZoneReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut matcher = crate::events::zones::matchers::WouldChangeZoneMatcher::new(
            self.filter.clone(),
            self.from_zone,
            self.to_zone,
        );
        if let Some(cause_filter) = self.cause_filter.clone() {
            matcher = matcher.with_cause_filter(cause_filter);
        }
        if self.require_cause_source_match {
            matcher = matcher.requiring_cause_source_match();
        }
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            matcher,
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
