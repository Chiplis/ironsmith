//! Shared prevent-all-combat-damage wrapper.

use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionTarget};
use crate::target::ChooseSpec;
pub use ironsmith_core::{CombatDamagePreventionTarget, PreventAllCombatDamageEffect};

impl EffectExecutor for PreventAllCombatDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        match &self.target {
            CombatDamagePreventionTarget::All => super::PreventAllDamageEffect::all_with_filter(
                DamageFilter::combat(),
                self.until.clone(),
            )
            .execute_child(game, ctx),
            CombatDamagePreventionTarget::Players => super::PreventAllDamageEffect::new(
                PreventionTarget::Players,
                DamageFilter::combat(),
                self.until.clone(),
            )
            .execute_child(game, ctx),
            CombatDamagePreventionTarget::You => super::PreventAllDamageEffect::new(
                PreventionTarget::You,
                DamageFilter::combat(),
                self.until.clone(),
            )
            .execute_child(game, ctx),
            CombatDamagePreventionTarget::From(source) => {
                super::PreventAllCombatDamageFromEffect::new(source.clone(), self.until.clone())
                    .execute_child(game, ctx)
            }
            CombatDamagePreventionTarget::ToAndFrom(object) => {
                // CR 615.1: two shields — combat damage the object would deal
                // and combat damage that would be dealt to it.
                let from = super::PreventAllCombatDamageFromEffect::new(
                    object.clone(),
                    self.until.clone(),
                )
                .execute_child(game, ctx)?;
                let to =
                    super::PreventAllDamageToTargetEffect::new(object.clone(), self.until.clone())
                        .with_filter(DamageFilter::combat())
                        .execute_child(game, ctx)?;
                Ok(EffectOutcome::aggregate([from, to])).or_else(|error| match error {
                    // The object is gone: nothing left to protect.
                    ExecutionError::InvalidTarget => Ok(EffectOutcome::resolved()),
                    other => Err(other),
                })
            }
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        match &self.target {
            CombatDamagePreventionTarget::From(source)
            | CombatDamagePreventionTarget::ToAndFrom(source)
                if source.is_target() =>
            {
                Some(source)
            }
            _ => None,
        }
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        match &self.target {
            CombatDamagePreventionTarget::From(source)
            | CombatDamagePreventionTarget::ToAndFrom(source)
                if source.is_target() =>
            {
                Some(source.count())
            }
            _ => None,
        }
    }

    fn target_description(&self) -> &'static str {
        "source to prevent combat damage from"
    }
}
