//! Effect for granting an ability directly to an object.

use crate::ability::Ability;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
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
}

impl EffectExecutor for GrantObjectAbilityEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        crate::ability::visit_owned_effects(&self.ability, visitor);
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let targets = resolve_objects_for_effect(game, ctx, &self.target)
            .map_err(|_| ExecutionError::InvalidTarget)?;
        if targets.is_empty() {
            return Ok(EffectOutcome::default());
        }

        for target_id in targets {
            game.install_authored_object_ability(
                target_id,
                self.ability.clone(),
                if self.allow_duplicates {
                    crate::game_state::AuthoredAbilityDuplicatePolicy::PreserveAll
                } else {
                    crate::game_state::AuthoredAbilityDuplicatePolicy::SuppressEquivalentKindAndZones
                },
            );
        }
        Ok(EffectOutcome::default())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
