use super::*;

pub(crate) fn parse_sacrifice(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    target: Option<TargetAst>,
) -> Result<EffectAst, CardTextError> {
    let mut tokens = tokens;
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let mut normalized_words = clause_words.as_slice();
    if let Some(unless_idx) = find_index(&normalized_words, |word| *word == "unless") {
        let tail = &normalized_words[unless_idx..];
        if tail == ["unless", "it", "escaped"] {
            let cut_idx = token_index_for_word_index(tokens, unless_idx).unwrap_or(tokens.len());
            tokens = &tokens[..cut_idx];
            normalized_words = &normalized_words[..unless_idx];
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported sacrifice-unless clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }
    let has_greatest_mana_value = grammar::contains_word(tokens, "greatest")
        && grammar::contains_word(tokens, "mana")
        && grammar::contains_word(tokens, "value");
    if has_greatest_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported greatest-mana-value sacrifice clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }
    let has_for_each_graveyard_history = grammar::contains_word(tokens, "for")
        && grammar::contains_word(tokens, "each")
        && grammar::contains_word(tokens, "graveyard")
        && grammar::contains_word(tokens, "turn");
    if has_for_each_graveyard_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported graveyard-history sacrifice clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if tokens
        .first()
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        let mut idx = 1usize;
        let mut other = false;
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_word("other") || token.is_word("another"))
        {
            other = true;
            idx += 1;
        }
        let mut filter = parse_object_filter_lexed(&tokens[idx..], other)?;
        if other {
            filter.other = true;
        }
        return Ok(EffectAst::SacrificeAll { filter, player });
    }

    let mut idx = 0;
    let mut count = 1u32;
    let mut other = false;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        other = true;
        idx += 1;
    }
    if count == 1
        && let Some((value, used)) = parse_number(&tokens[idx..])
    {
        count = value;
        idx += used;
    }

    // Split off a trailing "for each ..." suffix before parsing the filter.
    let remaining_tokens = &tokens[idx..];
    let for_each_idx = grammar::find_prefix(remaining_tokens, || grammar::phrase(&["for", "each"]))
        .map(|(idx, _, _)| idx);

    let (object_tokens, for_each_filter) = if let Some(fe_idx) = for_each_idx {
        let fe_count_tokens = &remaining_tokens[fe_idx..];
        let fe_value = parse_get_for_each_count_value(fe_count_tokens)?;
        (&remaining_tokens[..fe_idx], fe_value)
    } else {
        (remaining_tokens, None)
    };

    let filter_words = ZoneHandlerNormalizedWords::new(object_tokens);
    let suffix_word_count =
        if grammar::words_match_suffix(object_tokens, &["of", "their", "choice"]).is_some()
            || grammar::words_match_suffix(object_tokens, &["of", "your", "choice"]).is_some()
            || grammar::words_match_suffix(object_tokens, &["of", "its", "choice"]).is_some()
        {
            3usize
        } else if grammar::words_match_suffix(object_tokens, &["of", "his", "or", "her", "choice"])
            .is_some()
        {
            5usize
        } else {
            0usize
        };
    let filter_tokens = if suffix_word_count == 0 {
        object_tokens
    } else {
        let keep_words = filter_words
            .to_word_refs()
            .len()
            .saturating_sub(suffix_word_count);
        let cut_idx = filter_words
            .token_index_for_word_index(keep_words)
            .unwrap_or(object_tokens.len());
        &object_tokens[..cut_idx]
    };
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after chooser suffix (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let mut filter = parse_object_filter_lexed(filter_tokens, other)?;
    if other {
        filter.other = true;
    }
    if filter.source && count != 1 {
        return Err(CardTextError::ParseError(format!(
            "source sacrifice only supports count 1 (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let sacrifice_words = crate::cards::builders::parser::token_word_refs(tokens);
    let excludes_attached_object = find_window_by(&sacrifice_words, 3, |window| {
        matches!(
            window,
            ["than", "enchanted", "creature"]
                | ["than", "enchanted", "permanent"]
                | ["than", "equipped", "creature"]
                | ["than", "equipped", "permanent"]
        )
    })
    .is_some();
    if excludes_attached_object
        && filter.controller.is_none()
        && let Some(controller) = controller_filter_for_token_player(player)
    {
        filter.controller = Some(controller);
    }

    let sacrifice = EffectAst::Sacrifice {
        filter,
        player,
        count,
        target,
    };

    // Wrap in ForEachObject when the clause has a "for each <filter>" suffix,
    // e.g. "sacrifices a land for each card in your hand".
    if let Some(Value::Count(fe_filter)) = for_each_filter {
        Ok(EffectAst::ForEachObject {
            filter: fe_filter,
            effects: vec![sacrifice],
        })
    } else {
        Ok(sacrifice)
    }
}

pub(crate) fn parse_discard(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::contains_word(tokens, "hand") {
        return Ok(EffectAst::DiscardHand { player });
    }

    if matches!(clause_words.as_slice(), ["it"] | ["that", "card"]) {
        let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        tagged_filter.zone = Some(Zone::Hand);
        return Ok(EffectAst::Discard {
            count: Value::Fixed(1),
            player,
            random: false,
            filter: Some(tagged_filter),
            tag: None,
        });
    }

    let count_tokens =
        if let Some((_, rest)) = grammar::words_match_any_prefix(tokens, UP_TO_PREFIXES) {
            rest
        } else {
            tokens
        };
    let count_offset = tokens.len().saturating_sub(count_tokens.len());
    let uses_all_count = count_tokens
        .first()
        .is_some_and(|token| token.is_word("all"));
    let (mut count, used) = if uses_all_count {
        (Value::Fixed(0), count_offset + 1)
    } else {
        let (count, used_relative) = parse_value(count_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        (count, count_offset + used_relative)
    };

    let rest = &tokens[used..];
    let rest_words = crate::cards::builders::parser::token_word_refs(rest);
    let Some(card_word_idx) = find_index(&rest_words, |word| *word == "card" || *word == "cards")
    else {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    };

    let card_token_idx = token_index_for_word_index(rest, card_word_idx).unwrap_or(rest.len());
    let qualifier_tokens = trim_commas(&rest[..card_token_idx]);
    let mut discard_filter = None;
    if !qualifier_tokens.is_empty() {
        let mut filter = if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
            filter
        } else if let Some(filter) = parse_discard_chosen_color_qualifier_filter(&qualifier_tokens)
        {
            filter
        } else if let Some(filter) = parse_discard_color_qualifier_filter(&qualifier_tokens) {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported discard card qualifier (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        filter.zone = Some(Zone::Hand);
        if uses_all_count
            && let Some(owner) = discard_subject_owner_filter(subject)
            && filter.owner.is_none()
        {
            filter.owner = Some(owner);
        }
        discard_filter = Some(filter);
    }

    let trailing_tokens = if card_word_idx + 1 < rest_words.len() {
        let trailing_token_idx =
            token_index_for_word_index(rest, card_word_idx + 1).unwrap_or(rest.len());
        &rest[trailing_token_idx..]
    } else {
        &[]
    };
    let trailing_words = crate::cards::builders::parser::token_word_refs(trailing_tokens);
    let random = trailing_words.as_slice() == ["at", "random"];
    if !trailing_words.is_empty() && !random {
        let trailing_filter = if let Ok(filter) = parse_object_filter(trailing_tokens, false) {
            Some(filter)
        } else if let Some(filter) = parse_discard_chosen_color_qualifier_filter(trailing_tokens) {
            Some(filter)
        } else if let Some(filter) = parse_discard_color_qualifier_filter(trailing_tokens) {
            Some(filter)
        } else {
            None
        };

        if let Some(mut filter) = trailing_filter {
            filter.zone = Some(Zone::Hand);
            if uses_all_count
                && let Some(owner) = discard_subject_owner_filter(subject)
                && filter.owner.is_none()
            {
                filter.owner = Some(owner);
            }
            discard_filter = Some(filter);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discard clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if uses_all_count {
        count = if let Some(filter) = discard_filter.as_ref() {
            Value::Count(filter.clone())
        } else if let Some(owner) = discard_subject_owner_filter(subject) {
            Value::CardsInHand(owner)
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            )));
        };
    }

    Ok(EffectAst::Discard {
        count,
        player,
        random,
        filter: discard_filter,
        tag: None,
    })
}

pub(crate) fn parse_discard_color_qualifier_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let qualifier_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if qualifier_words.is_empty() {
        return None;
    }

    let mut colors = crate::color::ColorSet::new();
    let mut saw_color = false;
    for word in qualifier_words {
        if word == "or" {
            continue;
        }
        let color = parse_color(word)?;
        colors = colors.union(color);
        saw_color = true;
    }

    if !saw_color {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.colors = Some(colors);
    Some(filter)
}

pub(crate) fn parse_discard_chosen_color_qualifier_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let qualifier_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !matches!(
        qualifier_words.as_slice(),
        ["of", "that", "color"]
            | ["that", "color"]
            | ["of", "the", "chosen", "color"]
            | ["the", "chosen", "color"]
    ) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.chosen_color = true;
    Some(filter)
}

pub(crate) fn discard_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::target_opponent())
        }
        Some(SubjectAst::Player(PlayerAst::That)) => Some(PlayerFilter::IteratedPlayer),
        Some(SubjectAst::Player(PlayerAst::You)) => Some(PlayerFilter::You),
        _ => None,
    }
}
