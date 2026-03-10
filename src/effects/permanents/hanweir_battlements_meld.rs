//! Hanweir Battlements meld effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;

/// Exiles Hanweir Battlements and Hanweir Garrison, then creates
/// Hanweir, the Writhing Township on the battlefield.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HanweirBattlementsMeldEffect;

impl HanweirBattlementsMeldEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for HanweirBattlementsMeldEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source) = game.object(ctx.source).cloned() else {
            return Err(ExecutionError::ObjectNotFound(ctx.source));
        };

        if source.zone != Zone::Battlefield
            || source.name != "Hanweir Battlements"
            || source.owner != ctx.controller
            || source.controller != ctx.controller
        {
            return Ok(EffectOutcome::resolved());
        }

        let garrison_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.name == "Hanweir Garrison"
                        && obj.owner == ctx.controller
                        && obj.controller == ctx.controller
                })
            });

        let Some(garrison_id) = garrison_id else {
            return Ok(EffectOutcome::resolved());
        };

        let Some(result_def) = crate::cards::builtin_registry().get("Hanweir, the Writhing Township")
        else {
            return Ok(EffectOutcome::resolved());
        };

        if game.move_object(ctx.source, Zone::Exile).is_none() {
            return Ok(EffectOutcome::resolved());
        }
        if game.move_object(garrison_id, Zone::Exile).is_none() {
            return Ok(EffectOutcome::resolved());
        }

        game.create_object_from_definition(result_def, ctx.controller, Zone::Battlefield);

        Ok(EffectOutcome::resolved())
    }
}
