//! Shared delayed-trigger queue primitives.

use std::collections::HashMap;

use crate::effect::Effect;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::{DelayedTrigger, Trigger};

/// Config used to enqueue a delayed trigger.
#[derive(Debug, Clone)]
pub(crate) struct DelayedTriggerConfig {
    pub trigger: Trigger,
    pub effects: Vec<Effect>,
    pub one_shot: bool,
    pub not_before_turn: Option<u32>,
    pub expires_at_turn: Option<u32>,
    pub target_objects: Vec<ObjectId>,
    pub ability_source: Option<ObjectId>,
    pub controller: PlayerId,
    pub x_value: Option<u32>,
    pub choices: Vec<crate::target::ChooseSpec>,
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
}

impl DelayedTriggerConfig {
    pub fn new(
        trigger: Trigger,
        effects: Vec<Effect>,
        one_shot: bool,
        target_objects: Vec<ObjectId>,
        controller: PlayerId,
    ) -> Self {
        Self {
            trigger,
            effects,
            one_shot,
            not_before_turn: None,
            expires_at_turn: None,
            target_objects,
            ability_source: None,
            controller,
            x_value: None,
            choices: Vec::new(),
            tagged_objects: HashMap::new(),
        }
    }

    pub fn with_not_before_turn(mut self, not_before_turn: Option<u32>) -> Self {
        self.not_before_turn = not_before_turn;
        self
    }

    pub fn with_expires_at_turn(mut self, expires_at_turn: Option<u32>) -> Self {
        self.expires_at_turn = expires_at_turn;
        self
    }

    pub fn with_ability_source(mut self, ability_source: Option<ObjectId>) -> Self {
        self.ability_source = ability_source;
        self
    }

    pub fn with_x_value(mut self, x_value: Option<u32>) -> Self {
        self.x_value = x_value;
        self
    }

    pub fn with_choices(mut self, choices: Vec<crate::target::ChooseSpec>) -> Self {
        self.choices = choices;
        self
    }

    pub fn with_tagged_objects(
        mut self,
        tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged_objects;
        self
    }
}

/// How watcher identity should be represented in delayed scheduling.
#[derive(Debug, Clone)]
pub(crate) enum DelayedWatcherIdentity {
    /// One delayed trigger that watches any object in this set.
    Combined(Vec<ObjectId>),
    /// One delayed trigger per watched object.
    PerObject(Vec<ObjectId>),
}

impl DelayedWatcherIdentity {
    pub fn combined(watchers: Vec<ObjectId>) -> Self {
        Self::Combined(watchers)
    }

    pub fn per_object(watchers: Vec<ObjectId>) -> Self {
        Self::PerObject(watchers)
    }
}

/// Trigger/effect policy template for delayed scheduling.
#[derive(Debug, Clone)]
pub(crate) struct DelayedTriggerTemplate {
    pub trigger: Trigger,
    pub effects: Vec<Effect>,
    pub one_shot: bool,
    pub not_before_turn: Option<u32>,
    pub expires_at_turn: Option<u32>,
    pub ability_source: Option<ObjectId>,
    pub controller: PlayerId,
    pub x_value: Option<u32>,
    pub choices: Vec<crate::target::ChooseSpec>,
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
}

impl DelayedTriggerTemplate {
    pub fn new(
        trigger: Trigger,
        effects: Vec<Effect>,
        one_shot: bool,
        controller: PlayerId,
    ) -> Self {
        Self {
            trigger,
            effects,
            one_shot,
            not_before_turn: None,
            expires_at_turn: None,
            ability_source: None,
            controller,
            x_value: None,
            choices: Vec::new(),
            tagged_objects: HashMap::new(),
        }
    }

    pub fn with_not_before_turn(mut self, not_before_turn: Option<u32>) -> Self {
        self.not_before_turn = not_before_turn;
        self
    }

    pub fn with_expires_at_turn(mut self, expires_at_turn: Option<u32>) -> Self {
        self.expires_at_turn = expires_at_turn;
        self
    }

    pub fn with_ability_source(mut self, ability_source: Option<ObjectId>) -> Self {
        self.ability_source = ability_source;
        self
    }

    pub fn with_x_value(mut self, x_value: Option<u32>) -> Self {
        self.x_value = x_value;
        self
    }

    #[allow(dead_code)]
    pub fn with_choices(mut self, choices: Vec<crate::target::ChooseSpec>) -> Self {
        self.choices = choices;
        self
    }

    pub fn with_tagged_objects(
        mut self,
        tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged_objects;
        self
    }
}

/// Push a delayed trigger onto the game queue.
pub(crate) fn queue_delayed_trigger(game: &mut GameState, config: DelayedTriggerConfig) {
    game.delayed_triggers.push(DelayedTrigger {
        trigger: config.trigger,
        effects: config.effects,
        one_shot: config.one_shot,
        x_value: config.x_value,
        not_before_turn: config.not_before_turn,
        expires_at_turn: config.expires_at_turn,
        target_objects: config.target_objects,
        ability_source: config.ability_source,
        controller: config.controller,
        choices: config.choices,
        tagged_objects: config.tagged_objects,
    });
}

