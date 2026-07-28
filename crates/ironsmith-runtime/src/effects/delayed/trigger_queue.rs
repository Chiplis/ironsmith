//! Shared delayed-trigger queue primitives.

use std::collections::HashMap;

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::resolution::ResolutionProgram;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::{DelayedTrigger, Trigger};

pub(crate) fn tagged_collection_has_object_in_zone(
    game: &GameState,
    tagged_objects: &HashMap<TagKey, Vec<ObjectSnapshot>>,
    tag: &TagKey,
    zone: crate::zone::Zone,
) -> bool {
    tagged_objects.get(tag).is_some_and(|snapshots| {
        snapshots.iter().any(|snapshot| {
            game.find_object_by_stable_id(snapshot.stable_id)
                .or_else(|| game.object(snapshot.object_id).map(|_| snapshot.object_id))
                .and_then(|object_id| game.object(object_id))
                .is_some_and(|object| object.zone == zone)
        })
    })
}

/// Config used to enqueue a delayed trigger.
#[derive(Debug, Clone)]
pub struct DelayedTriggerConfig {
    pub trigger: Trigger,
    pub effects: ResolutionProgram,
    pub one_shot: bool,
    pub not_before_turn: Option<u32>,
    pub expires_at_turn: Option<u32>,
    /// Registration expires before events on its controller's first turn
    /// whose turn number is greater than this anchor.
    pub expires_before_controller_turn_after: Option<u32>,
    pub expires_at_end_of_combat: bool,
    pub while_any_tagged_object_in_zone: Option<(TagKey, crate::zone::Zone)>,
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
        effects: impl Into<ResolutionProgram>,
        one_shot: bool,
        target_objects: Vec<ObjectId>,
        controller: PlayerId,
    ) -> Self {
        Self {
            trigger,
            effects: effects.into(),
            one_shot,
            not_before_turn: None,
            expires_at_turn: None,
            expires_before_controller_turn_after: None,
            expires_at_end_of_combat: false,
            while_any_tagged_object_in_zone: None,
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

    pub fn with_expires_before_controller_turn_after(mut self, anchor_turn: Option<u32>) -> Self {
        self.expires_before_controller_turn_after = anchor_turn;
        self
    }

    pub fn with_expires_at_end_of_combat(mut self, expires: bool) -> Self {
        self.expires_at_end_of_combat = expires;
        self
    }

    pub fn while_any_tagged_object_in_zone_opt(
        mut self,
        duration: Option<(TagKey, crate::zone::Zone)>,
    ) -> Self {
        self.while_any_tagged_object_in_zone = duration;
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
    pub effects: ResolutionProgram,
    pub one_shot: bool,
    pub not_before_turn: Option<u32>,
    pub expires_at_turn: Option<u32>,
    pub expires_before_controller_turn_after: Option<u32>,
    pub expires_at_end_of_combat: bool,
    pub while_any_tagged_object_in_zone: Option<(TagKey, crate::zone::Zone)>,
    pub ability_source: Option<ObjectId>,
    pub controller: PlayerId,
    pub x_value: Option<u32>,
    pub choices: Vec<crate::target::ChooseSpec>,
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
}

impl DelayedTriggerTemplate {
    pub fn new(
        trigger: Trigger,
        effects: impl Into<ResolutionProgram>,
        one_shot: bool,
        controller: PlayerId,
    ) -> Self {
        Self {
            trigger,
            effects: effects.into(),
            one_shot,
            not_before_turn: None,
            expires_at_turn: None,
            expires_before_controller_turn_after: None,
            expires_at_end_of_combat: false,
            while_any_tagged_object_in_zone: None,
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

    pub fn with_expires_before_controller_turn_after(mut self, anchor_turn: Option<u32>) -> Self {
        self.expires_before_controller_turn_after = anchor_turn;
        self
    }

    pub fn with_expires_at_end_of_combat(mut self, expires: bool) -> Self {
        self.expires_at_end_of_combat = expires;
        self
    }

    pub fn while_any_tagged_object_in_zone_opt(
        mut self,
        duration: Option<(TagKey, crate::zone::Zone)>,
    ) -> Self {
        self.while_any_tagged_object_in_zone = duration;
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

    pub fn with_tagged_objects(
        mut self,
        tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged_objects;
        self
    }
}

/// Push a delayed trigger onto the game queue.
pub fn queue_delayed_trigger(game: &mut GameState, config: DelayedTriggerConfig) {
    let (ability_source_stable_id, ability_source_name, ability_source_snapshot) = config
        .ability_source
        .and_then(|source_id| {
            game.object(source_id).map(|object| {
                let mut snapshot =
                    ObjectSnapshot::from_object_with_calculated_characteristics(object, game);
                if game.object_has_static_ability_id(
                    source_id,
                    crate::static_abilities::StaticAbilityId::Lifelink,
                ) && !snapshot
                    .has_static_ability_id(crate::static_abilities::StaticAbilityId::Lifelink)
                {
                    std::sync::Arc::make_mut(&mut snapshot.abilities)
                        .push(crate::ability::lifelink());
                }
                (object.stable_id, object.name.to_string(), snapshot)
            })
        })
        .map(|(stable_id, name, snapshot)| (Some(stable_id), Some(name), Some(snapshot)))
        .unwrap_or((None, None, None));

    game.effect_store.delayed_triggers.push(DelayedTrigger {
        trigger: config.trigger,
        effects: config.effects,
        one_shot: config.one_shot,
        x_value: config.x_value,
        not_before_turn: config.not_before_turn,
        expires_at_turn: config.expires_at_turn,
        expires_before_controller_turn_after: config.expires_before_controller_turn_after,
        expires_at_end_of_combat: config.expires_at_end_of_combat,
        while_any_tagged_object_in_zone: config.while_any_tagged_object_in_zone,
        target_objects: config.target_objects,
        ability_source: config.ability_source,
        ability_source_stable_id,
        ability_source_name,
        ability_source_snapshot,
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
                .with_expires_before_controller_turn_after(
                    template.expires_before_controller_turn_after,
                )
                .with_expires_at_end_of_combat(template.expires_at_end_of_combat)
                .while_any_tagged_object_in_zone_opt(template.while_any_tagged_object_in_zone)
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
                    .with_expires_before_controller_turn_after(
                        template.expires_before_controller_turn_after,
                    )
                    .with_expires_at_end_of_combat(template.expires_at_end_of_combat)
                    .while_any_tagged_object_in_zone_opt(
                        template.while_any_tagged_object_in_zone.clone(),
                    )
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

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert!(delayed.one_shot);
        assert_eq!(delayed.target_objects, vec![watched]);
        assert_eq!(delayed.controller, alice);
        assert_eq!(delayed.not_before_turn, None);
        assert_eq!(delayed.expires_at_turn, None);
        assert_eq!(delayed.ability_source, None);
        assert_eq!(delayed.ability_source_stable_id, None);
        assert_eq!(delayed.ability_source_name, None);
        assert!(delayed.ability_source_snapshot.is_none());
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

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
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
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
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
        assert_eq!(game.effect_store.delayed_triggers.len(), 2);
        assert_eq!(
            game.effect_store.delayed_triggers[0].target_objects,
            vec![watched_a]
        );
        assert_eq!(
            game.effect_store.delayed_triggers[1].target_objects,
            vec![watched_b]
        );
    }
}
