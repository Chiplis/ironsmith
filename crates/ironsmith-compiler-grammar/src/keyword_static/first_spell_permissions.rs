//! "You may cast the first creature spell you cast each turn as though it had
//! flash." (The Blue Spirit): flash timing (CR 702.8a) for the first spell
//! matching the filter that you cast each turn, from any origin the spell is
//! otherwise castable from. The same typed first-spell filter drives the
//! "costs less and can be cast as though it had flash" permission.
use super::*;

pub fn parse_first_spell_flash_permission_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clean = trim_edge_punctuation_tokens(tokens);
    let Some(((), rest)) = crate::grammar::primitives::parse_prefix(
        clean,
        crate::grammar::primitives::phrase(&["you", "may", "cast"]),
    ) else {
        return Ok(None);
    };
    let Some((body, ())) = crate::grammar::primitives::split_lexed_once_before_suffix(rest, 1, || {
        crate::grammar::primitives::phrase(&["as", "though", "it", "had", "flash"])
    }) else {
        return Ok(None);
    };
    let Some(parsed) = crate::grammar::anthem_grants::parse_first_spell_each_turn_clause(body)
    else {
        return Ok(None);
    };
    if parsed.mana_source_tokens.is_some() {
        return Ok(None);
    }
    let Ok(mut filter) = crate::object_filters::parse_object_filter_lexed(parsed.filter_tokens, false)
    else {
        return Ok(None);
    };
    if !filter.has_mana_cost
        && filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        && filter.zone != Some(Zone::Stack)
    {
        return Ok(None);
    }
    filter.cast_by = Some(PlayerFilter::You);
    filter.first_spell_cast_each_turn = true;
    Ok(Some(StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::flash_timing_for_spells_matching(filter),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_creature_spell_flash_is_a_first_spell_timing_grant() {
        let tokens = crate::lexer::lex_line(
            "You may cast the first creature spell you cast each turn as though it had flash.",
            0,
        )
        .unwrap();
        let ability = parse_first_spell_flash_permission_line(&tokens)
            .unwrap()
            .expect("first-spell flash permission");
        let text = format!("{ability:?}");
        assert!(text.contains("first_spell_cast_each_turn: true"), "{text}");
        assert!(text.contains("Creature"), "{text}");
    }
}
