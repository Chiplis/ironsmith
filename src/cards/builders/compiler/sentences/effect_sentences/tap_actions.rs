use super::*;

pub(crate) fn collapse_leading_signed_pt_modifier_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let sign = match tokens.first()?.kind {
        crate::cards::builders::compiler::lexer::TokenKind::Dash => "-",
        crate::cards::builders::compiler::lexer::TokenKind::Plus => "+",
        _ => return None,
    };
    let modifier = tokens.get(1)?.as_word()?;
    if !modifier.chars().any(|ch| ch == '/') {
        return None;
    }

    let mut collapsed = Vec::with_capacity(tokens.len().saturating_sub(1));
    collapsed.push(OwnedLexToken::word(
        format!("{sign}{modifier}"),
        tokens[0].span(),
    ));
    collapsed.extend(tokens.iter().skip(2).cloned());
    Some(collapsed)
}

pub(crate) fn parse_tap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    if let Some(effect) = parse_tap_or_untap_all(tokens)? {
        return Ok(effect);
    }
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::TapAll { filter });
    }
    // Handle "tap or untap <target>" as a choice between tapping and untapping.
    if tokens.first().is_some_and(|t| t.is_word("or"))
        && tokens.get(1).is_some_and(|t| t.is_word("untap"))
    {
        let target_tokens = &tokens[2..];
        let target = parse_target_phrase(target_tokens)?;
        return Ok(EffectAst::TapOrUntap {
            target: target.clone(),
        });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Tap { target })
}

fn parse_tap_or_untap_all(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if !matches!(words.first().copied(), Some("all" | "each")) {
        return Ok(None);
    }
    let Some(or_idx) = find_word_sequence_start(&words, &["or", "untap", "all"]) else {
        return Ok(None);
    };
    if or_idx <= 1 {
        return Ok(None);
    }

    let left_start = token_index_for_word_index(tokens, 1).unwrap_or(tokens.len());
    let left_end = token_index_for_word_index(tokens, or_idx).unwrap_or(tokens.len());
    let right_start = token_index_for_word_index(tokens, or_idx + 3).unwrap_or(tokens.len());
    if left_start >= left_end || right_start > tokens.len() {
        return Ok(None);
    }

    let left_tokens = trim_commas(&tokens[left_start..left_end]).to_vec();
    let right_tokens = trim_commas(&tokens[right_start..]).to_vec();
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return Ok(None);
    }

    let analyze_type_choice_reference = |tokens: &[OwnedLexToken]| {
        let words = crate::cards::builders::compiler::token_word_refs(tokens);
        let qualifier = find_word_sequence_start(&words, &["of", "the", "chosen", "type"])
            .map(|start| (start, 4usize))
            .or_else(|| {
                find_word_sequence_start(&words, &["of", "chosen", "type"])
                    .map(|start| (start, 3usize))
            })
            .or_else(|| {
                find_word_sequence_start(&words, &["of", "that", "type"])
                    .map(|start| (start, 3usize))
            })
            .or_else(|| {
                find_word_sequence_start(&words, &["that", "type"]).map(|start| (start, 2usize))
            });
        let mentions = qualifier.is_some()
            || contains_word_sequence(&words, &["chosen", "type"])
            || contains_word_sequence(&words, &["that", "type"]);
        let stripped = if let Some((start, len)) = qualifier {
            let token_start = token_index_for_word_index(tokens, start).unwrap_or(tokens.len());
            let token_end = token_index_for_word_index(tokens, start + len).unwrap_or(tokens.len());
            let mut stripped = tokens[..token_start].to_vec();
            stripped.extend_from_slice(&tokens[token_end..]);
            trim_commas(&stripped).to_vec()
        } else {
            trim_commas(tokens).to_vec()
        };
        (stripped, mentions)
    };

    let left_words = crate::cards::builders::compiler::token_word_refs(&left_tokens);
    let right_words = crate::cards::builders::compiler::token_word_refs(&right_tokens);
    let (cleaned_left, left_mentions_chosen_type) = analyze_type_choice_reference(&left_tokens);
    let (cleaned_right, right_mentions_chosen_type) = analyze_type_choice_reference(&right_tokens);

    let mut tap_filter = parse_object_filter(&cleaned_left, false)?;
    let mut untap_filter = parse_object_filter(&cleaned_right, false)?;
    if left_mentions_chosen_type {
        tap_filter.chosen_creature_type = true;
    }
    if right_mentions_chosen_type {
        untap_filter.chosen_creature_type = true;
    }
    if contains_word_sequence(&left_words, &["target", "player", "controls"]) {
        tap_filter.controller = Some(PlayerFilter::target_player());
    }
    if contains_word_sequence(&right_words, &["that", "player", "controls"])
        || contains_word_sequence(&right_words, &["that", "players", "control"])
    {
        untap_filter.controller = tap_filter
            .controller
            .clone()
            .or_else(|| Some(PlayerFilter::target_player()));
    }

    Ok(Some(EffectAst::TapOrUntapAll {
        tap_filter,
        untap_filter,
    }))
}
