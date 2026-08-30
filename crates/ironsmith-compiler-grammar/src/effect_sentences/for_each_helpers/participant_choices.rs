use super::*;

/// Parse a participant-owned creature-type choice followed by another action.
///
/// This grammar owns only a complete `choose[s] a creature type` phrase. The
/// participant object-choice grammar owns object domains instead, so the two
/// routes are structurally disjoint even when a coordinated action follows.
pub(super) fn parse_participant_creature_type_choice(
    tokens: &[OwnedLexToken],
    chooser: PlayerAst,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let and_idx = crate::slice_primitives::select_position(tokens, |token| token.is_word("and"));
    let choice_end = and_idx.unwrap_or(tokens.len());
    let choice_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[..choice_end]);
    let choice_words = crate::lexer::token_word_refs(choice_tokens);
    let Some(parsed) = crate::grammar::choices::parse_choice_creature_type_phrase_words(
        &choice_words,
    )
    .map_err(|error| {
        CardTextError::ParseError(format!(
            "invalid participant creature-type choice ({error:?})"
        ))
    })?
    else {
        return Ok(None);
    };
    if parsed.consumed != choice_words.len() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::subject_verb_choose_creature_type(
        chooser,
        parsed.excluded_subtypes,
    )];
    let Some(and_idx) = and_idx else {
        return Ok(Some(effects));
    };
    let tail = crate::util::trim_edge_punctuation_tokens(&tokens[and_idx + 1..]);
    if tail.is_empty() {
        return Ok(None);
    }
    let tail = if chooser == PlayerAst::That {
        prepend_that_player_subject(tail)
    } else {
        tail.to_vec()
    };
    effects.extend(parse_effect_chain_inner(&tail)?);
    Ok(Some(effects))
}

pub(super) fn parse_participant_choice_complement_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut full_clause = Vec::with_capacity(tokens.len() + 2);
    full_clause.push(OwnedLexToken::word(
        "each".to_string(),
        TextSpan::synthetic(),
    ));
    full_clause.push(OwnedLexToken::word(
        "player".to_string(),
        TextSpan::synthetic(),
    ));
    full_clause.extend_from_slice(tokens);

    let Some(effect) = super::super::parse_choice_complement_subject_verb(&full_clause)? else {
        return Ok(None);
    };
    let EffectAst::ForEachPlayer { effects } = effect else {
        return Ok(None);
    };
    Ok(Some(effects))
}
