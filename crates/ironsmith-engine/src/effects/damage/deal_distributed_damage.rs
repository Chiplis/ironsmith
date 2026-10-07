//! Deal distributed damage among multiple targets.

use super::deal_damage::{
    CapturedDamageInstructionPlan, DamageInstructionInputProvider, DamageInstructionPlan,
};
use crate::decision::FallbackStrategy;
use crate::decisions::{DistributeSpec, make_decision_with_fallback};
use crate::effect::{ChoiceCount, EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_effect_source_with_lki, resolve_objects_from_spec, resolve_players_from_spec,
    resolve_value,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::damage::{checked_damage_amount, checked_damage_count};
use crate::events::processing::SimultaneousDamageEvent;
use crate::game_state::{GameState, Target};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::target::PlayerFilter;

pub use ironsmith_core::DamageDistributionMode;

/// Effect that deals a total amount of damage divided among chosen targets.
#[derive(Debug, Clone, PartialEq)]
pub struct DealDistributedDamageEffect {
    /// The total amount of damage to distribute.
    pub amount: Value,
    /// The target specification for the distributed damage choices.
    pub target: ChooseSpec,
    /// The object that is the source of the damage.
    pub source: ChooseSpec,
    /// The player who chooses the distribution.
    pub chooser: PlayerFilter,
    /// How the announced targets divide the total.
    pub distribution: DamageDistributionMode,
}

impl DealDistributedDamageEffect {
    /// Create a new distributed-damage effect.
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            target,
            source: ChooseSpec::Source,
            chooser: PlayerFilter::You,
            distribution: DamageDistributionMode::Chosen,
        }
    }

    /// Use a resolved object other than the enclosing effect's source as the damage source.
    pub fn with_source(mut self, source: ChooseSpec) -> Self {
        self.source = source;
        self
    }

    /// Let the indicated player choose how the damage is divided.
    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    /// Use the authored distribution rule instead of player-assigned shares.
    pub fn with_distribution(mut self, distribution: DamageDistributionMode) -> Self {
        self.distribution = distribution;
        self
    }

    fn with_resolved_source<T>(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<Option<T>, ExecutionError> {
        game.establish_control_transition_boundary()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let Some((damage_source, tagged_snapshot)) =
            resolve_effect_source_with_lki(game, ctx, &self.source)
        else {
            return Ok(None);
        };
        let source_snapshot = tagged_snapshot.or_else(|| {
            game.object(damage_source).map(|object| {
                ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
            })
        });

        let original_source = ctx.source;
        let original_source_snapshot = ctx.source_snapshot.clone();
        ctx.source = damage_source;
        ctx.source_snapshot = source_snapshot;
        let outcome = body(game, ctx);
        ctx.source = original_source;
        ctx.source_snapshot = original_source_snapshot;
        outcome.map(Some)
    }

    fn resolve_distribution_inputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<DamageInstructionPlan, ExecutionError> {
        let announced_distribution = if self.distribution == DamageDistributionMode::Chosen {
            ctx.take_target_distribution(&self.target)
        } else {
            None
        };
        let uses_announced_distribution = announced_distribution.is_some();
        // Chosen damage division is an announcement, including its complete
        // budget. Do not re-read a dynamic X/history/source value at resolution.
        // Keep the shares of illegal targets in that original budget too.
        let total = if let Some(announced) = &announced_distribution {
            checked_damage_amount(
                announced
                    .allocations
                    .iter()
                    .map(|(_, amount)| u128::from(*amount))
                    .sum(),
                "announced damage distribution",
            )?
        } else {
            resolve_value(game, &self.amount, ctx)?.max(0) as u32
        };
        checked_damage_count(u128::from(total), "distributed damage outcome")?;
        if total == 0 {
            return Ok(DamageInstructionPlan::Finished(EffectOutcome::count(0)));
        }

        let mut available_targets = Vec::new();

        let players = match resolve_players_from_spec(game, &self.target, ctx) {
            Ok(players) => players,
            Err(error) if error.is_incomplete_execution() => return Err(error),
            Err(_) => Vec::new(),
        };
        for player_id in players {
            available_targets.push(Target::Player(player_id));
        }

        let objects = match resolve_objects_from_spec(game, &self.target, ctx) {
            Ok(objects) => objects,
            Err(error) if error.is_incomplete_execution() => return Err(error),
            Err(_) => Vec::new(),
        };
        for object_id in objects {
            // "Any target" includes battles (CR 115.4); current (layered)
            // card types decide whether the object can be dealt damage.
            if super::deal_damage::object_can_be_dealt_damage(game, object_id) {
                available_targets.push(Target::Object(object_id));
            }
        }

        if available_targets.is_empty() {
            return if self.target.count().min == 0 {
                Ok(DamageInstructionPlan::Finished(EffectOutcome::count(0)))
            } else {
                Ok(DamageInstructionPlan::Finished(
                    EffectOutcome::target_invalid(),
                ))
            };
        }

        let distribution = if self.distribution == DamageDistributionMode::EvenRoundedDown {
            available_targets
                .iter()
                .copied()
                .map(|target| (target, 1))
                .collect()
        } else if let Some(distribution) = announced_distribution {
            distribution.allocations
        } else {
            let chooser = crate::effects::helpers::resolve_player_filter_as_chooser(
                game,
                &self.chooser,
                ctx,
            )?;
            let distribution = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                chooser,
                Some(ctx.source),
                DistributeSpec::damage(ctx.source, total, available_targets.clone()),
                FallbackStrategy::Maximum,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(DamageInstructionPlan::Finished(EffectOutcome::count(0)));
            }
            distribution
        };
        if distribution.is_empty() && self.target.count().min == 0 {
            return Ok(DamageInstructionPlan::Finished(EffectOutcome::count(0)));
        }

        // Keep allocations in the order targets were chosen (first mention wins)
        // so damage events apply in the same order on every peer.
        fn allocation_slot(allocations: &mut Vec<(Target, u32)>, target: Target) -> &mut u32 {
            let index = match allocations
                .iter()
                .position(|(existing, _)| *existing == target)
            {
                Some(index) => index,
                None => {
                    allocations.push((target, 0));
                    allocations.len() - 1
                }
            };
            &mut allocations[index].1
        }
        let mut allocations: Vec<(Target, u32)> = Vec::new();
        for (target, amount) in distribution {
            if amount > 0 && available_targets.contains(&target) {
                let slot = allocation_slot(&mut allocations, target);
                *slot = checked_damage_amount(
                    u128::from(*slot) + u128::from(amount),
                    "coalesced damage allocation",
                )?;
            }
        }

        let assigned_total = checked_damage_amount(
            allocations
                .iter()
                .map(|(_, amount)| u128::from(*amount))
                .sum(),
            "assigned damage total",
        )?;
        if self.distribution == DamageDistributionMode::Chosen && assigned_total > total {
            return Ok(DamageInstructionPlan::Finished(EffectOutcome::impossible()));
        }

        if self.distribution == DamageDistributionMode::EvenRoundedDown && !allocations.is_empty() {
            // CR 601.2d / 608.2b: the even division is fixed over the targets
            // announced as the spell was cast. A target that became illegal
            // loses its share; the others don't absorb it.
            let announced = ctx
                .announced_target_count(&self.target)
                .filter(|count| *count > 0)
                .unwrap_or(allocations.len())
                .max(allocations.len());
            let share = u128::from(total) / announced as u128;
            let share = checked_damage_amount(share, "even damage allocation")?;
            for (_, amount) in allocations.iter_mut() {
                *amount = share;
            }
        }

        let distributed_total = checked_damage_amount(
            allocations
                .iter()
                .map(|(_, amount)| u128::from(*amount))
                .sum(),
            "distributed damage total",
        )?;

        // CR 608.2b does not let a resolving spell reassign damage that was
        // announced for a target that has since become illegal.
        if self.distribution == DamageDistributionMode::Chosen
            && !uses_announced_distribution
            && distributed_total < total
        {
            let remaining = total - distributed_total;
            if let Some(first_target) = available_targets.first().copied() {
                let slot = allocation_slot(&mut allocations, first_target);
                *slot = checked_damage_amount(
                    u128::from(*slot) + u128::from(remaining),
                    "remaining damage allocation",
                )?;
            }
        }

        let events = allocations
            .into_iter()
            .filter(|(_, amount)| *amount > 0)
            .map(|(target, amount)| SimultaneousDamageEvent {
                source: ctx.source,
                target: match target {
                    Target::Player(player) => crate::events::DamageTarget::Player(player),
                    Target::Object(object) => crate::events::DamageTarget::Object(object),
                },
                amount,
                is_combat: false,
                unpreventable: false,
                cause: ctx.cause.clone(),
                source_snapshot: ctx.source_snapshot.clone(),
            })
            .collect::<Vec<_>>();
        if events.is_empty() {
            return Ok(DamageInstructionPlan::Finished(
                if self.distribution == DamageDistributionMode::EvenRoundedDown {
                    EffectOutcome::count(0)
                } else {
                    EffectOutcome::target_invalid()
                },
            ));
        }
        Ok(DamageInstructionPlan::from_independent_events(events))
    }
}

