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

    fn as_resolution_prelude(&self) -> Option<&dyn crate::effects::ResolutionPreludeBinding> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::ResolutionPreludeBinding::bind_resolution_prelude(self, game, ctx)
    }
}

impl crate::effects::ResolutionPreludeBinding for TagAttachedToSourceEffect {
    fn bind_resolution_prelude(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // A leaves-the-battlefield ability resolves after its source has become
        // a new object in another zone.  In that case the source ID no longer
        // exists, so use the trigger's battlefield LKI to remember what the
        // Aura or Equipment was attached to as it left.
        let attached_target = crate::effects::helpers::source_attachment_target_with_lki(
            game,
            ctx.source,
            ctx.source_snapshot.as_ref(),
        );
        let Some(attached_target) = attached_target else {
            // A known detached source cannot inherit an older event's host.
            ctx.set_tagged_objects(self.tag.clone(), Vec::new());
            ctx.tagged_players.remove(self.tag.as_str());
            return Ok(EffectOutcome::count(0));
        };

        match attached_target {
            crate::object::AttachmentTarget::Object(attached_id) => {
                ctx.tagged_players.remove(self.tag.as_str());
                if let Some(obj) = game.object(attached_id) {
                    // A source has one current attachment host. Replace the
                    // retained binding, including packets seeded by an older
                    // trigger or an earlier prelude pass, rather than append.
                    ctx.set_tagged_objects(self.tag.clone(), vec![ObjectSnapshot::try_from_object_with_calculated_characteristics(obj, game)?]);
                    return Ok(EffectOutcome::count(1));
                }
                // Exact departure LKI is newer than the trigger-time packet:
                // a host can change types, or the Aura can move, while its
                // triggered ability waits. Never chase a physical stable ID.
                if let Some(snapshot) = game.source_departure_snapshot(attached_id) {
                    ctx.set_tagged_objects(self.tag.clone(), vec![snapshot.clone()]);
                    return Ok(EffectOutcome::count(1));
                }
                ctx.set_tagged_objects(self.tag.clone(), Vec::new());
                return Err(ExecutionError::IncompleteEvidence(
                    "an attachment host departed without its exact last-known snapshot".into(),
                ));
            }
            crate::object::AttachmentTarget::Player(attached_player) => {
                ctx.set_tagged_objects(self.tag.clone(), Vec::new());
                ctx.tag_player(self.tag.clone(), attached_player);
                return Ok(EffectOutcome::count(1));
            }
        }

    }
}
