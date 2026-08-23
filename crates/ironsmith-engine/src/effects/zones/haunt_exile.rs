//! Haunt exile effect: exiles the source card and schedules a delayed trigger
//! to fire the haunt card's effects when the targeted (haunted) creature dies.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::delayed::trigger_queue::{DelayedTriggerConfig, queue_delayed_trigger};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::triggers::Trigger;
use crate::zone::Zone;
pub type HauntExileEffect = ironsmith_core::HauntExileEffect<Effect>;

impl EffectExecutor for HauntExileEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.haunt_effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Get the target creature (the one being haunted) from resolved targets.
        let haunted_creature_id = ctx
            .targets
            .iter()
            .find_map(|t| {
                if let ResolvedTarget::Object(id) = t {
                    Some(*id)
                } else {
                    None
                }
            })
            .ok_or(ExecutionError::InvalidTarget)?;

        // Verify the haunted creature is still on the battlefield.
        if game
            .object(haunted_creature_id)
            .is_none_or(|obj| obj.zone != Zone::Battlefield)
        {
            return Ok(EffectOutcome::resolved());
        }

        // Exile the source card (the haunt card that just died / went to graveyard).
        let new_exiled_id = game.move_object_by_effect(ctx.source, Zone::Exile);
        let ability_source = new_exiled_id.unwrap_or(ctx.source);

        // Schedule a one-shot delayed trigger: when the haunted creature dies,
        // execute the haunt card's effects.
        queue_delayed_trigger(
            game,
            DelayedTriggerConfig::new(
                Trigger::this_dies(),
                self.haunt_effects.clone(),
                true, // one-shot
                vec![haunted_creature_id],
                ctx.controller,
            )
            .with_ability_source(Some(ability_source))
            .with_choices(self.haunt_choices.clone()),
        );

        Ok(EffectOutcome::resolved())
    }
}
