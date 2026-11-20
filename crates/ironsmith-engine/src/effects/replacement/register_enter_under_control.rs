use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::ReplacementPriority;
use crate::events::zones::matchers::WouldEnterBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterEnterUnderControlReplacementEffect =
    ironsmith_core::RegisterEnterUnderControlReplacementEffect;

impl EffectExecutor for RegisterEnterUnderControlReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // "put that card onto the battlefield under your control": a filter
        // that names this resolution's target (is_target_object) is judged
        // in the prospective entry world, where the target context and the
        // pre-move zone are gone. Bind it to the targets it matches now
        // (CR 608.2b, 614.1c) so the entry under control still applies.
        let matchers = if self.filter.is_target_object {
            use crate::filter::ObjectFilterExt as _;
            let filter_ctx = ctx.filter_context(game);
            let mut bound = Vec::new();
            for target in &filter_ctx.target_objects {
                let Some(object) = game.object(target.object_id) else {
                    continue;
                };
                if object.stable_id != target.stable_id
                    || !self.filter.matches(object, &filter_ctx, game)
                {
                    continue;
                }
                bound.push(
                    WouldEnterBattlefieldMatcher::new(crate::target::ObjectFilter::specific(
                        object.id,
                    ))
                    .with_stable_id(object.stable_id),
                );
            }
            bound
        } else {
            vec![WouldEnterBattlefieldMatcher::new(self.filter.clone())]
        };

        let mut outcome = EffectOutcome::resolved();
        for matcher in matchers {
            let replacement = ReplacementEffect::with_matcher(
                ctx.source,
                ctx.controller,
                matcher,
                ReplacementAction::EnterUnderControl(ctx.controller),
            )
            .with_priority_override(ReplacementPriority::ControlChanging);

            outcome = ApplyReplacementEffect {
                effect: replacement,
                mode: self.mode,
            }
            .execute_child(game, ctx)?;
        }
        Ok(outcome)
    }
}
