//! Enter attacking effect implementation.

use crate::combat_state::{AttackerInfo, get_attack_target};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that causes a creature to enter the battlefield attacking the same
/// target as the source (if combat is active).
#[derive(Debug, Clone, PartialEq)]
pub struct EnterAttackingEffect {
    pub target: ChooseSpec,
}

impl EnterAttackingEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for EnterAttackingEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let creature_id = find_target_object(&ctx.targets)?;

        if let Some(ref mut combat) = game.combat
            && let Some(target) = get_attack_target(combat, ctx.source).cloned()
        {
            combat.attackers.push(AttackerInfo {
                creature: creature_id,
                target,
            });
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}
