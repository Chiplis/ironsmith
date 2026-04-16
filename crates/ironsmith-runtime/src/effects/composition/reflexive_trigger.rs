//! Reflexive trigger effect implementation.

use crate::decisions::context::{TargetRequirementContext, TargetsContext};
use crate::effect::{Effect, EffectId, EffectOutcome, EffectPredicate, EffectPredicateRuntimeExt};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::target::ChooseSpec;
use crate::targeting::normalize_targets_for_requirements;

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
        let selected = ctx.decision_maker.decide_targets(game, &targets_ctx);
        let selected = normalize_targets_for_requirements(&targets_ctx.requirements, selected)?;

        chosen_targets.extend(selected);
    }

    Some(chosen_targets)
}

#[cfg(test)]
mod tests {
    use super::choose_reflexive_targets;
    use crate::cards::definitions::{grizzly_bears, lightning_bolt};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::TargetsContext;
    use crate::effect::ChoiceCount;
    use crate::effects::ExecutionContext;
    use crate::game_state::{GameState, Target};
    use crate::ids::PlayerId;
    use crate::target::ChooseSpec;
    use crate::zone::Zone;

    struct DuplicateTargetDecisionMaker {
        target: Target,
    }

    impl DecisionMaker for DuplicateTargetDecisionMaker {
        fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
            vec![self.target, self.target]
        }
    }

    #[test]
    fn reflexive_targets_are_normalized_per_requirement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&lightning_bolt(), alice, Zone::Stack);
        let first = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);

        let mut dm = DuplicateTargetDecisionMaker {
            target: Target::Object(first),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let choices =
            vec![ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2))];

        let selected = choose_reflexive_targets(&game, &mut ctx, &choices).expect("valid targets");

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], Target::Object(first));
        assert_eq!(selected[1], Target::Object(second));
    }
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
        let outcome = ctx
            .get_outcome(self.condition)
            .ok_or(ExecutionError::EffectNotFound(self.condition))?;
        if !self.predicate.evaluate_outcome(outcome) {
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
        if let Some(defending_player) = ctx.combat.defending_player {
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
