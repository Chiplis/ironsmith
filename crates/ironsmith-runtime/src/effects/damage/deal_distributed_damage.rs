//! Deal distributed damage among multiple targets.

use crate::decision::FallbackStrategy;
use crate::decisions::{DistributeSpec, make_decision_with_fallback};
use crate::effect::{ChoiceCount, EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::damage::deal_damage::apply_processed_damage_outcome;
use crate::effects::helpers::{
    resolve_objects_from_spec, resolve_players_from_spec, resolve_single_object_for_effect,
    resolve_value,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Target};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::target::PlayerFilter;
use crate::types::CardType;
use std::collections::HashMap;

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
}

impl DealDistributedDamageEffect {
    /// Create a new distributed-damage effect.
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            target,
            source: ChooseSpec::Source,
            chooser: PlayerFilter::You,
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

    fn execute_with_resolved_source(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let damage_source = match resolve_single_object_for_effect(game, ctx, &self.source) {
            Ok(source) => source,
            Err(_) => return Ok(EffectOutcome::target_invalid()),
        };
        let source_snapshot = match self.source.base() {
            ChooseSpec::Tagged(tag) => ctx.get_tagged(tag).cloned(),
            _ => None,
        }
        .or_else(|| {
            game.object(damage_source)
                .map(|object| ObjectSnapshot::from_object(object, game))
        });

        let original_source = ctx.source;
        let original_source_snapshot = ctx.source_snapshot.clone();
        ctx.source = damage_source;
        ctx.source_snapshot = source_snapshot;
        let outcome = self.execute_distribution(game, ctx);
        ctx.source = original_source;
        ctx.source_snapshot = original_source_snapshot;
        outcome
    }

    fn execute_distribution(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let total = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        if total == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let mut available_targets = Vec::new();

        for player_id in resolve_players_from_spec(game, &self.target, ctx).unwrap_or_default() {
            available_targets.push(Target::Player(player_id));
        }

        for object_id in resolve_objects_from_spec(game, &self.target, ctx).unwrap_or_default() {
            if game.object(object_id).is_some_and(|obj| {
                obj.has_card_type(CardType::Creature) || obj.has_card_type(CardType::Planeswalker)
            }) {
                available_targets.push(Target::Object(object_id));
            }
        }

        if available_targets.is_empty() {
            return if self.target.count().min == 0 {
                Ok(EffectOutcome::count(0))
            } else {
                Ok(EffectOutcome::target_invalid())
            };
        }

        let announced_distribution = ctx.take_target_distribution(&self.target);
        let uses_announced_distribution = announced_distribution.is_some();
        let distribution = if let Some(distribution) = announced_distribution {
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
                return Ok(EffectOutcome::count(0));
            }
            distribution
        };
        if distribution.is_empty() && self.target.count().min == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let mut allocations: HashMap<Target, u32> = HashMap::new();
        for (target, amount) in distribution {
            if amount > 0 && available_targets.contains(&target) {
                *allocations.entry(target).or_insert(0) += amount;
            }
        }

        let distributed_total: u32 = allocations.values().copied().sum();
        if distributed_total > total {
            return Ok(EffectOutcome::impossible());
        }

        // CR 608.2b does not let a resolving spell reassign damage that was
        // announced for a target that has since become illegal.
        if !uses_announced_distribution && distributed_total < total {
            let remaining = total - distributed_total;
            if let Some(first_target) = available_targets.first().copied() {
                *allocations.entry(first_target).or_insert(0) += remaining;
            }
        }

        let mut outcomes = Vec::new();
        for (target, amount) in allocations {
            if amount == 0 {
                continue;
            }

            let damage_target = match target {
                Target::Player(player_id) => crate::events::DamageTarget::Player(player_id),
                Target::Object(object_id) => crate::events::DamageTarget::Object(object_id),
            };

            outcomes.push(apply_processed_damage_outcome(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                damage_target,
                amount,
                false,
                ctx.provenance,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ));
        }

        if outcomes.is_empty() {
            Ok(EffectOutcome::target_invalid())
        } else {
            Ok(EffectOutcome::aggregate_summing_counts(outcomes))
        }
    }
}

impl EffectExecutor for DealDistributedDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_resolved_source(game, ctx)
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
        Some(&self.amount)
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
}
