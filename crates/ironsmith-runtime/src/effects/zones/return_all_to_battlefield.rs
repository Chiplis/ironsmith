//! Return all matching cards to the battlefield.

use super::battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
pub type ReturnAllToBattlefieldEffect = ironsmith_core::ReturnAllToBattlefieldEffect;

impl EffectExecutor for ReturnAllToBattlefieldEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let spec = ChooseSpec::all(self.filter.clone());
        let objects = resolve_objects_from_spec(game, &spec, ctx)?;

        let mut returned_count = 0;
        for object_id in objects {
            if game.object(object_id).is_none() {
                continue;
            }

            let outcome = move_to_battlefield_with_options(
                game,
                ctx,
                object_id,
                BattlefieldEntryOptions::owner(self.tapped),
            );

            if matches!(outcome, BattlefieldEntryOutcome::Moved(_)) {
                returned_count += 1;
            }
        }

        Ok(EffectOutcome::count(returned_count))
    }
}
