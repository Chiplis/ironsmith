//! Ordinary face-down transition, without a zone change or entry/cast method.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type TurnFaceDownEffect = ironsmith_core::TurnFaceDownEffect;

impl EffectExecutor for TurnFaceDownEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        game.clear_pending_decision_controllers();
        crate::effects::composition::execute_result_transaction(game, ctx, |game, ctx| {
            game.refresh_continuous_state()
                .map_err(ExecutionError::ContinuousDiscovery)?;
            // Resolve and lock the complete legal set before changing any
            // characteristics. Losing one permanent's static abilities must
            // not reselect, invalidate, or add another selected permanent.
            let mut targets =
                crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            targets.retain(|id| game.can_turn_face_down_permanent(*id));
            targets.sort();
            targets.dedup();
            let mut turned = 0;
            for id in targets {
                if game.set_face_down(id) {
                    turned += 1;
                }
            }
            // No ETB, zone move, face-up event, manifest/cloak provenance, or
            // disguise ward is created by this operation. Observe the batch
            // only after every selected permanent has changed orientation.
            game.refresh_continuous_state()
                .map_err(ExecutionError::ContinuousDiscovery)?;
            Ok(EffectOutcome::count(turned))
        })
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }
    fn target_description(&self) -> &'static str {
        "permanent to turn face down"
    }
}
