use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
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
        let attached_to = game.object(self.aura).and_then(|aura| aura.attached_to);
        if let Some(permanent) = attached_to.and_then(|target| target.object_id()) {
            game.clear_damage(permanent);
        }

        let _ = crate::event_processor::process_destroy(game, self.aura, None, ctx.decision_maker);

        Ok(EffectOutcome::resolved())
    }
}
