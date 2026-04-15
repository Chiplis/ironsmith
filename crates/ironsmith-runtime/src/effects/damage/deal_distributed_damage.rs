//! Deal distributed damage among multiple targets.

use crate::decision::FallbackStrategy;
use crate::decisions::{DistributeSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::damage::deal_damage::apply_processed_damage_outcome;
use crate::effects::helpers::{
    resolve_objects_from_spec, resolve_players_from_spec, resolve_value,
};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Target};
use crate::target::ChooseSpec;
use crate::types::CardType;
use std::collections::HashMap;

/// Effect that deals a total amount of damage divided among chosen targets.
#[derive(Debug, Clone, PartialEq)]
pub struct DealDistributedDamageEffect {
    /// The total amount of damage to distribute.
    pub amount: Value,
    /// The target specification for the distributed damage choices.
    pub target: ChooseSpec,
}

impl DealDistributedDamageEffect {
    /// Create a new distributed-damage effect.
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            target,
        }
    }
}

impl EffectExecutor for DealDistributedDamageEffect {
    fn execute(
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
            return Ok(EffectOutcome::target_invalid());
        }

        let distribution = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            DistributeSpec::damage(ctx.source, total, available_targets.clone()),
            FallbackStrategy::Maximum,
        );

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

        if distributed_total < total {
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
                Target::Player(player_id) => crate::game_event::DamageTarget::Player(player_id),
                Target::Object(object_id) => crate::game_event::DamageTarget::Object(object_id),
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
            ));
        }

        if outcomes.is_empty() {
            Ok(EffectOutcome::target_invalid())
        } else {
            Ok(EffectOutcome::aggregate_summing_counts(outcomes))
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "targets for distributed damage"
    }
}
