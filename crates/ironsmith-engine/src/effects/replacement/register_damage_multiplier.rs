//! Duration-bound damage replacement created by a resolving spell/ability.
use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbilityKind;
pub type RegisterDamageMultiplierEffect = ironsmith_core::RegisterDamageMultiplierEffect;

impl EffectExecutor for RegisterDamageMultiplierEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut ability = crate::static_abilities::DoubleDamageAmountReplacement::new(
            self.source_filter.clone(),
            self.target_player_filter.clone(),
            self.target_object_filter.clone(),
            self.factor,
            self.combat_only,
            "Resolved damage multiplier",
        );
        if self.noncombat_only {
            ability = ability.noncombat_only();
        }
        let replacement = ability
            .generate_replacement_effect(ctx.source, ctx.controller)
            .expect("damage multiplier always creates a replacement");
        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute_child(game, ctx)
    }
    fn primary_execution_category(&self) -> crate::effects::EffectExecutionCategory {
        crate::effects::EffectExecutionCategory::ReplacementRegistration
    }
}
