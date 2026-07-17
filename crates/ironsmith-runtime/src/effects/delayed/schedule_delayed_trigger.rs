//! Schedule delayed trigger effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::{resolve_player_filter, resolve_source_object_id};
use crate::effects::{EffectExecutionCategory, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::resolution::ResolutionProgram;
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
    pub effects: ResolutionProgram,
    pub one_shot: bool,
    pub start_next_turn: bool,
    pub duration: ironsmith_core::DelayedTriggerDuration,
    pub until_end_of_turn: bool,
    pub until_end_of_combat: bool,
    pub watch_ability_source: bool,
    pub watch_all_object_targets: bool,
    pub either_of_watched_objects: bool,
    pub target_objects: Vec<crate::ids::ObjectId>,
    pub target_tag: Option<TagKey>,
    pub target_filter: Option<ObjectFilter>,
    pub controller: PlayerFilter,
}

impl ScheduleDelayedTriggerEffect {
    pub fn new(
        trigger: Trigger,
        effects: impl Into<ResolutionProgram>,
        one_shot: bool,
        target_objects: Vec<crate::ids::ObjectId>,
        controller: PlayerFilter,
    ) -> Self {
        Self {
            trigger,
            effects: effects.into(),
            one_shot,
            start_next_turn: false,
            duration: ironsmith_core::DelayedTriggerDuration::Forever,
            until_end_of_turn: false,
            until_end_of_combat: false,
            watch_ability_source: false,
            watch_all_object_targets: false,
            either_of_watched_objects: false,
            target_objects,
            target_tag: None,
            target_filter: None,
            controller,
        }
    }

    pub fn from_tag(
        trigger: Trigger,
        effects: impl Into<ResolutionProgram>,
        one_shot: bool,
        target_tag: impl Into<TagKey>,
        controller: PlayerFilter,
    ) -> Self {
        Self {
            trigger,
            effects: effects.into(),
            one_shot,
            start_next_turn: false,
            duration: ironsmith_core::DelayedTriggerDuration::Forever,
            until_end_of_turn: false,
            until_end_of_combat: false,
            watch_ability_source: false,
            watch_all_object_targets: false,
            either_of_watched_objects: false,
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
        self.duration = ironsmith_core::DelayedTriggerDuration::EndOfTurn;
        self.until_end_of_turn = true;
        self.until_end_of_combat = false;
        self
    }

    pub fn until_end_of_combat(mut self) -> Self {
        self.duration = ironsmith_core::DelayedTriggerDuration::EndOfCombat;
        self.until_end_of_combat = true;
        self.until_end_of_turn = false;
        self
    }

    pub fn until_controller_next_turn(mut self) -> Self {
        self.duration = ironsmith_core::DelayedTriggerDuration::UntilControllerNextTurn;
        self.until_end_of_turn = false;
        self.until_end_of_combat = false;
        self
    }

    pub fn with_either_of_watched_objects_surface(mut self) -> Self {
        self.either_of_watched_objects = true;
        self
    }

    pub fn watch_ability_source(mut self) -> Self {
        self.watch_ability_source = true;
        self
    }

    pub fn watch_all_object_targets(mut self) -> Self {
        self.watch_all_object_targets = true;
        self
    }
}

impl EffectExecutor for ScheduleDelayedTriggerEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in self.effects.flattened_default_effects() {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;
        // A resolving ability may already have moved its source to another
        // zone before registering this delayed trigger. Follow the source's
        // stable identity so the delayed ability and any `this card` effects
        // refer to the current object rather than the stale pre-zone-change
        // ObjectId.
        let ability_source = resolve_source_object_id(game, ctx).unwrap_or(ctx.source);
        let mut tagged_objects = ctx.tagged_objects.clone();
        if !ctx.targets_are_cost_choices {
            for (idx, target) in ctx.targets.iter().enumerate() {
                let crate::effects::ResolvedTarget::Object(object_id) = target else {
                    continue;
                };
                let Some(object) = game.object(*object_id) else {
                    continue;
                };
                let tag = TagKey::from(format!("targeted_{idx}"));
                if !tagged_objects.contains_key(&tag) {
                    tagged_objects.insert(
                        tag,
                        vec![
                            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                                object, game,
                            ),
                        ],
                    );
                }
            }
        }

