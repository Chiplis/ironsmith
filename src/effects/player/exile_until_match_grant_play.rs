//! Exile cards from the top of a library until one matches a filter, then
//! grant temporary play permission for that exiled card until end of turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryConsultMode, LibraryConsultStopRule, execute_library_consult,
};
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilMatchGrantPlayEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub caster: PlayerFilter,
}

impl ExileUntilMatchGrantPlayEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, caster: PlayerFilter) -> Self {
        Self {
            player,
            filter,
            caster,
        }
    }
}

impl EffectExecutor for ExileUntilMatchGrantPlayEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let caster_id = resolve_player_filter(game, &self.caster, ctx)?;
        let match_tag = TagKey::from("__exile_until_match_grant_play_match");
        let filter_ctx = ctx.filter_context(game);
        execute_library_consult(
            game,
            ctx,
            player_id,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::FirstMatch,
            None,
            Some(&match_tag),
            |object, game| self.filter.matches(object, &filter_ctx, game),
        )?;

        let Some(candidate_snapshot) = ctx.get_tagged(match_tag.as_str()).cloned() else {
            return Ok(EffectOutcome::count(0));
        };
        let mut candidate_id = candidate_snapshot.object_id;
        if game.object(candidate_id).is_none() {
            if let Some(found) = game.find_object_by_stable_id(candidate_snapshot.stable_id) {
                candidate_id = found;
            } else {
                return Ok(EffectOutcome::count(0));
            }
        }

        game.grant_registry.grant_to_card(
            candidate_id,
            Zone::Exile,
            caster_id,
            Grantable::PlayFrom,
            GrantSource::Effect {
                source_id: ctx.source,
                expires_end_of_turn: game.turn.turn_number,
            },
        );

        Ok(EffectOutcome::with_objects(vec![candidate_id]))
    }
}
