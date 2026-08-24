use super::*;

pub fn split_search_named_item_filters_lexed(
    filter_tokens: &[OwnedLexToken],
    clause_display: &str,
) -> Result<Option<Vec<ObjectFilter>>, CardTextError> {
    if !crate::lexer::contains_token_word(filter_tokens, "named") {
        return Ok(None);
    }

    let mut item_starts = Vec::new();
    let mut cursor = 0usize;
    while cursor < filter_tokens.len() {
        while filter_tokens
            .get(cursor)
            .is_some_and(OwnedLexToken::is_comma)
        {
            cursor += 1;
        }
        if filter_tokens
            .get(cursor)
            .is_some_and(|token| search_library_token_is_any_word(token, &["and"]))
        {
            cursor += 1;
            while filter_tokens
                .get(cursor)
                .is_some_and(OwnedLexToken::is_comma)
            {
                cursor += 1;
            }
        }
        if cursor >= filter_tokens.len() {
            break;
        }

        let item_start = cursor;
        if filter_tokens
            .get(cursor)
            .is_some_and(|token| search_library_token_is_any_word(token, &["a", "an"]))
        {
            cursor += 1;
        }
        if !filter_tokens
            .get(cursor)
            .is_some_and(|token| search_library_token_is_any_word(token, &["card", "cards"]))
            || !filter_tokens
                .get(cursor + 1)
                .is_some_and(|token| search_library_token_is_any_word(token, &["named"]))
        {
            return Ok(None);
        }
        item_starts.push(item_start);
        cursor += 2;

        while cursor < filter_tokens.len() {
            let mut probe = cursor;
            while filter_tokens
                .get(probe)
                .is_some_and(OwnedLexToken::is_comma)
            {
                probe += 1;
            }
            if filter_tokens
                .get(probe)
                .is_some_and(|token| search_library_token_is_any_word(token, &["and"]))
            {
                probe += 1;
                while filter_tokens
                    .get(probe)
                    .is_some_and(OwnedLexToken::is_comma)
                {
                    probe += 1;
                }
            }
            let mut phrase_probe = probe;
            if filter_tokens
                .get(phrase_probe)
                .is_some_and(|token| search_library_token_is_any_word(token, &["a", "an"]))
            {
                phrase_probe += 1;
            }
            if filter_tokens
                .get(phrase_probe)
                .is_some_and(|token| search_library_token_is_any_word(token, &["card", "cards"]))
                && filter_tokens
                    .get(phrase_probe + 1)
                    .is_some_and(|token| search_library_token_is_any_word(token, &["named"]))
            {
                break;
            }
            cursor += 1;
        }
    }
    if item_starts.len() <= 1 {
        return Ok(None);
    }

    let mut filters = Vec::new();
    for (pos, start) in item_starts.iter().enumerate() {
        let end = item_starts
            .get(pos + 1)
            .copied()
            .unwrap_or(filter_tokens.len());
        let item_tokens = trim_commas(&filter_tokens[*start..end]);
        let item_filter = parse_search_library_object_filter_lexed(&item_tokens, clause_display)?;
        if item_filter.name.is_none() {
            return Ok(None);
        }
        filters.push(item_filter);
    }
    Ok(Some(filters))
}

pub fn parse_search_library_leading_effect_prelude_lexed<'a>(
    subject_tokens: &'a [OwnedLexToken],
    subject_starts_effect_lexed: fn(&[OwnedLexToken]) -> bool,
    parse_leading_effects_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<SearchLibraryLeadingPrelude<'a>, CardTextError> {
    let typed_effect_head = super::super::chain_splitting::find_chain_verb_tokens(subject_tokens)
        .is_some()
        || super::super::chain_splitting::has_extended_effect_head_tokens(subject_tokens);
    if subject_tokens.is_empty()
        || (!subject_starts_effect_lexed(subject_tokens) && !typed_effect_head)
    {
        return Ok(SearchLibraryLeadingPrelude {
            subject_tokens,
            leading_effects: Vec::new(),
        });
    }

    let mut leading_tokens = trim_commas(subject_tokens);
    while leading_tokens
        .last()
        .is_some_and(|token| search_library_token_is_any_word(token, &["and", "then"]))
    {
        leading_tokens.pop();
    }
    let leading_effects = if leading_tokens.is_empty() {
        Vec::new()
    } else {
        parse_leading_effects_lexed(&leading_tokens)?
    };

    Ok(SearchLibraryLeadingPrelude {
        subject_tokens: &[],
        leading_effects,
    })
}

pub fn search_library_has_unsupported_top_position_probe(words: &[&str]) -> bool {
    word_slice_mentions_nth_from_top(words)
        && !search_word_stream_matches_at_some_offset(words, ON_TOP_OF_LIBRARY_PHRASE)
        && search_library_put_position_from_top_words(words).is_none()
}

pub fn search_library_has_unsupported_top_position_probe_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    search_library_has_unsupported_top_position_probe(&words)
}

pub fn search_library_put_position_from_top_words(words: &[&str]) -> Option<Value> {
    let mut idx = 0usize;
    while idx < words.len() {
        let Some((position, used)) = ironsmith_core::parse_ordinal_words(&words[idx..]) else {
            idx += 1;
            continue;
        };
        if idx + used + 2 < words.len()
            && words[idx + used..].starts_with(FROM_THE_TOP_PREFIX)
            && words[..idx]
                .iter()
                .any(|word| matches!(*word, "put" | "puts"))
        {
            return Some(Value::Fixed(position as i32));
        }
        idx += 1;
    }
    None
}

pub fn search_library_subject_wraps_each_target_player_lexed(
    subject_tokens: &[OwnedLexToken],
) -> bool {
    token_word_refs(subject_tokens).as_slice() == EACH_OF_THEM_SUBJECT
}

pub fn search_library_subject_player_iteration_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let words = token_word_refs(subject_tokens)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    match words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["each", "player"] | ["each", "players"] => Some(PlayerFilter::Any),
        ["each", "opponent"] | ["each", "opponents"] => Some(PlayerFilter::Opponent),
        ["each", "other", "player"] | ["each", "other", "players"] => Some(PlayerFilter::NotYou),
        _ => None,
    }
}

pub fn parse_search_library_iterated_object_subject_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    const PLAYER_OR_OPPONENT_PREFIXES: &[&[&str]] = &[
        &["player"],
        &["players"],
        &["opponent"],
        &["opponents"],
        &["target", "player"],
        &["target", "players"],
        &["target", "opponent"],
        &["target", "opponents"],
    ];

    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if token_word_refs(subject_tokens).as_slice() == EACH_OF_THEM_SUBJECT {
        return Ok(None);
    }

    let mut filter_tokens = if let Some((_, rest)) =
        primitives::strip_lexed_prefix_phrases(subject_tokens, &[&["for", "each"]])
    {
        rest
    } else if let Some((_, rest)) =
        primitives::strip_lexed_prefix_phrases(subject_tokens, &[&["each"]])
    {
        rest
    } else {
        return Ok(None);
    };

    if filter_tokens
        .first()
        .is_some_and(|token| search_library_token_is_any_word(token, &["of"]))
    {
        filter_tokens = &filter_tokens[1..];
    }

    let filter_tokens = trim_commas(filter_tokens);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    if primitives::strip_lexed_prefix_phrases(&filter_tokens, PLAYER_OR_OPPONENT_PREFIXES).is_some()
    {
        return Ok(None);
    }

    Ok(Some(parse_object_filter_lexed(&filter_tokens, false)?))
}

pub fn search_library_starts_with_search_verb_lexed(search_tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(search_tokens, search_library_search_verb).is_some()
}