impl DamageInstructionInputProvider for DealDistributedDamageEffect {
    fn capture_damage_instruction(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CapturedDamageInstructionPlan, ExecutionError> {
        // Capture inside the source binding so every assignment keeps the
        // source, LKI and value domains used by its authored selector.
        self.with_resolved_source(game, ctx, |game, ctx| {
            self.resolve_distribution_inputs(game, ctx)
                .map(|plan| plan.capture(ctx))
        })
        .map(|plan| {
            plan.unwrap_or_else(|| {
                CapturedDamageInstructionPlan::Finished(EffectOutcome::target_invalid())
            })
        })
    }
}

impl EffectExecutor for DealDistributedDamageEffect {
    fn supports_replacement_draw_continuation(&self) -> bool { true }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        crate::effects::replacement::prepare_native_draw_continuation_with_outputs(self, game, ctx)
    }

    fn supports_damage_action_cohort(&self) -> bool {
        self.distribution == DamageDistributionMode::EvenRoundedDown
    }
    fn shares_iterated_damage_action(&self) -> bool {
        self.supports_damage_action_cohort()
    }
    fn supports_simultaneous_player_action(&self) -> bool {
        self.supports_damage_action_cohort()
    }
    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        if !self.supports_damage_action_cohort() {
            return Err(ExecutionError::InternalError(
                "chosen damage divisions require independent announcement cursors before shared preparation".into(),
            ));
        }
        Ok(super::deal_damage::prepare_damage_instruction(self.clone()))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let result = crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                self.with_resolved_source(game, ctx, |game, ctx| {
                    self.resolve_distribution_inputs(game, ctx)?
                        .execute_with_outputs(game, ctx)
                })
                .map(|outputs| {
                    outputs.unwrap_or_else(|| {
                        crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::target_invalid(),
                        )
                    })
                })
            },
        );
        if ctx.decision_maker.awaiting_choice() && result.is_ok() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        Some(self.target.count())
    }

    fn get_target_distribution_value(&self) -> Option<&Value> {
        (self.distribution == DamageDistributionMode::Chosen).then_some(&self.amount)
    }

    fn target_reuse_policy(&self) -> crate::effects::TargetReusePolicy {
        crate::effects::TargetReusePolicy::AlwaysDeclareNew
    }

    fn target_description(&self) -> &'static str {
        "targets for distributed damage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::effect::ChoiceCount;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::ObjectRef;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        power: i32,
        subtype: Option<Subtype>,
    ) -> ObjectId {
        let id = game.new_object_id();
        let mut builder = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 10));
        if let Some(subtype) = subtype {
            builder = builder.subtypes(vec![subtype]);
        }
        let card = builder.build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    struct SourceControllerDistributes {
        chooser: PlayerId,
        source: ObjectId,
        first: ObjectId,
        second: ObjectId,
    }

    impl DecisionMaker for SourceControllerDistributes {
        fn decide_distribute(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::DistributeContext,
        ) -> Vec<(Target, u32)> {
            assert_eq!(ctx.player, self.chooser);
            assert_eq!(ctx.source, Some(self.source));
            assert_eq!(ctx.total, 3);
            let candidates = ctx
                .targets
                .iter()
                .map(|entry| entry.target)
                .collect::<Vec<_>>();
            assert_eq!(candidates.len(), 2);
            assert!(candidates.contains(&Target::Object(self.first)));
            assert!(candidates.contains(&Target::Object(self.second)));
            vec![
                (Target::Object(self.first), 1),
                (Target::Object(self.second), 2),
            ]
        }
    }

    #[test]
    fn dynamic_source_controller_distributes_power_damage_over_tagged_objects() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let ability_source = creature(&mut game, "Ability Source", alice, 1, None);
        let striking_creature = creature(&mut game, "Striking Creature", bob, 3, None);
        let first_wolf = creature(&mut game, "First Wolf", alice, 2, Some(Subtype::Wolf));
        let second_wolf = creature(&mut game, "Second Wolf", alice, 2, Some(Subtype::Wolf));
        let untagged_wolf = creature(&mut game, "Untagged Wolf", alice, 2, Some(Subtype::Wolf));

        let wolves = TagKey::from("tapped_this_way");
        let mut dm = SourceControllerDistributes {
            chooser: bob,
            source: striking_creature,
            first: first_wolf,
            second: second_wolf,
        };
        let mut ctx = ExecutionContext::new(ability_source, alice, &mut dm).with_targets(vec![
            crate::effects::ResolvedTarget::Object(striking_creature),
        ]);
        for id in [first_wolf, second_wolf] {
            let snapshot = ObjectSnapshot::from_object(game.object(id).unwrap(), &game);
            ctx.tag_object(wolves.clone(), snapshot);
        }

        let effect = DealDistributedDamageEffect::new(
            Value::PowerOf(Box::new(ChooseSpec::Source)),
            ChooseSpec::WithCount(
                Box::new(ChooseSpec::Tagged(wolves)),
                ChoiceCount::any_number(),
            ),
        )
        .with_source(ChooseSpec::SpecificObject(striking_creature))
        .with_chooser(PlayerFilter::ControllerOf(ObjectRef::Target));

        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.damage_on(first_wolf), 1);
        assert_eq!(game.damage_on(second_wolf), 2);
        assert_eq!(game.damage_on(untagged_wolf), 0);
    }

    #[test]
    fn even_rounded_down_uses_selected_targets_not_authored_allocations() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source = creature(&mut game, "Even Damage Source", alice, 1, None);
        let target = ChooseSpec::WithCount(
            Box::new(ChooseSpec::Target(Box::new(ChooseSpec::Player(
                PlayerFilter::Opponent,
            )))),
            ChoiceCount::any_number(),
        );
        let effect = DealDistributedDamageEffect::new(5, target)
            .with_distribution(DamageDistributionMode::EvenRoundedDown);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_targets(vec![
            crate::effects::ResolvedTarget::Player(bob),
            crate::effects::ResolvedTarget::Player(charlie),
        ]);

        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            outcome.count_or_zero(),
            4,
            "one point is lost to rounded-down division"
        );
        assert_eq!(game.player(bob).unwrap().life, 18);
        assert_eq!(game.player(charlie).unwrap().life, 18);
        assert!(effect.get_target_distribution_value().is_none());
    }

    #[test]
    fn announced_budget_does_not_read_a_now_unavailable_value_or_reassign_an_illegal_share() {
        struct NoNewDivision;
        impl DecisionMaker for NoNewDivision {
            fn decide_distribute(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::DistributeContext,
            ) -> Vec<(Target, u32)> {
                panic!("an announced division must not be chosen again");
            }
        }
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let source = creature(&mut game, "Source", a, 1, None);
        let first = creature(&mut game, "Illegal original", b, 1, None);
        let second = creature(&mut game, "Legal original", b, 1, None);
        game.move_object_by_effect(first, Zone::Exile).unwrap();
        let target = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::up_to(2));
        let saved = crate::game_state::TargetDistribution {
            spec: target.clone(),
            range: 0..2,
            allocations: vec![(Target::Object(first), 3), (Target::Object(second), 2)],
        };
        let mut dm = NoNewDivision;
        let mut ctx = ExecutionContext::new(source, a, &mut dm)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(second)])
            .with_target_distributions(vec![saved]);
        assert!(ctx.x_value.is_none());
        let outcome = DealDistributedDamageEffect::new(Value::X, target)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.damage_on(second), 2);
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert!(ctx.target_distributions.is_empty());
    }

    #[test]
    fn pending_damage_replacement_restores_the_consumed_distribution_and_every_allocation() {
        #[derive(Default)]
        struct Pause {
            pending: bool,
        }
        impl DecisionMaker for Pause {
            fn decide_options(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                self.pending = true;
                vec![]
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let source = creature(&mut game, "Source", a, 1, None);
        let first = creature(&mut game, "First", b, 1, None);
        let second = creature(&mut game, "Second", b, 1, None);
        for modification in [
            crate::replacement::EventModification::Multiply(2),
            crate::replacement::EventModification::Add(1),
        ] {
            game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    a,
                    crate::events::damage::matchers::DamageToObjectMatcher::new(
                        crate::target::ObjectFilter::specific(second),
                    ),
                    crate::replacement::ReplacementAction::Modify(modification),
                ),
            );
        }
        let target = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::up_to(2));
        let saved = crate::game_state::TargetDistribution {
            spec: target.clone(),
            range: 0..2,
            allocations: vec![(Target::Object(first), 3), (Target::Object(second), 2)],
        };
        let mut dm = Pause::default();
        let mut ctx = ExecutionContext::new(source, a, &mut dm)
            .with_targets(vec![
                crate::effects::ResolvedTarget::Object(first),
                crate::effects::ResolvedTarget::Object(second),
            ])
            .with_target_distributions(vec![saved.clone()]);
        let history = game.turn_store.turn_history.event_records.len();
        let effect = DealDistributedDamageEffect::new(5, target);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(outcome.events.is_empty());
        assert_eq!(ctx.target_distributions, vec![saved.clone()]);
        assert_eq!(ctx.source, source);
        assert_eq!(game.damage_on(first), 0);
        assert_eq!(game.damage_on(second), 0);
        assert_eq!(game.turn_store.turn_history.event_records.len(), history);
        assert!(!game.effect_store.has_pending_trigger_work());
        drop(ctx);
        let mut ctx = ExecutionContext::new_default(source, a)
            .with_targets(vec![
                crate::effects::ResolvedTarget::Object(first),
                crate::effects::ResolvedTarget::Object(second),
            ])
            .with_target_distributions(vec![saved]);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.damage_on(first), 3);
        assert!(matches!(game.damage_on(second), 5 | 6));
        let damage = outcome
            .events
            .iter()
            .filter(|event| event.downcast::<crate::events::DamageEvent>().is_some())
            .collect::<Vec<_>>();
        assert_eq!(damage.len(), 2);
        assert_eq!(
            damage[0].simultaneous_batch(),
            damage[1].simultaneous_batch()
        );
    }

    #[test]
    fn unrepresentable_announced_total_is_incomplete_and_preserves_announcement() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let a = PlayerId::from_index(0);
        let source = creature(&mut game, "Source", a, 1, None);
        let first = creature(&mut game, "First", a, 1, None);
        let second = creature(&mut game, "Second", a, 1, None);
        let target = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::up_to(2));
        let saved = crate::game_state::TargetDistribution {
            spec: target.clone(),
            range: 0..2,
            allocations: vec![
                (Target::Object(first), u32::MAX),
                (Target::Object(second), 1),
            ],
        };
        let mut ctx = ExecutionContext::new_default(source, a)
            .with_targets(vec![
                crate::effects::ResolvedTarget::Object(first),
                crate::effects::ResolvedTarget::Object(second),
            ])
            .with_target_distributions(vec![saved.clone()]);
        let outcome =
            DealDistributedDamageEffect::new(Value::X, target).execute(&mut game, &mut ctx);
        assert!(matches!(
            outcome,
            Err(ExecutionError::ResourceLimitExceeded { .. })
        ));
        assert_eq!(ctx.target_distributions, vec![saved]);
        assert_eq!(game.damage_on(first), 0);
        assert_eq!(game.damage_on(second), 0);
    }
}
