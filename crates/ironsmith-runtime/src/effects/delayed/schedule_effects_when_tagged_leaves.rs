//! Schedule effects when tagged objects leave the battlefield.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;

use super::trigger_queue::{
    DelayedTriggerTemplate, DelayedWatcherIdentity, queue_delayed_from_template,
};

/// Determines which object should be treated as the source when the delayed
/// trigger resolves.
pub use ironsmith_core::TaggedLeavesAbilitySource;

/// Schedules one delayed trigger per tagged object:
/// "When that object leaves the battlefield, execute these effects."
pub type ScheduleEffectsWhenTaggedLeavesEffect =
    ironsmith_core::ScheduleEffectsWhenTaggedLeavesEffect<Effect>;

impl EffectExecutor for ScheduleEffectsWhenTaggedLeavesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;
        let Some(tagged) = ctx.get_tagged_all(&self.tag) else {
            return Ok(EffectOutcome::count(0));
        };

        let watched = tagged
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        let delayed = DelayedTriggerTemplate::new(
            Trigger::this_leaves_battlefield(),
            self.effects.clone(),
            true,
            controller_id,
        )
        .with_ability_source(match self.ability_source {
            TaggedLeavesAbilitySource::WatchedObject => None,
            TaggedLeavesAbilitySource::CurrentSource => Some(ctx.source),
        });
        let scheduled =
            queue_delayed_from_template(game, DelayedWatcherIdentity::per_object(watched), delayed);

        Ok(EffectOutcome::count(scheduled as i32))
    }
}
