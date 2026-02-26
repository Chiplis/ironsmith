//! Redirect the next N damage from this source to a chosen target.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_value};
use crate::events::damage::matchers::DamageToSelfMatcher;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
use crate::target::ChooseSpec;

/// "The next N damage that would be dealt to this permanent this turn is dealt to target creature instead."
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextDamageToTargetEffect {
    pub amount: Value,
    pub target: ChooseSpec,
}

impl RedirectNextDamageToTargetEffect {
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            target,
        }
    }
}

impl EffectExecutor for RedirectNextDamageToTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        if amount == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let target = resolve_objects_from_spec(game, &self.target, ctx)?
            .into_iter()
            .next()
            .ok_or(ExecutionError::InvalidTarget)?;

        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            DamageToSelfMatcher::new(),
            ReplacementAction::RedirectDamageAmount {
                target: RedirectTarget::ToObject(target),
                which: RedirectWhich::First,
                amount,
            },
        );
        game.replacement_effects.add_one_shot_effect(replacement);
        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to receive redirected damage"
    }
}
