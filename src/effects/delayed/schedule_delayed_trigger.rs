//! Schedule delayed trigger effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::Trigger;

use super::trigger_queue::{
    DelayedTriggerTemplate, DelayedWatcherIdentity, queue_delayed_from_template,
};

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
                let mut tagged_objects = ctx.tagged_objects.clone();
                tagged_objects.insert(tag.clone(), vec![snapshot.clone()]);
                let delayed = DelayedTriggerTemplate::new(
                    self.trigger.clone(),
                    self.effects.clone(),
                    self.one_shot,
                    controller_id,
                )
                .with_x_value(ctx.x_value)
                .with_not_before_turn(if self.start_next_turn {
                    Some(game.turn.turn_number.saturating_add(1))
                } else {
                    None
                })
                .with_expires_at_turn(if self.until_end_of_turn {
                    Some(game.turn.turn_number)
                } else {
                    None
                })
                .with_tagged_objects(tagged_objects);
                queue_delayed_from_template(
                    game,
                    DelayedWatcherIdentity::combined(vec![snapshot.object_id]),
                    delayed,
                );
                matched += 1;
            }
            return Ok(EffectOutcome::count(matched));
        }

        let delayed = DelayedTriggerTemplate::new(
            self.trigger.clone(),
            self.effects.clone(),
            self.one_shot,
            controller_id,
        )
        .with_x_value(ctx.x_value)
        .with_not_before_turn(if self.start_next_turn {
            Some(game.turn.turn_number.saturating_add(1))
        } else {
            None
        })
        .with_expires_at_turn(if self.until_end_of_turn {
            Some(game.turn.turn_number)
        } else {
            None
        })
        .with_tagged_objects(ctx.tagged_objects.clone());
        queue_delayed_from_template(
            game,
            DelayedWatcherIdentity::combined(self.target_objects.clone()),
            delayed,
        );

        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Effect;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_schedule_delayed_trigger_captures_tagged_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let card = CardBuilder::new(CardId::from_raw(991), "Tagged Creature")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let graveyard_id = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let snapshot = ObjectSnapshot::from_object(
            game.object(graveyard_id)
                .expect("graveyard object should exist"),
            &game,
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("triggering", snapshot.clone());

        let effect = ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![Effect::new(
                crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
                    crate::target::ChooseSpec::Tagged("triggering".into()),
                    false,
                ),
            )],
            true,
            Vec::new(),
            PlayerFilter::You,
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("schedule should resolve");
        assert_eq!(outcome.result, crate::effect::EffectResult::Resolved);
        assert_eq!(game.delayed_triggers.len(), 1);

        let delayed = &game.delayed_triggers[0];
        let tagged = delayed
            .tagged_objects
            .get("triggering")
            .expect("captured triggering tag");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].object_id, snapshot.object_id);
    }
}
