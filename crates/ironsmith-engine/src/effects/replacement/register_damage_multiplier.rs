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
        // "if that creature would deal combat damage to one of your
        // opponents" / "to that player or a permanent that player controls":
        // the object and player this resolution names stay the ones it named
        // (CR 611.2c).
        use crate::effects::player_reference_binding::{
            bind_filter_resolution_references, bind_player_reference,
        };
        let mut ability = crate::static_abilities::DoubleDamageAmountReplacement::new(
            bind_filter_resolution_references(&self.source_filter, game, ctx),
            self.target_player_filter
                .as_ref()
                .map(|player| bind_player_reference(player, game, ctx)),
            self.target_object_filter
                .as_ref()
                .map(|filter| bind_filter_resolution_references(filter, game, ctx)),
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
