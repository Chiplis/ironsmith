//! Phase-out effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::effects::{ExecutionContext, ExecutionError};
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
                    && game.can_phase_out(object_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    #[test]
    fn phase_out_effect_respects_cant_phase_out_restriction() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(21), "Rooted Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let permanent_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.effect_store
            .cant_effects
            .cant_phase_out
            .insert(permanent_id);

        let mut ctx = ExecutionContext::new_default(permanent_id, alice);
        PhaseOutEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("phase-out effect should resolve");

        assert!(
            !game.is_phased_out(permanent_id),
            "restricted permanent should not phase out"
        );
    }
}
