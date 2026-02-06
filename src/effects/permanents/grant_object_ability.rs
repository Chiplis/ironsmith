//! Effect for granting an ability directly to an object.

use crate::ability::Ability;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Grants an ability to a target object (typically a permanent on the battlefield).
#[derive(Debug, Clone)]
pub struct GrantObjectAbilityEffect {
    /// Ability to add to the target.
    pub ability: Ability,
    /// Target object receiving the ability.
    pub target: ChooseSpec,
    /// Whether duplicate granted abilities are allowed.
    pub allow_duplicates: bool,
}

impl GrantObjectAbilityEffect {
    pub fn new(ability: Ability, target: ChooseSpec) -> Self {
        Self {
            ability,
            target,
            allow_duplicates: false,
        }
    }

    pub fn to_source(ability: Ability) -> Self {
        Self::new(ability, ChooseSpec::Source)
    }

    fn ability_fingerprint(ability: &Ability) -> String {
        format!(
            "{:?}|{:?}|{:?}",
            ability.kind, ability.functional_zones, ability.text
        )
    }
}

impl EffectExecutor for GrantObjectAbilityEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let targets = resolve_objects_from_spec(game, &self.target, ctx)
            .map_err(|_| ExecutionError::InvalidTarget)?;
        if targets.is_empty() {
            return Ok(EffectOutcome::default());
        }

        for target_id in targets {
            let Some(target) = game.object_mut(target_id) else {
                continue;
            };

            if !self.allow_duplicates {
                let new_fp = Self::ability_fingerprint(&self.ability);
                let already_present = target
                    .abilities
                    .iter()
                    .any(|a| Self::ability_fingerprint(a) == new_fp);
                if already_present {
                    continue;
                }
            }

            target.abilities.push(self.ability.clone());
        }
        Ok(EffectOutcome::default())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
