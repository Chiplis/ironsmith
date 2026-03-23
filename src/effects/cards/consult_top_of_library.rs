use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{EffectExecutor, consult_helpers::*};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};

#[derive(Debug, Clone, PartialEq)]
pub enum ConsultTopOfLibraryStopRule {
    FirstMatch,
    MatchCount(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsultTopOfLibraryEffect {
    pub player: PlayerFilter,
    pub mode: LibraryConsultMode,
    pub filter: ObjectFilter,
    pub stop_rule: ConsultTopOfLibraryStopRule,
    pub all_tag: TagKey,
    pub match_tag: TagKey,
}

impl ConsultTopOfLibraryEffect {
    pub fn new(
        player: PlayerFilter,
        mode: LibraryConsultMode,
        filter: ObjectFilter,
        stop_rule: ConsultTopOfLibraryStopRule,
        all_tag: impl Into<TagKey>,
        match_tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            player,
            mode,
            filter,
            stop_rule,
            all_tag: all_tag.into(),
            match_tag: match_tag.into(),
        }
    }
}

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
