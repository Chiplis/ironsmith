//! Phase-out effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that phases permanents out.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseOutEffect {
    /// What to phase out - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl PhaseOutEffect {
    /// Create a phase-out effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted phase-out effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted phase-out effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted phase-out effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a phase-out effect that phases out the source permanent.
    pub fn source() -> Self {
        Self {
            spec: ChooseSpec::Source,
        }
    }
}

impl EffectExecutor for PhaseOutEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result_policy = if self.spec.is_target() && self.spec.is_single() {
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid
        } else {
            ObjectApplyResultPolicy::CountApplied
        };

        let apply_result = apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            result_policy,
            |game, _ctx, object_id| {
                if game
                    .object(object_id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                    && !game.is_phased_out(object_id)
                {
                    game.phase_out(object_id);
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )?;

        Ok(apply_result.outcome)
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
        "permanent to phase out"
    }
}
