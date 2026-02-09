//! Tag the triggering damage target object snapshot for later reference.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::events::DamageEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_event::DamageTarget;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that tags the object that was dealt damage in the triggering event.
#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringDamageTargetEffect {
    /// Tag name to store the damaged object snapshot under.
    pub tag: TagKey,
}

impl TagTriggeringDamageTargetEffect {
    /// Create a new effect that tags the triggering damage target object.
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

impl EffectExecutor for TagTriggeringDamageTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let event = ctx.triggering_event.as_ref().ok_or_else(|| {
            ExecutionError::UnresolvableValue("missing triggering event".to_string())
        })?;
        let Some(damage_event) = event.downcast::<DamageEvent>() else {
            return Ok(EffectOutcome::count(0));
        };
        let DamageTarget::Object(target_id) = damage_event.target else {
            return Ok(EffectOutcome::count(0));
        };
        let Some(target_obj) = game.object(target_id) else {
            return Ok(EffectOutcome::count(0));
        };

        ctx.tag_object(
            self.tag.clone(),
            ObjectSnapshot::from_object(target_obj, game),
        );
        Ok(EffectOutcome::from_result(EffectResult::Count(1)))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
