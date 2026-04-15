//! Haunt exile effect: exiles the source card and schedules a delayed trigger
//! to fire the haunt card's effects when the targeted (haunted) creature dies.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::delayed::trigger_queue::{DelayedTriggerConfig, queue_delayed_trigger};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::triggers::Trigger;
use crate::zone::Zone;

/// Effect that exiles the source card "haunting" a target creature.
///
/// When executed, this effect:
/// 1. Exiles the source card (the haunt creature that just died, or the haunt spell)
/// 2. Schedules a one-shot delayed trigger watching the targeted creature:
///    when that creature dies, the embedded haunt effects fire.
#[derive(Debug, Clone, PartialEq)]
pub struct HauntExileEffect {
    /// The effects to execute when the haunted creature dies.
    pub haunt_effects: Vec<Effect>,
    /// The targeting choices for the haunt effects (e.g., "target player").
    pub haunt_choices: Vec<ChooseSpec>,
}

impl HauntExileEffect {
    pub fn new(haunt_effects: Vec<Effect>, haunt_choices: Vec<ChooseSpec>) -> Self {
        Self {
            haunt_effects,
            haunt_choices,
        }
    }
}

impl EffectExecutor for HauntExileEffect {
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
            .map_or(true, |obj| obj.zone != Zone::Battlefield)
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
