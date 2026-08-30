use super::*;

pub(super) fn parse_combat_damage_history_participant(
    inner_tokens: &[OwnedLexToken],
    iteration_filter: PlayerFilter,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(history) =
        for_each_shapes::parse_combat_damage_history_player_clause_shape(inner_tokens)
    else {
        return Ok(None);
    };
    let sources = parse_object_filter(history.source_tokens, false)?;
    let normalized = prepend_that_player_subject(history.effect_tokens);
    let effects = parse_maybe_effects(&normalized, false, true)?;
    Ok(Some(EffectAst::ForEachPlayersFiltered {
        filter: PlayerFilter::was_dealt_combat_damage_by_sources_this_game(
            iteration_filter,
            sources,
        ),
        effects,
    }))
}
