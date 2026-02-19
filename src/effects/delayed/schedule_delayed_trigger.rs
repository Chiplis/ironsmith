//! Schedule delayed trigger effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::{DelayedTrigger, Trigger};

/// Effect that schedules a delayed trigger.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleDelayedTriggerEffect {
    pub trigger: Trigger,
    pub effects: Vec<crate::effect::Effect>,
    pub one_shot: bool,
    pub start_next_turn: bool,
    pub until_end_of_turn: bool,
    pub target_objects: Vec<crate::ids::ObjectId>,
    pub target_tag: Option<TagKey>,
    pub target_filter: Option<ObjectFilter>,
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
            start_next_turn: false,
            until_end_of_turn: false,
            target_objects,
            target_tag: None,
            target_filter: None,
            controller,
        }
    }

    pub fn from_tag(
        trigger: Trigger,
        effects: Vec<crate::effect::Effect>,
        one_shot: bool,
        target_tag: impl Into<TagKey>,
        controller: PlayerFilter,
    ) -> Self {
        Self {
            trigger,
            effects,
            one_shot,
            start_next_turn: false,
            until_end_of_turn: false,
            target_objects: Vec::new(),
            target_tag: Some(target_tag.into()),
            target_filter: None,
            controller,
        }
    }

    pub fn with_target_filter(mut self, filter: ObjectFilter) -> Self {
        self.target_filter = Some(filter);
        self
    }

    pub fn starting_next_turn(mut self) -> Self {
        self.start_next_turn = true;
        self
    }

    pub fn until_end_of_turn(mut self) -> Self {
        self.until_end_of_turn = true;
        self
    }
}

impl EffectExecutor for ScheduleDelayedTriggerEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;

        if let Some(tag) = &self.target_tag {
            let Some(tagged) = ctx.get_tagged_all(tag) else {
                return Ok(EffectOutcome::count(0));
            };
            let filter_ctx = ctx.filter_context(game);
            let mut matched = 0i32;
            for snapshot in tagged {
                if let Some(filter) = &self.target_filter
                    && !filter.matches_snapshot(snapshot, &filter_ctx, game)
                {
                    continue;
                }
                let delayed = DelayedTrigger {
                    trigger: self.trigger.clone(),
                    effects: self.effects.clone(),
                    one_shot: self.one_shot,
                    not_before_turn: if self.start_next_turn {
                        Some(game.turn.turn_number.saturating_add(1))
                    } else {
                        None
                    },
                    expires_at_turn: if self.until_end_of_turn {
                        Some(game.turn.turn_number)
                    } else {
                        None
                    },
                    target_objects: vec![snapshot.object_id],
                    controller: controller_id,
                };
                game.delayed_triggers.push(delayed);
                matched += 1;
            }
            return Ok(EffectOutcome::count(matched));
        }

        let delayed = DelayedTrigger {
            trigger: self.trigger.clone(),
            effects: self.effects.clone(),
            one_shot: self.one_shot,
            not_before_turn: if self.start_next_turn {
                Some(game.turn.turn_number.saturating_add(1))
            } else {
                None
            },
            expires_at_turn: if self.until_end_of_turn {
                Some(game.turn.turn_number)
            } else {
                None
            },
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
