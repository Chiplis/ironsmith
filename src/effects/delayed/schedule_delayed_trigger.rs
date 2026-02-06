//! Schedule delayed trigger effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::triggers::{DelayedTrigger, Trigger};

/// Effect that schedules a delayed trigger.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleDelayedTriggerEffect {
    pub trigger: Trigger,
    pub effects: Vec<crate::effect::Effect>,
    pub one_shot: bool,
    pub target_objects: Vec<crate::ids::ObjectId>,
    pub controller: PlayerFilter,
}

impl ScheduleDelayedTriggerEffect {
    pub fn new(
        trigger: Trigger,
        effects: Vec<crate::effect::Effect>,
        one_shot: bool,
        target_objects: Vec<crate::ids::ObjectId>,
        controller: PlayerFilter,
    ) -> Self {
        Self {
            trigger,
            effects,
            one_shot,
            target_objects,
            controller,
        }
    }
}

impl EffectExecutor for ScheduleDelayedTriggerEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;

        let delayed = DelayedTrigger {
            trigger: self.trigger.clone(),
            effects: self.effects.clone(),
            one_shot: self.one_shot,
            target_objects: self.target_objects.clone(),
            controller: controller_id,
        };

        game.delayed_triggers.push(delayed);

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
