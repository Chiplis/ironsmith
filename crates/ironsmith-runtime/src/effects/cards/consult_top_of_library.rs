use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{EffectExecutor, consult_helpers::*};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
pub use ironsmith_core::{ConsultTopOfLibraryEffect, ConsultTopOfLibraryStopRule};

impl EffectExecutor for ConsultTopOfLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let filter_ctx = ctx.filter_context(game);
        let stop_rule = match &self.stop_rule {
            ConsultTopOfLibraryStopRule::FirstMatch => LibraryConsultStopRule::FirstMatch,
            ConsultTopOfLibraryStopRule::MatchCount(value) => {
                let resolved = resolve_value(game, value, ctx)?.max(0) as u32;
                LibraryConsultStopRule::MatchCount(resolved)
            }
        };

        let result = execute_library_consult(
            game,
            ctx,
            player,
            self.mode,
            stop_rule,
            Some(&self.all_tag),
            Some(&self.match_tag),
            |object, game| self.filter.matches(object, &filter_ctx, game),
        )?;

        if result.exposed_object_ids.is_empty() {
            Ok(EffectOutcome::count(0))
        } else {
            Ok(EffectOutcome::with_objects(result.exposed_object_ids))
        }
    }
}
