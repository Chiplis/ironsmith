//! Reflexive trigger effect implementation.

use crate::decisions::context::{TargetRequirementContext, TargetsContext};
use crate::effect::{Effect, EffectId, EffectOutcome, EffectPredicate, EffectPredicateRuntimeExt};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry, TargetAssignment};
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
) -> Option<(Vec<crate::game_state::Target>, Vec<TargetAssignment>)> {
    let mut chosen_targets = Vec::new();
    let mut assignments = Vec::new();

    for spec in choices {
        let count = spec.count();
        let (min_targets, max_targets) = if count.dynamic_x || spec.count_value().is_some() {
            let resolved = if let Some(value) = spec.count_value() {
                crate::effects::helpers::resolve_value(game, value, ctx).ok()?.max(0) as usize
            } else {
                ctx.x_value? as usize
            };
            if count.up_to_x {
                (0, Some(resolved))
            } else {
                (resolved, Some(resolved))
            }
        } else {
            (count.min, count.max)
        };
        let legal_targets = crate::targeting::compute_legal_targets_with_tagged_objects(
            game,
            spec,
            ctx.controller,
            Some(ctx.source),
            Some(&ctx.tagged_objects),
        );

        if legal_targets.len() < min_targets {
            return None;
        }

        let targets_ctx = TargetsContext::new(
            ctx.controller,
            ctx.source,
            "reflexive triggered ability",
            vec![TargetRequirementContext {
                description: describe_choice(spec),
                legal_targets: legal_targets.clone(),
                min_targets,
                max_targets,
            }],
        );
        let selected = ctx.decision_maker.decide_targets(game, &targets_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        let selected = normalize_targets_for_requirements(&targets_ctx.requirements, selected)?;

        let start = chosen_targets.len();
        chosen_targets.extend(selected);
        let end = chosen_targets.len();
        assignments.push(TargetAssignment {
            spec: spec.clone(),
            range: start..end,
        });
    }

    Some((chosen_targets, assignments))
}

#[cfg(test)]
mod tests {
    use super::{ReflexiveTriggerEffect, choose_reflexive_targets};
    use crate::cards::definitions::{grizzly_bears, lightning_bolt};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::TargetsContext;
    use crate::effect::{ChoiceCount, Effect, EffectId, EffectOutcome, EffectPredicate};
    use crate::effects::{EffectExecutor, ExecutionContext};
    use crate::game_state::{GameState, Target};
    use crate::ids::PlayerId;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
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

    #[cfg(ironsmith_runtime_parser_tests)]
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

        let (selected, assignments) =
            choose_reflexive_targets(&game, &mut ctx, &choices).expect("valid targets");

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], Target::Object(first));
        assert_eq!(selected[1], Target::Object(second));
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].range, 0..2);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn reflexive_trigger_pushes_stack_ability_with_captured_context() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source =
            game.create_object_from_definition(&lightning_bolt(), alice, Zone::Battlefield);
        let tagged = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let tagged_snapshot =
            ObjectSnapshot::from_object(game.object(tagged).expect("tagged object"), &game);

        let condition = EffectId(77);
        let reflexive = ReflexiveTriggerEffect::new(
            condition,
            EffectPredicate::Happened,
            vec![Effect::draw(1)],
            Vec::new(),
        );

        let mut dm = DuplicateTargetDecisionMaker {
            target: Target::Object(tagged),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.store_outcome(condition, EffectOutcome::count(1));
        ctx.x_value = Some(7);
        ctx.combat.defending_player = Some(bob);
        ctx.set_tagged_objects(TagKey::from("sacrificed"), vec![tagged_snapshot.clone()]);

        reflexive
            .execute(&mut game, &mut ctx)
            .expect("reflexive trigger should push a stack ability");

        let entry = game.stack.last().expect("reflexive ability on stack");
        assert!(entry.is_ability);
        assert_eq!(entry.object_id, source);
        assert_eq!(entry.controller, alice);
        assert_eq!(entry.x_value, Some(7));
        assert_eq!(entry.defending_player, Some(bob));
        let captured = entry
            .tagged_objects
            .get(&TagKey::from("sacrificed"))
            .expect("tagged object snapshots carried to stack entry");
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].object_id, tagged_snapshot.object_id);
        assert_eq!(
            entry.source_name.as_deref(),
            Some(game.object(source).expect("source object").name.as_str())
        );
        assert!(entry.source_snapshot.is_some());
    }
}

impl EffectExecutor for ReflexiveTriggerEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = ctx
            .get_outcome(self.condition)
            .ok_or(ExecutionError::EffectNotFound(self.condition))?
            .clone();
        if !self.predicate.evaluate_outcome(&outcome) {
            return Ok(EffectOutcome::resolved());
        }

        let (targets, target_assignments) = choose_reflexive_targets(game, ctx, &self.choices)
            .ok_or(ExecutionError::InvalidTarget)?;

        let mut entry = StackEntry::ability(ctx.source, ctx.controller, self.effects.clone())
            .with_targets(targets)
            .with_target_assignments(target_assignments)
            .with_optional_costs_paid(ctx.optional_costs_paid.clone())
            .with_tagged_objects(ctx.tagged_objects.clone())
            .with_effect_outcomes(std::collections::HashMap::from([(
                self.condition,
                outcome,
            )]));

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
