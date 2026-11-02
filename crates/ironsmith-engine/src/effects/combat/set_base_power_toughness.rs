//! Set base power/toughness effect implementation.

use crate::continuous::EffectTarget;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::continuous::ResolvedPowerToughness;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::effects::helpers::resolve_single_object_for_effect;
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
        if ctx.target_assignments.iter().any(|assignment| {
            assignment.spec.base() == self.target.base() && assignment.range.len() > 1
        }) {

        let values = ResolvedPowerToughness::resolve(game, ctx, &self.power, &self.toughness)?;

        let mut apply = values.setting(EffectTarget::AllPermanents, self.duration.clone());
        apply.target_spec = Some(self.target.clone());
        apply.require_creature_target = true;
        return execute_effect(game, &Effect::new(apply), ctx);
        }

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

    fn scalar_target_resolution_specs(&self) -> Vec<ChooseSpec> {
        self.target.is_target().then(|| self.target.clone()).into_iter().collect()
    }

    fn supports_simultaneous_player_action(&self) -> bool { true }

    fn prepare_simultaneous_player_action(
        &self, game: &GameState, ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Freeze each scalar recipient's own contextual quantities before any
        // sibling continuous original can change those characteristics.
        let values = ResolvedPowerToughness::resolve(game, ctx, &self.power, &self.toughness)?;
        let mut apply = values.setting(EffectTarget::AllPermanents, self.duration.clone());
        apply.target_spec = Some(self.target.clone());
        apply.require_creature_target = true;
        apply.prepare_simultaneous_player_action(game, ctx)
    }

    fn supports_additional_target_role(&self, spec: &ChooseSpec) -> bool { self.target == *spec }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature"
    }
}
