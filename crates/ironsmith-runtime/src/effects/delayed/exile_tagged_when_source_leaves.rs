//! Schedule exile of previously tagged objects when this source leaves.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::Trigger;
use crate::zone::Zone;

use super::trigger_queue::{
    DelayedTriggerTemplate, DelayedWatcherIdentity, queue_delayed_from_template,
};

/// Schedules one delayed trigger per tagged object:
/// "When this source leaves the battlefield, exile that object."
#[derive(Debug, Clone, PartialEq)]
pub struct ExileTaggedWhenSourceLeavesEffect {
    pub tag: crate::tag::TagKey,
    pub controller: PlayerFilter,
}

impl ExileTaggedWhenSourceLeavesEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>, controller: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            controller,
        }
    }
}

impl EffectExecutor for ExileTaggedWhenSourceLeavesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;
        let Some(tagged) = ctx.get_tagged_all(&self.tag) else {
            return Ok(EffectOutcome::count(0));
        };

        let mut scheduled = 0i32;
        for snapshot in tagged {
            let delayed = DelayedTriggerTemplate::new(
                Trigger::this_leaves_battlefield(),
                vec![Effect::move_to_zone(
                    ChooseSpec::SpecificObject(snapshot.object_id),
                    Zone::Exile,
                    true,
                )],
                true,
                controller_id,
            );
            scheduled += queue_delayed_from_template(
                game,
                DelayedWatcherIdentity::combined(vec![ctx.source]),
                delayed,
            ) as i32;
        }

        Ok(EffectOutcome::count(scheduled))
    }
}
