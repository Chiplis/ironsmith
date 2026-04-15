//! Prevent-the-next-time damage replacement effect.
//!
//! Supports patterns like:
//! - "The next time a source of your choice would deal damage to you this turn, prevent that damage."
//! - "The next time a red source would deal damage to any target this turn, prevent that damage."

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::events::damage::matchers::{
    DamageSourceConstraint, DamageTargetConstraint, PreventableDamageConstraintMatcher,
};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ObjectFilter;

/// How to constrain which source's damage is prevented.
#[derive(Debug, Clone, PartialEq)]
pub enum PreventNextTimeDamageSource {
    /// Choose a specific source as the effect resolves ("a source of your choice").
    Choice,
    /// Match any source that satisfies a filter ("a red source", "an artifact source", etc.).
    Filter(ObjectFilter),
}

/// How to constrain which target is protected.
#[derive(Debug, Clone, PartialEq)]
pub enum PreventNextTimeDamageTarget {
    /// Any damage target.
    AnyTarget,
    /// Only damage that would be dealt to you.
    You,
}

/// Register a one-shot replacement effect that prevents the next damage event matching constraints.
///
/// One-shot effects are cleaned up at end of turn by the cleanup step, and consumed after use.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventNextTimeDamageEffect {
    pub source: PreventNextTimeDamageSource,
    pub target: PreventNextTimeDamageTarget,
}

impl PreventNextTimeDamageEffect {
    pub fn new(source: PreventNextTimeDamageSource, target: PreventNextTimeDamageTarget) -> Self {
        Self { source, target }
    }
}

impl EffectExecutor for PreventNextTimeDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_constraint = match self.target {
            PreventNextTimeDamageTarget::AnyTarget => DamageTargetConstraint::Any,
            PreventNextTimeDamageTarget::You => DamageTargetConstraint::Player(ctx.controller),
        };

        let source_constraint = match &self.source {
            PreventNextTimeDamageSource::Filter(filter) => {
                DamageSourceConstraint::Filter(filter.clone())
            }
            PreventNextTimeDamageSource::Choice => {
                // Choose from a pragmatic union of likely "sources": stack objects + permanents.
                // This is not perfect MTG coverage, but it captures the primary gameplay cases.
                let mut candidates = Vec::new();
                candidates.extend(game.stack.iter().map(|e| e.object_id));
                candidates.extend(game.battlefield.iter().copied());
                candidates.sort_by_key(|id| id.0);
                candidates.dedup();

                if candidates.is_empty() {
                    return Ok(EffectOutcome::resolved());
                }

                let selectable = candidates
                    .iter()
                    .copied()
                    .map(|id| {
                        let name = game
                            .object(id)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| format!("object {}", id.0));
                        crate::decisions::context::SelectableObject::new(id, name)
                    })
                    .collect::<Vec<_>>();
                let select_ctx = crate::decisions::context::SelectObjectsContext::new(
                    ctx.controller,
                    Some(ctx.source),
                    "Choose a source",
                    selectable,
                    1,
                    Some(1),
                );
                let chosen = ctx
                    .decision_maker
                    .decide_objects(game, &select_ctx)
                    .into_iter()
                    .next()
                    .unwrap_or(candidates[0]);
                DamageSourceConstraint::Specific(chosen)
            }
        };

        let matcher = PreventableDamageConstraintMatcher {
            source: source_constraint,
            target: target_constraint,
        };
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            matcher,
            ReplacementAction::Prevent,
        );

        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::event_processor::process_damage_with_event;
    use crate::events::traits::ReplacementMatcher;
    use crate::game_event::DamageTarget;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn prevent_next_time_damage_choice_any_target_prevents() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);

        // Make the source a permanent so it's in the candidate pool.
        game.battlefield.push(source);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let effect = PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::Choice,
            PreventNextTimeDamageTarget::AnyTarget,
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        let (final_damage, prevented) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        );
        assert_eq!(final_damage, 0);
        assert!(prevented);
    }

    #[test]
    fn prevent_next_time_damage_choice_to_you_only_affects_you() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);

        game.battlefield.push(source);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let effect = PreventNextTimeDamageEffect::new(
            PreventNextTimeDamageSource::Choice,
            PreventNextTimeDamageTarget::You,
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        // Damage to Alice is prevented.
        let (final_damage, prevented) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        );
        assert_eq!(final_damage, 0);
        assert!(prevented);

        // Damage to Bob is not prevented (replacement is consumed or doesn't match target).
        let (final_damage, prevented) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::from_effect(source, alice),
        );
        assert_eq!(final_damage, 3);
        assert!(!prevented);
    }

    #[test]
    fn prevent_next_time_damage_matcher_does_not_match_unpreventable_damage() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(10);

        let matcher = PreventableDamageConstraintMatcher::from_specific_source(
            source,
            DamageTargetConstraint::Player(alice),
        );
        let ctx = crate::events::EventContext::for_replacement_effect(alice, source, &game);

        let unpreventable = crate::events::DamageEvent::unpreventable_with_cause(
            source,
            DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        );
        assert!(
            !matcher.matches_event(&unpreventable, &ctx),
            "matcher should not match unpreventable damage"
        );
    }
}
