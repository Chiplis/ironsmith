//! Return all matching cards to the battlefield.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// Effect that returns all matching cards to the battlefield.
///
/// This is used by clauses like "Return all creature cards from all graveyards
/// to the battlefield tapped under their owners' control."
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnAllToBattlefieldEffect {
    /// Filter used to select cards to return.
    pub filter: ObjectFilter,
    /// Whether the returned permanents enter tapped.
    pub tapped: bool,
}

impl ReturnAllToBattlefieldEffect {
    /// Create a new return-all effect.
    pub fn new(filter: ObjectFilter, tapped: bool) -> Self {
        Self { filter, tapped }
    }
}

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
            let Some(owner) = game.object(object_id).map(|obj| obj.owner) else {
                continue;
            };

            if let Some(result) = game.move_object_with_etb_processing_with_dm(
                object_id,
                Zone::Battlefield,
                &mut ctx.decision_maker,
            ) {
                let new_id = result.new_id;
                if let Some(obj) = game.object_mut(new_id) {
                    // Oracle wording for these clauses uses owners' control.
                    obj.controller = owner;
                }
                if self.tapped && !result.enters_tapped {
                    game.tap(new_id);
                }
                returned_count += 1;
            }
        }

        Ok(EffectOutcome::count(returned_count))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
