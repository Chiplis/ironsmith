//! Return all matching cards to the battlefield.

use super::battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
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
        let mut affected_memory = Vec::new();
        for object_id in objects {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let memory =
                OutcomeObjectMemory::from_snapshot(&ObjectSnapshot::from_object(obj, game));

            let outcome = move_to_battlefield_with_options(
                game,
                ctx,
                object_id,
                BattlefieldEntryOptions::owner(self.tapped),
            );

            if matches!(outcome, BattlefieldEntryOutcome::Moved(_)) {
                returned_count += 1;
                affected_memory.push(memory);
            }
        }

        let mut outcome = EffectOutcome::count(returned_count);
        if !affected_memory.is_empty() {
            outcome = outcome.with_affected_object_memory(affected_memory);
        }
        Ok(outcome)
    }
}
