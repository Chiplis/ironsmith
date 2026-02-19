//! Schedule effects when tagged objects leave the battlefield.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::triggers::{DelayedTrigger, Trigger};

/// Determines which object should be treated as the source when the delayed
/// trigger resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedLeavesAbilitySource {
    /// Use the watched tagged object as the source (default).
    WatchedObject,
    /// Use the object executing this scheduling effect as the source.
    CurrentSource,
}

/// Schedules one delayed trigger per tagged object:
/// "When that object leaves the battlefield, execute these effects."
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleEffectsWhenTaggedLeavesEffect {
    pub tag: TagKey,
    pub effects: Vec<Effect>,
    pub controller: PlayerFilter,
    pub ability_source: TaggedLeavesAbilitySource,
}

impl ScheduleEffectsWhenTaggedLeavesEffect {
    pub fn new(tag: impl Into<TagKey>, effects: Vec<Effect>, controller: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            effects,
            controller,
            ability_source: TaggedLeavesAbilitySource::WatchedObject,
        }
    }

    pub fn with_current_source_as_ability_source(mut self) -> Self {
        self.ability_source = TaggedLeavesAbilitySource::CurrentSource;
        self
    }
}

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

        let mut scheduled = 0i32;
        for snapshot in tagged {
            let delayed = DelayedTrigger {
                trigger: Trigger::this_leaves_battlefield(),
                effects: self.effects.clone(),
                one_shot: true,
                not_before_turn: None,
                expires_at_turn: None,
                target_objects: vec![snapshot.object_id],
                ability_source: match self.ability_source {
                    TaggedLeavesAbilitySource::WatchedObject => None,
                    TaggedLeavesAbilitySource::CurrentSource => Some(ctx.source),
                },
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

