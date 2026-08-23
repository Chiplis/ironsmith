//! Grant abilities to a target creature until a duration.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;

/// Effect that grants one or more abilities to a target creature.
pub type GrantAbilitiesTargetEffect = ironsmith_core::GrantAbilitiesTargetEffect<StaticAbility>;

impl EffectExecutor for GrantAbilitiesTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
        if self.abilities.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcomes = Vec::new();
        for ability in &self.abilities {
            let apply = ApplyContinuousEffect::new(
                EffectTarget::Specific(target_id),
                Modification::AddAbility(ability.clone()),
                self.duration.clone(),
            );
            outcomes.push(execute_effect(game, &Effect::new(apply), ctx)?);
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
