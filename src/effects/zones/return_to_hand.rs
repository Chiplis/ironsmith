//! Return to hand effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    ObjectApplyResultPolicy, apply_single_target_object_from_context, apply_to_selected_objects,
};
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that returns permanents to their owners' hands.
///
/// This is commonly called "bouncing" in MTG terminology.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Return target creature to its owner's hand (targeted - can fizzle)
/// let effect = ReturnToHandEffect::target(ChooseSpec::creature());
///
/// // Return all creatures to their owners' hands (non-targeted - cannot fizzle)
/// let effect = ReturnToHandEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnToHandEffect {
    /// What to return - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl ReturnToHandEffect {
    /// Create a return to hand effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted return to hand effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted return to hand effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted return to hand effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a return to hand effect targeting any creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create a return to hand effect targeting any permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Create an effect that returns all creatures.
    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    /// Create an effect that returns all nonland permanents.
    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }

    /// Helper to return a single object to hand (shared logic).
    fn return_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Result<Option<EffectResult>, ExecutionError> {
        if let Some(obj) = game.object(object_id) {
            let from_zone = obj.zone;

            // Process through replacement effects with decision maker
            let result = process_zone_change(
                game,
                object_id,
                from_zone,
                Zone::Hand,
                &mut ctx.decision_maker,
            );

            match result {
                EventOutcome::Prevented => return Ok(Some(EffectResult::Prevented)),
                EventOutcome::Proceed(final_zone) => {
                    game.move_object(object_id, final_zone);
                    return Ok(None); // Successfully returned
                }
                EventOutcome::Replaced => {
                    // Replacement effects already executed
                    return Ok(Some(EffectResult::Replaced));
                }
                EventOutcome::NotApplicable => {
                    return Ok(Some(EffectResult::TargetInvalid));
                }
            }
        }
        // Object doesn't exist - target is invalid
        Ok(Some(EffectResult::TargetInvalid))
    }
}

impl EffectExecutor for ReturnToHandEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        if self.spec.is_target() && self.spec.is_single() {
            return apply_single_target_object_from_context(game, ctx, |game, ctx, object_id| {
                Self::return_object(game, ctx, object_id)
            });
        }

        // For all/multi-target effects, count successful moves to hand.
        let apply_result = match apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, _ctx, object_id| Ok(game.move_object(object_id, Zone::Hand).is_some()),
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
        "permanent to return"
    }
}
