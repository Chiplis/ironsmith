//! Schedule exile of previously tagged objects when this source leaves.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::{DelayedTrigger, Trigger};
use crate::zone::Zone;

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
            let delayed = DelayedTrigger {
                trigger: Trigger::this_leaves_battlefield(),
                effects: vec![Effect::move_to_zone(
                    ChooseSpec::SpecificObject(snapshot.object_id),
                    Zone::Exile,
                    true,
                )],
                one_shot: true,
                not_before_turn: None,
                expires_at_turn: None,
                target_objects: vec![ctx.source],
                ability_source: None,
                controller: controller_id,
            };
            game.delayed_triggers.push(delayed);
            scheduled += 1;
        }

        Ok(EffectOutcome::count(scheduled))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
