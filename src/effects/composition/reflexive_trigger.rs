//! Reflexive trigger effect implementation.

use crate::decisions::context::{TargetRequirementContext, TargetsContext};
use crate::effect::{Effect, EffectId, EffectOutcome, EffectPredicate};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::target::ChooseSpec;

/// Effect that creates a reflexive triggered ability from a prior effect result.
///
/// This models clauses like "When you do, ..." where the follow-up trigger is
/// created only if an earlier effect satisfied a result predicate, and targets
/// are chosen when that new ability is put onto the stack.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveTriggerEffect {
    /// The prior effect result to inspect.
    pub condition: EffectId,
    /// How to evaluate the prior effect result.
    pub predicate: EffectPredicate,
    /// Effects for the reflexive triggered ability.
    pub effects: Vec<Effect>,
    /// Target choices that must be made when the reflexive ability is created.
    pub choices: Vec<ChooseSpec>,
}

impl ReflexiveTriggerEffect {
    pub fn new(
        condition: EffectId,
        predicate: EffectPredicate,
        effects: Vec<Effect>,
        choices: Vec<ChooseSpec>,
    ) -> Self {
        Self {
            condition,
            predicate,
            effects,
            choices,
        }
    }
}

fn describe_choice(spec: &ChooseSpec) -> String {
    match spec.base() {
        ChooseSpec::Player(_) => "target player".to_string(),
        ChooseSpec::Object(_) => "target object".to_string(),
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => "target".to_string(),
        ChooseSpec::PlayerOrPlaneswalker(_) => "target player or planeswalker".to_string(),
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "target attacked player or planeswalker".to_string()
        }
        _ => "target".to_string(),
    }
}

fn choose_reflexive_targets(
    game: &GameState,
    ctx: &mut ExecutionContext,
    choices: &[ChooseSpec],
) -> Option<Vec<crate::game_state::Target>> {
    let mut chosen_targets = Vec::new();

    for spec in choices {
        let count = spec.count();
        let legal_targets = crate::targeting::compute_legal_targets_with_tagged_objects(
            game,
            spec,
            ctx.controller,
            Some(ctx.source),
            Some(&ctx.tagged_objects),
        );

        if legal_targets.len() < count.min {
            return None;
        }

        let targets_ctx = TargetsContext::new(
            ctx.controller,
            ctx.source,
            "reflexive triggered ability",
            vec![TargetRequirementContext {
                description: describe_choice(spec),
                legal_targets: legal_targets.clone(),
                min_targets: count.min,
                max_targets: count.max,
            }],
        );
        let mut selected = ctx.decision_maker.decide_targets(game, &targets_ctx);

        if selected.len() < count.min {
            selected = legal_targets.iter().take(count.min).copied().collect();
        }
        if let Some(max_targets) = count.max {
            selected.truncate(max_targets);
        }
        if selected.len() < count.min {
            return None;
        }

        chosen_targets.extend(selected);
    }

    Some(chosen_targets)
}

impl EffectExecutor for ReflexiveTriggerEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result = ctx
            .get_result(self.condition)
            .ok_or(ExecutionError::EffectNotFound(self.condition))?;
        if !self.predicate.evaluate(result) {
            return Ok(EffectOutcome::resolved());
        }

        let targets = choose_reflexive_targets(game, ctx, &self.choices)
            .ok_or(ExecutionError::InvalidTarget)?;

        let mut entry = StackEntry::ability(ctx.source, ctx.controller, self.effects.clone())
            .with_targets(targets)
            .with_optional_costs_paid(ctx.optional_costs_paid.clone())
            .with_tagged_objects(ctx.tagged_objects.clone());

        if let Some(x) = ctx.x_value {
            entry = entry.with_x(x);
        }
        if let Some(defending_player) = ctx.defending_player {
            entry = entry.with_defending_player(defending_player);
        }
        if let Some(source) = game.object(ctx.source) {
            entry = entry.with_source_info(source.stable_id, source.name.clone());
        } else if let Some(snapshot) = ctx.source_snapshot.clone() {
            entry = entry
                .with_source_info(snapshot.stable_id, snapshot.name.clone())
                .with_source_snapshot(snapshot);
        }

        game.push_to_stack(entry);
        Ok(EffectOutcome::count(1))
    }
}
