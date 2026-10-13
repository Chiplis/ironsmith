//! Set base power/toughness effect implementation.

use crate::continuous::EffectTarget;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::continuous::ResolvedPowerToughness;
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::SetBasePowerToughnessEffect;

/// Effect that sets a creature's base power and toughness.
///
/// Creates a continuous effect in layer 7b ("setting" sublayer).
impl EffectExecutor for SetBasePowerToughnessEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let values = ResolvedPowerToughness::resolve(game, ctx, &self.power, &self.toughness)?;

        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

        if !game.current_is_creature(target_id) {
            return Ok(EffectOutcome::target_invalid());
        }
        // An undated resolution effect still belongs in layer 7b. Rewriting
        // the object's copiable base fields would make later copies inherit
        // this assignment and would order it incorrectly against copy effects.
        let apply = values.setting(EffectTarget::Specific(target_id), self.duration.clone());
        execute_effect(game, &Effect::new(apply), ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature"
    }
}
