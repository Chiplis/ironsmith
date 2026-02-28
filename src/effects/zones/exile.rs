//! Exile effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    ObjectApplyResultPolicy, apply_single_target_object_from_context, apply_to_selected_objects,
};
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that exiles permanents.
///
/// Exile moves an object to the exile zone, subject to replacement effects.
/// Unlike destroy, exile is not affected by indestructible.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Exile target creature (targeted - can fizzle)
/// let effect = ExileEffect::target(ChooseSpec::creature());
///
/// // Exile all creatures (non-targeted - cannot fizzle)
/// let effect = ExileEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExileEffect {
    /// What to exile - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
    /// Whether exiled objects should be turned face down in exile.
    pub face_down: bool,
}

impl ExileEffect {
    /// Create an exile effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self {
            spec,
            face_down: false,
        }
    }

    /// Mark exiled cards as face down.
    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    /// Create a targeted exile effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self::with_spec(ChooseSpec::target(spec))
    }

    /// Create a targeted exile effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self::with_spec(ChooseSpec::target(spec).with_count(count))
    }

    /// Create a non-targeted exile effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self::with_spec(ChooseSpec::all(filter))
    }

    /// Create an exile effect targeting a single creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create an exile effect targeting a single permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Create an exile effect targeting any number of targets.
    pub fn any_number(target: ChooseSpec) -> Self {
        Self::targets(target, ChoiceCount::any_number())
    }

    /// Create an exile effect for a specific object.
    pub fn specific(object_id: crate::ids::ObjectId) -> Self {
        Self::with_spec(ChooseSpec::SpecificObject(object_id))
    }

    /// Helper for convenience constructors that mirror ExileAllEffect.
    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    /// Create an effect that exiles all nonland permanents.
    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }

    /// Helper to exile a single object (shared logic).
    fn exile_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
        face_down: bool,
    ) -> Result<Option<EffectResult>, ExecutionError> {
        if let Some(obj) = game.object(object_id) {
            let from_zone = obj.zone;

            // Process through replacement effects with decision maker
            let result = process_zone_change(
                game,
                object_id,
                from_zone,
                Zone::Exile,
                &mut ctx.decision_maker,
            );

            match result {
                EventOutcome::Prevented => return Ok(Some(EffectResult::Prevented)),
                EventOutcome::Proceed(final_zone) => {
                    if let Some(new_id) = game.move_object(object_id, final_zone)
                        && final_zone == Zone::Exile
                    {
                        if face_down {
                            game.set_face_down(new_id);
                        }
                        game.add_exiled_with_source_link(ctx.source, new_id);
                    }
                    return Ok(None); // Successfully exiled
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

    /// Check if spec uses ctx.targets (Object/Player/AnyTarget filters)
    fn uses_ctx_targets(&self) -> bool {
        matches!(
            self.spec.base(),
            ChooseSpec::Object(_) | ChooseSpec::Player(_) | ChooseSpec::AnyTarget
        )
    }
}

impl EffectExecutor for ExileEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        // BUT skip for special specs (Tagged, Source, SpecificObject) which don't use ctx.targets
        if self.spec.is_target() && self.uses_ctx_targets() {
            let count = self.spec.count();
            if count.is_single() {
                return apply_single_target_object_from_context(
                    game,
                    ctx,
                    |game, ctx, object_id| Self::exile_object(game, ctx, object_id, self.face_down),
                );
            }
            // Multi-target with count - handle "any number" specially
            if count.min == 0 {
                // "any number" effects - 0 targets is valid
                let mut exiled_count = 0;
                for target in ctx.targets.clone() {
                    if let ResolvedTarget::Object(object_id) = target
                        && Self::exile_object(game, ctx, object_id, self.face_down)?.is_none()
                    {
                        exiled_count += 1;
                    }
                }
                return Ok(EffectOutcome::count(exiled_count));
            }
        }

        // For all/non-targeted effects and special specs (Tagged, Source, etc.),
        // count successful moves to exile.
        let apply_result = match apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, ctx, object_id| {
                if let Some(new_id) = game.move_object(object_id, Zone::Exile) {
                    if self.face_down {
                        game.set_face_down(new_id);
                    }
                    game.add_exiled_with_source_link(ctx.source, new_id);
                    Ok(true)
                } else {
                    Ok(false)
                }
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
        "target to exile"
    }
}
