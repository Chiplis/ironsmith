use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;

/// Replacement payload for Umbra armor.
///
/// When the enchanted permanent would be destroyed, instead clear all damage
/// from it and destroy the Aura that created the replacement effect.
#[derive(Debug, Clone, PartialEq)]
pub struct UmbraArmorEffect {
    pub aura: ObjectId,
}

impl UmbraArmorEffect {
    pub const fn new(aura: ObjectId) -> Self {
        Self { aura }
    }
}

impl EffectExecutor for UmbraArmorEffect {
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
                // The replaced destruction owns its permanent even if this
                // Aura already left in another part of the simultaneous event.
                let permanent = ctx.triggering_event.as_ref()
                    .and_then(|event| event.downcast::<crate::events::DestroyEvent>())
                    .map(|event| event.permanent)
                    .or_else(|| game.object(self.aura).and_then(|aura| aura.attached_to).and_then(|target| target.object_id()));
                let mut children = Vec::new();
                if let Some(permanent) = permanent {
                    children.push(
                        crate::effects::ClearDamageEffect::specific(permanent)
                            .execute_child_with_outputs(game, ctx)?,
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                }
                let Some(destroyed) = crate::effects::zones::execute_resolved_destroy_with_outputs(
                    game, ctx, self.aura,
                )?
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::with_primary_result(
                        EffectOutcome::count(0),
                        children,
                    ));
                };
                let summary = destroyed.outcome.summary_projection();
                children.push(destroyed);
                Ok(crate::effects::CompletedEffectOutputs::with_primary_result(
                    summary, children,
                ))
            },
        )
    }
}