        if let Some(tag) = &self.target_tag {
            let Some(tagged) = tagged_objects.get(tag) else {
                return Ok(EffectOutcome::count(0));
            };
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.tagged_objects = tagged_objects.clone();
            let mut matched = 0i32;
            for snapshot in tagged {
                if let Some(filter) = &self.target_filter
                    && !filter.matches_snapshot(snapshot, &filter_ctx, game)
                {
                    continue;
                }
                let mut delayed_tagged_objects = tagged_objects.clone();
                delayed_tagged_objects.insert(tag.clone(), vec![snapshot.clone()]);
                let delayed = DelayedTriggerTemplate::new(
                    self.trigger.clone(),
                    self.effects.clone(),
                    self.one_shot,
                    controller_id,
                )
                .with_ability_source(Some(ability_source))
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
                .with_expires_before_controller_turn_after(
                    (self.duration
                        == ironsmith_core::DelayedTriggerDuration::UntilControllerNextTurn)
                        .then_some(game.turn.turn_number),
                )
                .with_expires_at_end_of_combat(self.until_end_of_combat)
                .with_tagged_objects(delayed_tagged_objects);
                queue_delayed_from_template(
                    game,
                    DelayedWatcherIdentity::combined(if self.watch_ability_source {
                        vec![ability_source]
                    } else {
                        vec![snapshot.object_id]
                    }),
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
        .with_ability_source(Some(ability_source))
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
        .with_expires_before_controller_turn_after(
            (self.duration == ironsmith_core::DelayedTriggerDuration::UntilControllerNextTurn)
                .then_some(game.turn.turn_number),
        )
        .with_expires_at_end_of_combat(self.until_end_of_combat)
        .with_tagged_objects(tagged_objects);
        let mut watched_targets = if self.watch_all_object_targets {
            ctx.targets
                .iter()
                .filter_map(|target| match target {
                    crate::effects::ResolvedTarget::Object(object_id) => Some(*object_id),
                    _ => None,
                })
                .collect::<Vec<_>>()
        } else {
            self.target_objects.clone()
        };
        if self.watch_all_object_targets
            && let Some(filter) = &self.target_filter
        {
            let filter_ctx = ctx.filter_context(game);
            watched_targets.retain(|object_id| {
                game.object(*object_id)
                    .is_some_and(|object| filter.matches(object, &filter_ctx, game))
            });
        }
        watched_targets.sort_unstable();
        watched_targets.dedup();

        queue_delayed_from_template(
            game,
            if self.watch_all_object_targets {
                DelayedWatcherIdentity::per_object(watched_targets)
            } else {
                DelayedWatcherIdentity::combined(if self.watch_ability_source {
                    vec![ability_source]
                } else {
                    watched_targets
                })
            },
            delayed,
        );

        Ok(EffectOutcome::resolved())
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::DelayedTriggerRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;
    use crate::game_loop::{put_triggers_on_stack, resolve_stack_entry};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ChooseSpec;
    use crate::triggers::TriggerQueue;
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
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        let delayed = &game.effect_store.delayed_triggers[0];
        let tagged = delayed
            .tagged_objects
            .get("triggering")
            .expect("captured triggering tag");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].object_id, snapshot.object_id);
    }

