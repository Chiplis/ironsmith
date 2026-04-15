//! Tag the object attached to the source (equipment/aura) for later reference.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that tags the object attached to the source.
#[derive(Debug, Clone, PartialEq)]
pub struct TagAttachedToSourceEffect {
    /// Tag name to store the attached object's snapshot under.
    pub tag: TagKey,
}

impl TagAttachedToSourceEffect {
    /// Create a new effect that tags the object attached to the source.
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

impl EffectExecutor for TagAttachedToSourceEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source) = game.object(ctx.source) else {
            return Ok(EffectOutcome::count(0));
        };

        let Some(attached_target) = source.attached_to else {
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
