use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;

pub type MarkPlottedEffect = ironsmith_core::MarkPlottedEffect;

impl EffectExecutor for MarkPlottedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut objects = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) => Vec::new(),
            Err(err) => return Err(err),
        };
        if objects.is_empty()
            && let crate::target::ChooseSpec::SpecificObject(old_id) = self.target
            && let Some(current_id) = game.current_object_id_after_zone_change(old_id)
        {
            objects.push(current_id);
        }
        if objects.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let mut events = Vec::new();
        for object_id in objects.iter().copied() {
            game.set_plotted(object_id, ctx.controller);
            let event_provenance = game.alloc_child_event_provenance(
                ctx.provenance,
                crate::events::EventKind::KeywordAction,
            );
            events.push(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(KeywordActionKind::Plot, ctx.controller, object_id, 1),
                event_provenance,
            ));
        }

        Ok(EffectOutcome::with_objects(objects).with_events(events))
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to mark plotted"
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::Standard
    }
}
