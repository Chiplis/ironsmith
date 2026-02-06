//! Grant abilities to a target creature until a duration.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::helpers::find_target_object;
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;

/// Effect that grants one or more abilities to a target creature.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantAbilitiesTargetEffect {
    /// Which creature to grant abilities to.
    pub target: ChooseSpec,
    /// Abilities to grant.
    pub abilities: Vec<StaticAbility>,
    /// Duration for the granted abilities.
    pub duration: Until,
}

impl GrantAbilitiesTargetEffect {
    /// Create a new grant abilities to target effect.
    pub fn new(target: ChooseSpec, abilities: Vec<StaticAbility>, duration: Until) -> Self {
        Self {
            target,
            abilities,
            duration,
        }
    }
}

impl EffectExecutor for GrantAbilitiesTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;
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

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
