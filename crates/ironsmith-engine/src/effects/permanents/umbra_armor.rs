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
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let attached_to = game.object(self.aura).and_then(|aura| aura.attached_to);
            if let Some(permanent) = attached_to.and_then(|target| target.object_id()) { game.clear_damage(permanent); }
            let Some(receipt) = crate::events::processing::process_destroy_scoped(game, self.aura, Some(ctx.source), ctx, None)?
                else { return Ok(EffectOutcome::count(0)); };
            crate::events::processing::finish_destroy_receipts(game, ctx, EffectOutcome::resolved(), vec![receipt])
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() { *game = checkpoint; context_checkpoint.restore(ctx); }
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        result
    }
}
