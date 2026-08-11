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
    tagged_collection_has_object_in_zone,
};

/// A payment window attached to a delayed-trigger registration.
#[derive(Debug, Clone, PartialEq)]
pub struct DelayedTriggerPrepayment {
    pub player: PlayerFilter,
    pub cost: crate::cost::TotalCost,
}

impl DelayedTriggerPrepayment {
    pub fn new(player: PlayerFilter, cost: crate::cost::TotalCost) -> Self {
        Self { player, cost }
    }
}

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
    pub leading_duration_surface: bool,
    pub watch_ability_source: bool,
    pub watch_all_object_targets: bool,
    pub either_of_watched_objects: bool,
    pub while_any_tagged_object_in_zone: Option<(TagKey, crate::zone::Zone)>,
    pub target_objects: Vec<crate::ids::ObjectId>,
    pub target_tag: Option<TagKey>,
    pub target_filter: Option<ObjectFilter>,
    pub controller: PlayerFilter,
    pub prepayment: Option<DelayedTriggerPrepayment>,
    /// Capture the prevention shield registered immediately before this
    /// delayed trigger and expose its accumulated prevented damage as the
    /// delayed ability's numeric event value.
    pub event_value_from_prior_prevention: bool,
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
            leading_duration_surface: false,
            watch_ability_source: false,
            watch_all_object_targets: false,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
            target_objects,
            target_tag: None,
            target_filter: None,
            controller,
            prepayment: None,
            event_value_from_prior_prevention: false,
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
            leading_duration_surface: false,
            watch_ability_source: false,
            watch_all_object_targets: false,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
            target_objects: Vec::new(),
            target_tag: Some(target_tag.into()),
            target_filter: None,
            controller,
            prepayment: None,
            event_value_from_prior_prevention: false,
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

    pub fn unless_paid_before_trigger(
        mut self,
        player: PlayerFilter,
        cost: crate::cost::TotalCost,
    ) -> Self {
        self.prepayment = Some(DelayedTriggerPrepayment::new(player, cost));
        self
    }

    pub fn with_prior_prevention_event_value(mut self) -> Self {
        self.event_value_from_prior_prevention = true;
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

    pub fn with_leading_duration_surface(mut self) -> Self {
        self.leading_duration_surface = true;
        self
    }

    pub fn with_either_of_watched_objects_surface(mut self) -> Self {
        self.either_of_watched_objects = true;
        self
    }

    pub fn while_any_tagged_object_in_zone(
        mut self,
        tag: impl Into<TagKey>,
        zone: crate::zone::Zone,
    ) -> Self {
        self.while_any_tagged_object_in_zone = Some((tag.into(), zone));
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
        let filter_ctx = ctx.filter_context(game);
        let mut tagged_players = filter_ctx.tagged_players.clone();
        if !ctx.targets_are_cost_choices {
            let mut target_players = ctx
                .targets
                .iter()
                .filter_map(|target| match target {
                    crate::effects::ResolvedTarget::Player(player) => Some(*player),
                    crate::effects::ResolvedTarget::Object(_) => None,
                })
                .collect::<Vec<_>>();
            target_players.sort_unstable();
            target_players.dedup();
            if !target_players.is_empty() {
                tagged_players
                    .entry(crate::tag::DELAYED_TARGET_PLAYERS_TAG.into())
                    .or_insert(target_players);
            }
        }
        let prepayment = self
            .prepayment
            .as_ref()
            .map(|payment| {
                resolve_player_filter(game, &payment.player, ctx).map(|player| {
                    crate::triggers::PendingDelayedTriggerPayment {
                        player,
                        cost: payment.cost.clone(),
                        source: ability_source,
                    }
                })
            })
            .transpose()?;
        let prevention_shield = if self.event_value_from_prior_prevention {
            Some(ctx.last_prevention_shield.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "delayed prevention metric requires a prior prevention shield".to_string(),
                )
            })?)
        } else {
            None
        };
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

        if let Some((tag, zone)) = &self.while_any_tagged_object_in_zone
            && !tagged_collection_has_object_in_zone(game, &tagged_objects, tag, *zone)
        {
            return Ok(EffectOutcome::count(0));
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
                .while_any_tagged_object_in_zone_opt(self.while_any_tagged_object_in_zone.clone())
                .with_tagged_objects(delayed_tagged_objects)
                .with_tagged_players(tagged_players.clone())
                .with_prepayment(prepayment.clone())
                .with_prevention_shield(prevention_shield);
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
        .while_any_tagged_object_in_zone_opt(self.while_any_tagged_object_in_zone.clone())
        .with_tagged_objects(tagged_objects)
        .with_tagged_players(tagged_players)
        .with_prepayment(prepayment)
        .with_prevention_shield(prevention_shield);
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
    fn returned_object_enter_watcher_defers_payload_and_ignores_other_entries() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(980), "Linked Return Spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
        let permanent_card = CardBuilder::new(CardId::from_raw(981), "Returned Permanent")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let watched = game.create_object_from_card(&permanent_card, bob, Zone::Graveyard);
        let decoy_card = CardBuilder::new(CardId::from_raw(982), "Unrelated Permanent")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let decoy = game.create_object_from_card(&decoy_card, bob, Zone::Graveyard);

        game.move_object_by_effect(decoy, Zone::Battlefield)
            .expect("decoy should enter first");
        let mut ctx = ExecutionContext::new_default(source, alice);
        crate::effects::TaggedEffect::new(
            "returned",
            Effect::new(crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
                ChooseSpec::SpecificObject(watched),
                false,
            )),
        )
        .execute(&mut game, &mut ctx)
        .expect("watched permanent should return");

        ScheduleDelayedTriggerEffect::from_tag(
            Trigger::this_enters_battlefield(),
            vec![Effect::gain_life(3)],
            true,
            "returned",
            PlayerFilter::You,
        )
        .execute(&mut game, &mut ctx)
        .expect("linked enter watcher should register");

        let life_before = game.player(alice).expect("Alice should exist").life;
        let mut trigger_queue = TriggerQueue::new();
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "only the linked returned permanent's entry should fire"
        );
        assert_eq!(
            game.player(alice).expect("Alice should exist").life,
            life_before
        );

        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("linked delayed trigger should go on the stack");
        resolve_stack_entry(&mut game).expect("linked delayed payload should resolve");
        assert_eq!(
            game.player(alice).expect("Alice should exist").life,
            life_before + 3
        );
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
    fn scheduled_cleanup_trigger_waits_for_cleanup_step() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_cleanup_step(PlayerFilter::Any),
            vec![Effect::draw(1)],
            true,
            Vec::new(),
            PlayerFilter::You,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("cleanup schedule should resolve");

        let end_step = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            crate::triggers::check_delayed_triggers(&mut game, &end_step).is_empty(),
            "cleanup action must not fire during the end step"
        );

        let cleanup = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfCleanupStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(
            crate::triggers::check_delayed_triggers(&mut game, &cleanup).len(),
            1,
            "cleanup action should fire when the cleanup step begins"
        );
    }

    #[test]
    fn next_cleanup_trigger_exiles_every_captured_tagged_object_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(997), "Delayed Cleanup Source")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let token_card = CardBuilder::new(CardId::from_raw(998), "Knight Token")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let token_ids = (0..3)
            .map(|_| game.create_object_from_card(&token_card, alice, Zone::Battlefield))
            .collect::<Vec<_>>();
        let token_snapshots = token_ids
            .iter()
            .map(|object_id| {
                ObjectSnapshot::from_object(
                    game.object(*object_id)
                        .expect("created token should be on the battlefield"),
                    &game,
                )
            })
            .collect::<Vec<_>>();
        let token_stable_ids = token_snapshots
            .iter()
            .map(|snapshot| snapshot.stable_id)
            .collect::<Vec<_>>();

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_objects("created_0", token_snapshots);
        let exile_created = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged("created_0".into()),
            Zone::Exile,
            true,
        )
        .with_target_plural_surface();
        let effect = ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_next_cleanup_step(PlayerFilter::Any),
            vec![Effect::new(exile_created)],
            true,
            Vec::new(),
            PlayerFilter::You,
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("next-cleanup schedule should resolve");
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        let end_step = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            crate::triggers::check_delayed_triggers(&mut game, &end_step).is_empty(),
            "next-cleanup action must not fire during the end step"
        );
        assert_eq!(
            game.effect_store.delayed_triggers.len(),
            1,
            "an unrelated event must leave the delayed action armed"
        );

        let cleanup = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfCleanupStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let entries = crate::triggers::check_delayed_triggers(&mut game, &cleanup);
        assert_eq!(entries.len(), 1);
        assert!(
            game.effect_store.delayed_triggers.is_empty(),
            "the next-cleanup registration must be consumed by its first match"
        );

        let mut trigger_queue = TriggerQueue::new();
        for entry in entries {
            trigger_queue.add(entry);
        }
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("next-cleanup action should go on the stack");
        resolve_stack_entry(&mut game).expect("next-cleanup action should resolve");

        for stable_id in token_stable_ids {
            let current_id = game
                .find_object_by_stable_id(stable_id)
                .expect("each created object should still exist in exile");
            assert_eq!(
                game.object(current_id)
                    .expect("exiled object should exist")
                    .zone,
                Zone::Exile
            );
        }

        let second_cleanup = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfCleanupStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            crate::triggers::check_delayed_triggers(&mut game, &second_cleanup).is_empty(),
            "a one-shot next-cleanup action must not fire again"
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

    #[test]
    fn collection_scoped_upkeep_trigger_partitions_choices_by_active_owner_and_expires() {
        fn resolve_upkeep(game: &mut GameState, player: PlayerId) {
            game.turn.active_player = player;
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::phase::BeginningOfUpkeepEvent::new(player),
                crate::provenance::ProvNodeId::default(),
            );
            let entries = crate::triggers::check_delayed_triggers(game, &event);
            assert_eq!(entries.len(), 1, "one collection trigger should fire");
            let mut queue = TriggerQueue::new();
            for entry in entries {
                queue.add(entry);
            }
            put_triggers_on_stack(game, &mut queue).expect("upkeep trigger should reach the stack");
            resolve_stack_entry(game).expect("upkeep return should resolve");
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = |raw_id, name| {
            CardBuilder::new(CardId::from_raw(raw_id), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build()
        };
        let alice_exiled =
            game.create_object_from_card(&card(1101, "Alice Exiled"), alice, Zone::Exile);
        let bob_exiled = game.create_object_from_card(&card(1102, "Bob Exiled"), bob, Zone::Exile);
        let alice_stable = game.object(alice_exiled).expect("Alice's card").stable_id;
        let bob_stable = game.object(bob_exiled).expect("Bob's card").stable_id;
        let source =
            game.create_object_from_card(&card(1103, "Schedule Source"), alice, Zone::Graveyard);

        let collection_tag = TagKey::from(crate::tag::SOURCE_EXILED_TAG);
        let choice_tag = TagKey::from("__collection_upkeep_choice");
        let choice_filter = ObjectFilter::tagged(collection_tag.clone())
            .in_zone(Zone::Exile)
            .owned_by(PlayerFilter::Active);
        let choose = crate::effects::ChooseObjectsEffect::new(
            choice_filter,
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::Active,
            choice_tag.clone(),
        )
        .in_zone(Zone::Exile);
        let return_owned = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(choice_tag),
            Zone::Battlefield,
            false,
        )
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
        .under_owner_control();
        let schedule = ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_upkeep(PlayerFilter::Any),
            vec![Effect::new(choose), Effect::new(return_owned)],
            false,
            Vec::new(),
            PlayerFilter::You,
        )
        .while_any_tagged_object_in_zone(collection_tag.clone(), Zone::Exile);

        let mut ctx = ExecutionContext::new_default(source, alice);
        for object_id in [alice_exiled, bob_exiled] {
            ctx.tag_object(
                collection_tag.clone(),
                ObjectSnapshot::from_object(game.object(object_id).expect("exiled card"), &game),
            );
        }
        schedule
            .execute(&mut game, &mut ctx)
            .expect("collection trigger should register");

        resolve_upkeep(&mut game, alice);
        assert_eq!(
            game.find_object_by_stable_id(alice_stable)
                .and_then(|id| game.object(id))
                .map(|object| object.zone),
            Some(Zone::Battlefield)
        );
        assert_eq!(
            game.object(bob_exiled).map(|object| object.zone),
            Some(Zone::Exile),
            "Alice's upkeep must not return Bob's card"
        );

        resolve_upkeep(&mut game, bob);
        assert_eq!(
            game.find_object_by_stable_id(bob_stable)
                .and_then(|id| game.object(id))
                .map(|object| object.zone),
            Some(Zone::Battlefield)
        );

        game.turn.active_player = alice;
        let empty_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(
            crate::triggers::check_delayed_triggers(&mut game, &empty_event).is_empty(),
            "the registration must not fire after the captured exile collection empties"
        );
        assert!(game.effect_store.delayed_triggers.is_empty());
    }

    #[test]
    fn delayed_prepayment_is_payable_after_the_source_leaves() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_card = CardBuilder::new(CardId::new(), "Asp Payment Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(bob),
                1,
                false,
                crate::events::EventCause::from_effect(source, alice),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_triggering_event(damage_event);
        ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_draw_step(PlayerFilter::DamagedPlayer),
            vec![Effect::lose_life_player(1, PlayerFilter::DamagedPlayer)],
            true,
            Vec::new(),
            PlayerFilter::You,
        )
        .starting_next_turn()
        .unless_paid_before_trigger(
            PlayerFilter::DamagedPlayer,
            crate::cost::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
        )
        .execute(&mut game, &mut ctx)
        .expect("Nafs-like delayed obligation should register");

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert_eq!(
            delayed.tagged_players.get(&TagKey::from("damaged_player")),
            Some(&vec![bob])
        );
        assert_eq!(
            delayed.prepayment.as_ref().map(|payment| payment.player),
            Some(bob)
        );

        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should leave the battlefield");
        game.turn.priority_player = Some(bob);
        game.player_mut(bob)
            .expect("Bob should exist")
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        let action = crate::special_actions::SpecialAction::PayDelayedTrigger {
            delayed_trigger_index: 0,
        };
        assert!(crate::special_actions::can_perform_check(&action, &game, bob).is_ok());
        assert!(
            crate::decision::compute_legal_actions(&game, bob)
                .contains(&crate::decision::LegalAction::SpecialAction(action.clone()))
        );
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        crate::special_actions::perform(action, &mut game, bob, &mut decision_maker)
            .expect("Bob should be able to prepay the delayed obligation");
        assert!(game.effect_store.delayed_triggers.is_empty());
    }

    #[test]
    fn unpaid_delayed_penalty_keeps_the_captured_damaged_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_card = CardBuilder::new(CardId::new(), "Asp Binding Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Player(bob),
                1,
                false,
                crate::events::EventCause::from_effect(source, alice),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_triggering_event(damage_event);
        ScheduleDelayedTriggerEffect::new(
            Trigger::beginning_of_draw_step(PlayerFilter::DamagedPlayer),
            vec![Effect::lose_life_player(1, PlayerFilter::DamagedPlayer)],
            true,
            Vec::new(),
            PlayerFilter::You,
        )
        .starting_next_turn()
        .unless_paid_before_trigger(
            PlayerFilter::DamagedPlayer,
            crate::cost::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
        )
        .execute(&mut game, &mut ctx)
        .expect("Nafs-like delayed obligation should register");
        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should leave the battlefield");

        game.turn.turn_number = game.turn.turn_number.saturating_add(1);
        game.turn.active_player = bob;
        let draw_step = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfDrawStepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        let triggered = crate::triggers::check_delayed_triggers(&mut game, &draw_step);
        assert_eq!(
            triggered.len(),
            1,
            "captured player's draw step should fire"
        );
        assert!(game.effect_store.delayed_triggers.is_empty());
        assert_eq!(
            triggered[0]
                .triggering_event
                .player_tags()
                .get(&TagKey::from("damaged_player")),
            Some(&vec![bob])
        );

        let effect = triggered[0].ability.effects.flattened_default_effects()[0].clone();
        let mut resolution_ctx = ExecutionContext::new_default(source, alice)
            .with_triggering_event(triggered[0].triggering_event.clone());
        assert_eq!(
            resolution_ctx.get_tagged_players("damaged_player"),
            Some(&vec![bob])
        );
        let life_before = game.player(bob).expect("Bob should exist").life;
        crate::effects::execute_effect(&mut game, &effect, &mut resolution_ctx)
            .expect("unpaid penalty should resolve");
        assert_eq!(
            game.player(bob).expect("Bob should exist").life,
            life_before - 1
        );
    }
}
