//! Tag the source object from the triggering event for later reference.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::TagTriggeringSourceEffect;

impl EffectExecutor for TagTriggeringSourceEffect {
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
        let event = ctx.triggering_event.as_ref().ok_or_else(|| {
            ExecutionError::UnresolvableValue("missing triggering event".to_string())
        })?;
        let source_id = event.source_object().ok_or_else(|| {
            ExecutionError::UnresolvableValue("triggering event missing source".to_string())
        })?;
        let Some(source) = game.object(source_id) else {
            return Ok(EffectOutcome::count(0));
        };
        ctx.set_tagged_objects(
            self.tag.as_str(),
            vec![ObjectSnapshot::from_object_with_calculated_characteristics(
                source, game,
            )],
        );
        Ok(EffectOutcome::count(1))
    }
}
