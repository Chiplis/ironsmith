//! Destroy effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    ObjectApplyResultPolicy, apply_single_target_object_from_context, apply_to_selected_objects,
};
use crate::event_processor::{EventOutcome, process_destroy};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};

/// Effect that destroys permanents.
///
/// Destruction moves permanents from the battlefield to the graveyard,
/// subject to replacement effects (regeneration, indestructible, etc.).
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Destroy target creature (targeted - can fizzle)
/// let effect = DestroyEffect::target(ChooseSpec::creature());
///
/// // Destroy all creatures (non-targeted - cannot fizzle)
/// let effect = DestroyEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DestroyEffect {
    /// What to destroy - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl DestroyEffect {
    /// Create a destroy effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted destroy effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted destroy effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted destroy effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a destroy effect targeting any creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create a destroy effect targeting any permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Helper to destroy a single object (shared logic).
    ///
    /// Uses `process_destroy` to handle all destruction logic through
    /// the trait-based event/replacement system with decision maker support.
    fn destroy_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Result<Option<EffectResult>, ExecutionError> {
        let result = process_destroy(game, object_id, Some(ctx.source), &mut ctx.decision_maker);

        match result {
            EventOutcome::Proceed(_) => Ok(None), // Successfully destroyed
            EventOutcome::Prevented => Ok(Some(EffectResult::Protected)),
            EventOutcome::Replaced => Ok(Some(EffectResult::Replaced)),
            EventOutcome::NotApplicable => Ok(Some(EffectResult::TargetInvalid)),
        }
    }
}

impl EffectExecutor for DestroyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        if self.spec.is_target() && self.spec.is_single() {
            return apply_single_target_object_from_context(game, ctx, |game, ctx, object_id| {
                Self::destroy_object(game, ctx, object_id)
            });
        }

        // For all/multi-target effects, count only successful destructions.
        let apply_result = match apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, ctx, object_id| {
                let result =
                    process_destroy(game, object_id, Some(ctx.source), &mut ctx.decision_maker);
                Ok(matches!(result, EventOutcome::Proceed(_)))
            },
        ) {
            Ok(result) => result,
            Err(_) => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };

        Ok(apply_result.outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to destroy"
    }
}
