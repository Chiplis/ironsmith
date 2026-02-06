//! Tag the triggering object's snapshot for later reference.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that tags the object that caused the trigger.
#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringObjectEffect {
    /// Tag name to store the triggering object snapshot under.
    pub tag: TagKey,
}

impl TagTriggeringObjectEffect {
    /// Create a new effect that tags the triggering object.
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

impl EffectExecutor for TagTriggeringObjectEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let event = ctx.triggering_event.as_ref().ok_or_else(|| {
            ExecutionError::UnresolvableValue("missing triggering event".to_string())
        })?;

        let object_id = event.object_id().ok_or_else(|| {
            ExecutionError::UnresolvableValue("triggering event missing object".to_string())
        })?;

        if let Some(obj) = game.object(object_id) {
            ctx.tag_object(self.tag.clone(), ObjectSnapshot::from_object(obj, game));
            return Ok(EffectOutcome::from_result(EffectResult::Count(1)));
        }

        if let Some(snapshot) = event.snapshot() {
            ctx.tag_object(self.tag.clone(), snapshot.clone());
            return Ok(EffectOutcome::from_result(EffectResult::Count(1)));
        }

        Ok(EffectOutcome::count(0))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
