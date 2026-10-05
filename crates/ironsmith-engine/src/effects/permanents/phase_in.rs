//! Phase-in effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that phases permanents in.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseInEffect {
    /// What to phase in - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
    /// Both sets are captured from one pre-instruction state.
    pub simultaneous_phase_out: Option<ObjectFilter>,
}

impl PhaseInEffect {
    /// Create a phase-in effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec, simultaneous_phase_out: None }
    }

    /// Create a targeted phase-in effect.
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
            simultaneous_phase_out: None,
        }
    }

    /// Create a non-targeted phase-in effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
            simultaneous_phase_out: None,
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
            let outgoing = if let Some(out) = &self.simultaneous_phase_out {
                game.battlefield.iter().copied().filter(|id| {
                    !game.is_phased_out(*id) && game.can_phase_out(*id)
                        && game.object(*id).is_some_and(|object| out.matches(object, &filter_ctx, game))
                }).collect::<Vec<_>>()
            } else { Vec::new() };
            // Legality and both filters were evaluated above without mutation.
            // One producer owns all flags, attachment trees and event lookback.
            game.phase_simultaneously(&outgoing, &candidates);
            let phased_in = candidates
                .into_iter()
                .filter(|id| !game.is_phased_out(*id))
                .collect::<Vec<_>>();
            let phased_out = outgoing.iter().filter(|id| game.is_phased_out(**id)).count();
            return Ok(EffectOutcome::count((phased_in.len() + phased_out) as i32));
        }
        if self.simultaneous_phase_out.is_some() {
            // Exchange payloads require the explicit non-targeted All domain.
            return Err(ExecutionError::InvalidTarget);
        }
        let result_policy = if self.spec.is_target() && self.spec.is_single() {
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid
        } else {
            ObjectApplyResultPolicy::CountApplied
        };

        let mut affected = Vec::new();
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
                    affected.push(object_id);
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )?;

        game.phase_in_simultaneously(&affected);
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
