use super::*;
use crate::runtime_backend::lexer::{
    token_slice_at_is, token_slice_first_is, token_slice_starts_with,
    word_slice_contains_any_phrase, word_slice_find_phrase_start_or_zero, word_slice_first_is,
};

pub(crate) fn parse_add_mana(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    parser_trace_stack("parse_add_mana:entry", tokens);
    let clause_word_storage = ZoneHandlerNormalizedWords::new(tokens);
    let clause_words = clause_word_storage.to_word_refs();
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[OwnedLexToken]|
     -> Result<Option<EffectAst>, CardTextError> {
        if !token_slice_first_is(tail_tokens, "instead") || !token_slice_at_is(tail_tokens, 1, "if")
        {
            return Ok(None);
        }
        let predicate =
            parse_trailing_instead_if_predicate_lexed(tail_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported trailing mana clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        Ok(Some(EffectAst::Conditional {
            predicate,
            if_true: vec![base_effect],
            if_false: Vec::new(),
        }))
    };

    let has_card_word = clause_words
        .iter()
        .any(|word| *word == "card" || *word == "cards");
    if let Some(colors_among) = parse_add_mana_colors_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_colors_among(
            player,
            colors_among,
        ));
    }
    if grammar::contains_word(tokens, "exiled")
        && has_card_word
        && grammar::contains_word(tokens, "colors")
    {
        return Ok(EffectAst::subject_verb_add_mana_imprinted_colors());
    }

    if (grammar::contains_word(tokens, "commander") || grammar::contains_word(tokens, "commanders"))
        && grammar::contains_word(tokens, "color")
        && grammar::contains_word(tokens, "identity")
    {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_commander_identity(
            player, amount,
        ));
    }

    if let Some(available_colors) = parse_any_combination_mana_colors(tokens)? {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            amount,
            Some(available_colors),
        ));
    }

    if let Some(available_colors) = parse_or_mana_color_choices(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            Value::Fixed(1),
            Some(available_colors),
        ));
    }

    // "Add one mana of the chosen color."
    let has_explicit_symbol = tokens
        .iter()
        .any(|token| mana_pips_from_token(token).is_some());
    if !has_explicit_symbol
        && let Some(chosen_idx) =
            word_slice_find_phrase_start_or_zero(&clause_words, &["chosen", "color"])
    {
        let prefix = &clause_words[..chosen_idx];
        let references_mana_of_chosen_color =
            crate::runtime_backend::lexer::word_slice_ends_with(prefix, &["mana", "of", "the"])
                || crate::runtime_backend::lexer::word_slice_ends_with(prefix, &["mana", "of"]);
        if references_mana_of_chosen_color {
            let tail_words = &clause_words[chosen_idx + 2..];
            let has_only_pool_tail = tail_words.is_empty()
                || tail_words.iter().all(|word| {
                    matches!(
                        *word,
                        "to" | "your"
                            | "their"
                            | "its"
                            | "that"
                            | "player"
                            | "players"
                            | "mana"
                            | "pool"
                    )
                });
            if has_only_pool_tail {
                let amount = parse_value(tokens)
                    .map(|(value, _)| value)
                    .unwrap_or(Value::Fixed(1));
                return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                    player, amount, None,
                ));
            }
        }
    }
    if grammar::words_match_prefix(
        tokens,
        &["an", "amount", "of", "mana", "of", "that", "color"],
    )
    .is_some()
    {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }

    let any_one = word_slice_contains_any_phrase(
        &clause_words,
        &[&["any", "one", "color"], &["any", "one", "type"]],
    );
    let any_color =
        word_slice_contains_any_phrase(&clause_words, &[&["any", "color"], &["one", "color"]]);
    let any_type =
        word_slice_contains_any_phrase(&clause_words, &[&["any", "type"], &["one", "type"]]);
    if any_color || any_type {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let allow_colorless = any_type;
        let phrase_end = crate::slice_primitives::find_index(tokens, |token| {
            token
                .as_word()
                .is_some_and(|word| (word == "color" && any_color) || (word == "type" && any_type))
        })
        .map(|idx| idx + 1)
        .unwrap_or(tokens.len());
        let tail_tokens = trim_leading_commas(&tokens[phrase_end..]);

        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if let Some(filter) = parse_land_could_produce_filter(tail_tokens)? {
            parser_trace_stack("parse_add_mana:land-could-produce", tokens);
            return Ok(EffectAst::subject_verb_add_mana_from_land_could_produce(
                player,
                amount,
                filter,
                allow_colorless,
                any_one,
            ));
        }

        if matches!(amount, Value::X)
            && let Some(dynamic_amount) = parse_where_x_is_number_of_filter_value(tail_tokens)
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        let tail_words = crate::runtime_backend::token_word_refs(tail_tokens);
        let chosen_by_player_tail = matches!(
            tail_words.as_slice(),
            ["they", "choose"]
                | ["that", "player", "chooses"]
                | ["they", "choose", "to", "their", "mana", "pool"]
                | ["that", "player", "chooses", "to", "their", "mana", "pool"]
        );
        if chosen_by_player_tail {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }
        if grammar::words_match_any_prefix(tail_tokens, FOR_EACH_PREFIXES).is_some()
            && grammar::words_match_suffix(tail_tokens, &["removed", "this", "way"]).is_some()
            && let Some(dynamic_amount) = parse_dynamic_cost_modifier_value(tail_tokens)?
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if word_slice_first_is(&tail_words, "among") {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        let base_effect = if any_one {
            EffectAst::subject_verb_add_mana_any_one_color(player, amount)
        } else {
            EffectAst::subject_verb_add_mana_any_color(player, amount, None)
        };
        if let Some(conditional) = wrap_instead_if_tail(base_effect, tail_tokens)? {
            return Ok(conditional);
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let for_each_idx = find_window_by(tokens, 2, |window: &[OwnedLexToken]| {
        token_slice_starts_with(window, &["for", "each"])
    });
    let mana_scan_end = for_each_idx.unwrap_or(tokens.len());

    let mut mana = Vec::new();
    let mut last_mana_idx = None;
    for (idx, token) in tokens[..mana_scan_end].iter().enumerate() {
        if let Some(group) = mana_pips_from_token(token) {
            mana.extend(group);
            last_mana_idx = Some(idx);
            continue;
        }
        if let Some(word) = token.as_word() {
            if word.eq_ignore_ascii_case("mana")
                || word.eq_ignore_ascii_case("to")
                || word.eq_ignore_ascii_case("your")
                || word.eq_ignore_ascii_case("pool")
            {
                continue;
            }
        }
    }

    if !mana.is_empty() {
        if let Some(amount) = parse_add_mana_that_much_value(tokens) {
            parser_trace_stack("parse_add_mana:scaled-that-much", tokens);
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_devotion_value_from_add_clause(tokens)? {
            parser_trace_stack("parse_add_mana:scaled-devotion", tokens);
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(for_each_idx) = for_each_idx {
            let amount_tokens = &tokens[for_each_idx..];
            let amount = parse_dynamic_cost_modifier_value(amount_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic mana amount (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
            parser_trace_stack("parse_add_mana:scaled", tokens);
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_equal_to_aggregate_filter_value(tokens)
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
        {
            parser_trace_stack("parse_add_mana:scaled-equal", tokens);
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        let trailing_words = if let Some(last_idx) = last_mana_idx {
            crate::runtime_backend::token_word_refs(&tokens[last_idx + 1..])
        } else {
            Vec::new()
        };
        if !trailing_words.is_empty() {
            let trailing_token_slice = last_mana_idx.map(|idx| &tokens[idx + 1..]).unwrap_or(&[]);
            let chosen_color_tail = grammar::words_match_prefix(
                trailing_token_slice,
                &["or", "one", "mana", "of", "the", "chosen", "color"],
            )
            .is_some();
            let pool_tail = if chosen_color_tail {
                trailing_words[7..].to_vec()
            } else {
                Vec::new()
            };
            let has_only_pool_tail = chosen_color_tail
                && (pool_tail.is_empty()
                    || pool_tail
                        .iter()
                        .all(|word| matches!(*word, "to" | "your" | "mana" | "pool")));
            if chosen_color_tail && has_only_pool_tail {
                if mana.len() != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with multiple symbols (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                let Some(color) = mana_symbol_to_color(mana[0]) else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with non-colored symbol (clause: '{}')",
                        clause_words.join(" ")
                    )));
                };
                parser_trace_stack("parse_add_mana:chosen-color-option", tokens);
                return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                    player,
                    Value::Fixed(1),
                    Some(color),
                ));
            }
        }
        let has_only_pool_tail = !trailing_words.is_empty()
            && trailing_words
                .iter()
                .all(|word| matches!(*word, "to" | "your" | "mana" | "pool"));
        let has_only_instead_tail =
            crate::runtime_backend::lexer::word_slice_eq(&trailing_words, &["instead"]);
        if !trailing_words.is_empty() && !has_only_pool_tail && !has_only_instead_tail {
            if let Some(last_idx) = last_mana_idx
                && let Some(conditional) = wrap_instead_if_tail(
                    EffectAst::subject_verb_add_mana(player, mana.clone()),
                    trim_leading_commas(&tokens[last_idx + 1..]),
                )?
            {
                return Ok(conditional);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parser_trace_stack("parse_add_mana:flat", tokens);
        return Ok(EffectAst::subject_verb_add_mana(player, mana));
    }

    Err(CardTextError::ParseError(format!(
        "missing mana symbols (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_add_mana_colors_among_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_find_phrase_start_or_zero(&words, &["for", "each", "color", "among"]).is_none()
        || word_slice_find_phrase_start_or_zero(
            &words,
            &["add", "one", "mana", "of", "that", "color"],
        )
        .is_none()
    {
        return Ok(None);
    }

    let Some(among_idx) = find_index(tokens, |token| token.is_word("among")) else {
        return Ok(None);
    };
    let Some(add_idx) = find_index(tokens, |token| token.is_word("add")) else {
        return Ok(None);
    };
    if add_idx <= among_idx + 1 {
        return Ok(None);
    }

    let filter_tokens = trim_edge_punctuation(&tokens[among_idx + 1..add_idx]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false)?;
    Ok(Some(filter))
}

pub(crate) fn mana_symbol_to_color(symbol: ManaSymbol) -> Option<crate::color::Color> {
    match symbol {
        ManaSymbol::White => Some(crate::color::Color::White),
        ManaSymbol::Blue => Some(crate::color::Color::Blue),
        ManaSymbol::Black => Some(crate::color::Color::Black),
        ManaSymbol::Red => Some(crate::color::Color::Red),
        ManaSymbol::Green => Some(crate::color::Color::Green),
        _ => None,
    }
}

pub(crate) fn parse_or_mana_color_choices(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    use winnow::combinator::{alt, opt};
    use winnow::prelude::*;

    if !grammar::contains_word(tokens, "or") {
        return Ok(None);
    }

    /// Parse one mana token and convert its symbols to colors, pushing unique
    /// colors into the accumulator.  Returns `None` (backtrack) if any symbol
    /// is not a valid color.
    fn mana_color_token<'a>(
        input: &mut LexStream<'a>,
    ) -> Result<Vec<crate::color::Color>, winnow::error::ErrMode<winnow::error::ContextError>> {
        let pips = grammar::mana_pips_token.parse_next(input)?;
        let mut colors = Vec::new();
        for symbol in pips {
            let Some(color) = mana_symbol_to_color(symbol) else {
                return Err(grammar::backtrack_err("color", "colored mana symbol"));
            };
            if !slice_contains(&colors, &color) {
                colors.push(color);
            }
        }
        Ok(colors)
    }

    let mut stream = LexStream::new(tokens);
    let mut colors = Vec::new();
    while !stream.is_empty() {
        // Skip noise words, "or", and commas
        if opt(alt((
            grammar::skip_mana_noise,
            grammar::kw("or").void(),
            grammar::comma().void(),
        )))
        .parse_next(&mut stream)
        .unwrap_or(None)
        .is_some()
        {
            continue;
        }
        // Try to parse a mana color token
        if let Some(new_colors) = opt(mana_color_token)
            .parse_next(&mut stream)
            .unwrap_or(None)
        {
            for c in new_colors {
                if !slice_contains(&colors, &c) {
                    colors.push(c);
                }
            }
        } else {
            // Unrecognized token — bail
            return Ok(None);
        }
    }

    if colors.len() < 2 {
        return Ok(None);
    }

    Ok(Some(colors))
}

pub(crate) fn parse_any_combination_mana_colors(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_word_storage = ZoneHandlerNormalizedWords::new(tokens);
    let clause_words = clause_word_storage.to_word_refs();
    let Some(combination_idx) =
        word_slice_find_phrase_start_or_zero(&clause_words, &["any", "combination", "of"])
    else {
        return Ok(None);
    };

    let color_words = clause_words[combination_idx + 3..]
        .iter()
        .take_while(|w| **w != "where");

    let mut colors = Vec::new();
    for word in color_words {
        if matches!(
            *word,
            "and" | "or" | "and/or" | "mana" | "to" | "your" | "their" | "its" | "pool"
        ) {
            continue;
        }
        if matches!(*word, "color" | "colors") {
            for color in crate::color::Color::ALL {
                if !slice_contains(&colors, &color) {
                    colors.push(color);
                }
            }
            continue;
        }
        let symbol = parse_mana_symbol(word).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported restricted mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        let color = mana_symbol_to_color(symbol).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported non-colored mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        if !slice_contains(&colors, &color) {
            colors.push(color);
        }
    }

    if colors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing color options in any-combination mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(colors))
}

pub(crate) fn is_mana_pool_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty()
        || words[0] != "to"
        || !grammar::contains_word(tokens, "mana")
        || !grammar::contains_word(tokens, "pool")
    {
        return false;
    }
    words.iter().all(|word| {
        matches!(
            *word,
            "to" | "your" | "their" | "its" | "that" | "player" | "players" | "mana" | "pool"
        )
    })
}

pub(crate) fn parse_counter_type_from_descriptor_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterType> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let last = *words.last()?;
    if let Some(counter_type) = parse_counter_type_word(last) {
        return Some(counter_type);
    }
    if last == "strike" && words.len() >= 2 {
        return match words[words.len() - 2] {
            "double" => Some(CounterType::DoubleStrike),
            "first" => Some(CounterType::FirstStrike),
            _ => None,
        };
    }
    if last == "another" || ironsmith_core::parse_cardinal_word(last).is_some() {
        return None;
    }
    if last.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(CounterType::Named(intern_counter_name(last)));
    }
    None
}
