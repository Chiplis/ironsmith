use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterManaReplacementEffect = ironsmith_core::RegisterManaReplacementEffect;

impl EffectExecutor for RegisterManaReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                self.source_filter.clone(),
            ),
            ReplacementAction::ReplaceMana(self.replacement_mana.clone()),
        );

        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute(game, ctx)
    }
}