    #[test]
    fn test_schedule_delayed_trigger_starting_next_turn_waits_for_next_draw_step() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_draw_step(PlayerFilter::You),
            vec![Effect::draw(1)],
            true,
            Vec::new(),
            PlayerFilter::You,
        )
        .starting_next_turn();

        effect
            .execute(&mut game, &mut ctx)
            .expect("schedule should resolve");
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        let same_turn_draw = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfDrawStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            crate::triggers::check_delayed_triggers(&mut game, &same_turn_draw).is_empty(),
            "draw-step trigger should not fire during the turn it was created"
        );

        game.turn.turn_number += 2;
        let next_draw = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfDrawStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_delayed_triggers(&mut game, &next_draw);
        assert_eq!(
            triggers.len(),
            1,
            "draw-step trigger should fire on the next turn's draw step"
        );
    }

    #[test]
    fn scheduled_land_play_trigger_fires_for_the_matching_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let land = CardBuilder::new(CardId::from_raw(992), "Delayed Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ScheduleDelayedTriggerEffect::new(
            Trigger::player_plays_land(PlayerFilter::You, ObjectFilter::land()),
            vec![Effect::draw(1)],
            true,
            Vec::new(),
            PlayerFilter::You,
        )
        .until_end_of_turn();
        effect
            .execute(&mut game, &mut ctx)
            .expect("schedule should resolve");

        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LandPlayedEvent::new(land_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(
            crate::triggers::check_delayed_triggers(&mut game, &event).len(),
            1,
            "scheduled land-play trigger should fire for its controller"
        );
    }

    #[test]
    fn repeating_tagged_trigger_watches_only_the_captured_set_until_controllers_next_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;

        let card = CardBuilder::new(CardId::from_raw(996), "Duration Watcher")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let chosen_a = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let chosen_b = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let decoy = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(chosen_a),
            crate::effects::ResolvedTarget::Object(chosen_b),
        ]);

        let effect = ScheduleDelayedTriggerEffect::new(
            Trigger::deals_combat_damage(ObjectFilter::source()),
            vec![Effect::draw(1)],
            false,
            Vec::new(),
            PlayerFilter::You,
        )
        .with_target_filter(ObjectFilter::creature())
        .watch_all_object_targets()
        .until_controller_next_turn();
        effect
            .execute(&mut game, &mut ctx)
            .expect("duration-scoped trigger should schedule");
        assert_eq!(
            game.effect_store.delayed_triggers.len(),
            2,
            "one repeating registration should be captured per watched object"
        );

        let combat_damage = |source| {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::DamageEvent::with_cause(
                    source,
                    crate::events::DamageTarget::Player(bob),
                    1,
                    true,
                    crate::events::cause::EventCause::combat_damage(source),
                ),
                crate::provenance::ProvNodeId::default(),
            )
        };

        assert!(
            crate::triggers::check_delayed_triggers(&mut game, &combat_damage(decoy)).is_empty(),
            "an object outside the captured set must not fire the trigger"
        );
        assert_eq!(
            crate::triggers::check_delayed_triggers(&mut game, &combat_damage(chosen_a)).len(),
            1
        );
        assert_eq!(
            crate::triggers::check_delayed_triggers(&mut game, &combat_damage(chosen_a)).len(),
            1,
            "the registration must repeat"
        );

        game.turn.turn_number += 1;
        game.turn.active_player = bob;
        assert_eq!(
            crate::triggers::check_delayed_triggers(&mut game, &combat_damage(chosen_b)).len(),
            1,
            "the registration remains active through intervening turns"
        );

        game.turn.turn_number += 1;
        game.turn.active_player = alice;
        assert!(
            crate::triggers::check_delayed_triggers(&mut game, &combat_damage(chosen_a)).is_empty(),
            "the registration expires before events on its controller's next turn"
        );
        assert!(game.effect_store.delayed_triggers.is_empty());
    }

    #[test]
    fn tagged_leaves_trigger_tracks_chosen_object_and_current_exiled_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(993), "Returning Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let source_battlefield_id =
            game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let source_snapshot = ObjectSnapshot::from_object(
            game.object(source_battlefield_id)
                .expect("source should be on the battlefield"),
            &game,
        );
        let source_stable_id = source_snapshot.stable_id;
        let source_graveyard_id = game
            .move_object_by_effect(source_battlefield_id, Zone::Graveyard)
            .expect("source should move to the graveyard");
        let source_exile_id = game
            .move_object_by_effect(source_graveyard_id, Zone::Exile)
            .expect("source should move from the graveyard to exile");
        game.take_pending_trigger_events();

        let chosen_card = CardBuilder::new(CardId::from_raw(994), "Chosen Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let chosen_id = game.create_object_from_card(&chosen_card, bob, Zone::Battlefield);
        let chosen_snapshot = ObjectSnapshot::from_object(
            game.object(chosen_id)
                .expect("chosen creature should be on the battlefield"),
            &game,
        );

        let decoy_card = CardBuilder::new(CardId::from_raw(995), "Decoy Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let decoy_id = game.create_object_from_card(&decoy_card, bob, Zone::Battlefield);

        let return_source = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Object(ObjectFilter::source().in_zone(Zone::Exile)),
            Zone::Battlefield,
            false,
        )
        .under_owner_control();
        let effect = ScheduleDelayedTriggerEffect::from_tag(
            Trigger::this_leaves_battlefield(),
            vec![Effect::new(return_source)],
            true,
            "chosen",
            PlayerFilter::You,
        )
        .with_target_filter(ObjectFilter::creature());

        let mut ctx = ExecutionContext::new_default(source_battlefield_id, alice)
            .with_source_snapshot(source_snapshot);
        ctx.tag_object("chosen", chosen_snapshot);
        effect
            .execute(&mut game, &mut ctx)
            .expect("delayed trigger should be scheduled");

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert_eq!(
            delayed.ability_source,
            Some(source_exile_id),
            "the stale pre-zone-change source id should resolve to the current exile object"
        );
        assert_eq!(delayed.ability_source_stable_id, Some(source_stable_id));
        assert_eq!(delayed.target_objects, vec![chosen_id]);
        assert!(!delayed.target_objects.contains(&decoy_id));

        game.move_object_by_effect(decoy_id, Zone::Graveyard)
            .expect("decoy should leave the battlefield");
        let decoy_entries = game
            .take_pending_trigger_events()
            .into_iter()
            .flat_map(|event| crate::triggers::check_delayed_triggers(&mut game, &event))
            .collect::<Vec<_>>();
        assert!(
            decoy_entries.is_empty(),
            "an unrelated creature leaving must not fire the delayed trigger"
        );
        assert_eq!(
            game.effect_store.delayed_triggers.len(),
            1,
            "the one-shot trigger should remain armed after the decoy leaves"
        );

        game.move_object_by_effect(chosen_id, Zone::Graveyard)
            .expect("chosen creature should leave the battlefield");
        let chosen_entries = game
            .take_pending_trigger_events()
            .into_iter()
            .flat_map(|event| crate::triggers::check_delayed_triggers(&mut game, &event))
            .collect::<Vec<_>>();
        assert_eq!(chosen_entries.len(), 1);
        assert_eq!(chosen_entries[0].source, source_exile_id);
        assert_eq!(chosen_entries[0].source_stable_id, source_stable_id);
        assert!(
            game.effect_store.delayed_triggers.is_empty(),
            "the one-shot delayed trigger should be consumed by the chosen creature"
        );

        let mut trigger_queue = TriggerQueue::new();
        for entry in chosen_entries {
            trigger_queue.add(entry);
        }
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("delayed trigger should go on the stack");
        resolve_stack_entry(&mut game).expect("delayed trigger should resolve");

        let returned_id = game
            .find_object_by_stable_id(source_stable_id)
            .expect("source should still exist after returning from exile");
        let returned = game
            .object(returned_id)
            .expect("returned source object should exist");
        assert_eq!(returned.zone, Zone::Battlefield);
        assert_eq!(returned.owner, alice);
        assert_eq!(game.controller_of(returned), alice);
    }
}
