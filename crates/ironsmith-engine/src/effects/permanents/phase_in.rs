//! Phase-in effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Context tag recording the permanents an untargeted "all ... phase in"
/// instruction phased in during the current resolution.
pub(crate) const PHASED_IN_THIS_RESOLUTION_TAG: &str = "__phased_in_this_resolution__";

/// Effect that phases permanents in.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseInEffect {
    /// What to phase in - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl PhaseInEffect {
    /// Create a phase-in effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted phase-in effect.
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a non-targeted phase-in effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }
}

impl EffectExecutor for PhaseInEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // "All phased-out creatures phase in": ordinary filter matching treats
        // phased-out permanents as nonexistent (CR 702.26b), so an untargeted
        // all-spec has to look at the phased-out permanents directly.
        if let ChooseSpec::All(filter) = self.spec.base() {
            use crate::filter::ObjectFilterExt as _;
            let filter_ctx = ctx.filter_context(game);
            let candidates = game
                .battlefield
                .iter()
                .copied()
                .filter(|&object_id| {
                    game.is_phased_out(object_id)
                        && game.can_phase_in(object_id)
                        && game.object(object_id).is_some_and(|object| {
                            object.zone == Zone::Battlefield
                                && filter.matches_internal(object, &filter_ctx, game, true, None)
                        })
                })
                .collect::<Vec<_>>();
            let mut phased_in = Vec::new();
            for object_id in candidates {
                if game.is_phased_out(object_id) {
                    game.phase_in(object_id);
                    if !game.is_phased_out(object_id) {
                        phased_in.push(object_id);
                    }
                }
            }
            // A following "... and all creatures with phasing phase out" in
            // the same instruction happens simultaneously (Time and Tide):
            // the permanents that just phased in must not phase out again.
            let snapshots = phased_in
                .iter()
                .filter_map(|id| game.object(*id))
                .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
                .collect::<Vec<_>>();
            ctx.set_tagged_objects(PHASED_IN_THIS_RESOLUTION_TAG, snapshots);
            return Ok(EffectOutcome::count(phased_in.len() as i32));
        }
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
                    && game.is_phased_out(object_id)
                    && game.can_phase_in(object_id)
                {
                    game.phase_in(object_id);
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

    fn get_target_count(&self) -> Option<ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to phase in"
    }
}
