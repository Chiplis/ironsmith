//! Tag the object attached to the source (equipment/aura) for later reference.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::TagAttachedToSourceEffect;

/// Effect that tags the object attached to the source.
impl EffectExecutor for TagAttachedToSourceEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn is_resolution_prelude(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // A leaves-the-battlefield ability resolves after its source has become
        // a new object in another zone.  In that case the source ID no longer
        // exists, so use the trigger's battlefield LKI to remember what the
        // Aura or Equipment was attached to as it left.
        let attached_target = game
            .object(ctx.source)
            .and_then(|source| source.attached_to)
            .or_else(|| {
                game.object(ctx.source)
                    .is_none()
                    .then(|| ctx.source_snapshot.as_ref()?.attached_to)
                    .flatten()
            });
        let Some(attached_target) = attached_target else {
            return Ok(EffectOutcome::count(0));
        };

        match attached_target {
            crate::object::AttachmentTarget::Object(attached_id) => {
                if let Some(obj) = game.object(attached_id) {
                    ctx.tag_object(self.tag.clone(), ObjectSnapshot::from_object(obj, game));
                    return Ok(EffectOutcome::count(1));
                }
            }
            crate::object::AttachmentTarget::Player(attached_player) => {
                ctx.tag_player(self.tag.clone(), attached_player);
                return Ok(EffectOutcome::count(1));
            }
        }

        Ok(EffectOutcome::count(0))
    }
}