/// Queue delayed trigger(s) using a shared template and watcher identity policy.
///
/// Returns how many delayed triggers were enqueued.
pub(crate) fn queue_delayed_from_template(
    game: &mut GameState,
    watchers: DelayedWatcherIdentity,
    template: DelayedTriggerTemplate,
) -> usize {
    match watchers {
        DelayedWatcherIdentity::Combined(target_objects) => {
            queue_delayed_trigger(
                game,
                DelayedTriggerConfig::new(
                    template.trigger,
                    template.effects,
                    template.one_shot,
                    target_objects,
                    template.controller,
                )
                .with_not_before_turn(template.not_before_turn)
                .with_expires_at_turn(template.expires_at_turn)
                .with_ability_source(template.ability_source)
                .with_x_value(template.x_value)
                .with_choices(template.choices)
                .with_tagged_objects(template.tagged_objects),
            );
            1
        }
        DelayedWatcherIdentity::PerObject(target_objects) => {
            let mut queued = 0usize;
            for watched in target_objects {
                queue_delayed_trigger(
                    game,
                    DelayedTriggerConfig::new(
                        template.trigger.clone(),
                        template.effects.clone(),
                        template.one_shot,
                        vec![watched],
                        template.controller,
                    )
                    .with_not_before_turn(template.not_before_turn)
                    .with_expires_at_turn(template.expires_at_turn)
                    .with_ability_source(template.ability_source)
                    .with_x_value(template.x_value)
                    .with_choices(template.choices.clone())
                    .with_tagged_objects(template.tagged_objects.clone()),
                );
                queued += 1;
            }
            queued
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::ids::PlayerId;
    use crate::target::ChooseSpec;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_queue_delayed_trigger_defaults() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let watched = game.new_object_id();

        let config = DelayedTriggerConfig::new(
            Trigger::this_leaves_battlefield(),
            vec![Effect::sacrifice_source()],
            true,
            vec![watched],
            alice,
        );
        queue_delayed_trigger(&mut game, config);

        assert_eq!(game.delayed_triggers.len(), 1);
        let delayed = &game.delayed_triggers[0];
        assert!(delayed.one_shot);
        assert_eq!(delayed.target_objects, vec![watched]);
        assert_eq!(delayed.controller, alice);
        assert_eq!(delayed.not_before_turn, None);
        assert_eq!(delayed.expires_at_turn, None);
        assert_eq!(delayed.ability_source, None);
    }

    #[test]
    fn test_queue_delayed_trigger_with_optional_fields() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let watched = game.new_object_id();
        let turn = game.turn.turn_number;

        let config = DelayedTriggerConfig::new(
            Trigger::end_of_combat(),
            vec![Effect::exile(ChooseSpec::SpecificObject(watched))],
            false,
            vec![watched],
            alice,
        )
        .with_not_before_turn(Some(turn + 1))
        .with_expires_at_turn(Some(turn))
        .with_ability_source(Some(source));
        queue_delayed_trigger(&mut game, config);

        assert_eq!(game.delayed_triggers.len(), 1);
        let delayed = &game.delayed_triggers[0];
        assert!(!delayed.one_shot);
        assert_eq!(delayed.not_before_turn, Some(turn + 1));
        assert_eq!(delayed.expires_at_turn, Some(turn));
        assert_eq!(delayed.ability_source, Some(source));
    }

    #[test]
    fn test_queue_delayed_from_template_combined_watchers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let watched_a = game.new_object_id();
        let watched_b = game.new_object_id();

        let template = DelayedTriggerTemplate::new(
            Trigger::this_leaves_battlefield(),
            vec![Effect::exile(ChooseSpec::SpecificObject(source))],
            true,
            alice,
        )
        .with_ability_source(Some(source));

        let queued = queue_delayed_from_template(
            &mut game,
            DelayedWatcherIdentity::combined(vec![watched_a, watched_b]),
            template,
        );

        assert_eq!(queued, 1);
        assert_eq!(game.delayed_triggers.len(), 1);
        let delayed = &game.delayed_triggers[0];
        assert_eq!(delayed.target_objects, vec![watched_a, watched_b]);
    }

    #[test]
    fn test_queue_delayed_from_template_per_object_watchers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let watched_a = game.new_object_id();
        let watched_b = game.new_object_id();

        let template = DelayedTriggerTemplate::new(
            Trigger::this_leaves_battlefield(),
            vec![Effect::sacrifice_source()],
            true,
            alice,
        );

        let queued = queue_delayed_from_template(
            &mut game,
            DelayedWatcherIdentity::per_object(vec![watched_a, watched_b]),
            template,
        );

        assert_eq!(queued, 2);
        assert_eq!(game.delayed_triggers.len(), 2);
        assert_eq!(game.delayed_triggers[0].target_objects, vec![watched_a]);
        assert_eq!(game.delayed_triggers[1].target_objects, vec![watched_b]);
    }
}
