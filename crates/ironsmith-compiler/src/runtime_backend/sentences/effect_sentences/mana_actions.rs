use super::*;
use crate::runtime_backend::lexer::{
    word_slice_contains_all_words, word_slice_contains_any_phrase, word_slice_contains_any_word,
    word_slice_contains_phrase, word_slice_ends_with_any, word_slice_eq, word_slice_eq_any,
    word_slice_find_phrase_start, word_slice_starts_with,
};

const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const ADD_MANA_IMPRINTED_COLOR_WORDS: &[&str] = &["exiled", "colors"];
const ADD_MANA_COMMANDER_WORDS: &[&str] = &["commander", "commanders"];
const ADD_MANA_COMMANDER_IDENTITY_WORDS: &[&str] = &["color", "identity"];
const MANA_OF_CHOSEN_COLOR_SUFFIXES: &[&[&str]] = &[&["mana", "of", "the"], &["mana", "of"]];
const ADD_MANA_THAT_COLOR_AMOUNT_PREFIX: &[&str] =
    &["an", "amount", "of", "mana", "of", "that", "color"];
const ADD_MANA_ONE_THAT_COLOR_PREFIX: &[&str] = &["one", "mana", "of", "that", "color"];
const ANY_ONE_COLOR_OR_TYPE_PHRASES: &[&[&str]] =
    &[&["any", "one", "color"], &["any", "one", "type"]];
const ANY_COLOR_PHRASES: &[&[&str]] = &[&["any", "color"], &["one", "color"]];
const DIFFERENT_COLORS_PHRASE: &[&str] = &["different", "colors"];
const ANY_TYPE_PHRASES: &[&[&str]] = &[&["any", "type"], &["one", "type"]];
const COLOR_WORD: &str = "color";
const TYPE_WORD: &str = "type";
const INSTEAD_WORD: &str = "instead";
const IF_WORD: &str = "if";
const AMONG_WORD: &str = "among";
const ADD_WORD: &str = "add";
const WHERE_WORD: &str = "where";
const FOR_EACH_PHRASE: &[&str] = &["for", "each"];
const FOR_EACH_REMOVED_THIS_WAY_PREFIX: &[&str] = &["for", "each"];
const FOR_EACH_REMOVED_THIS_WAY_SUFFIX: &[&str] = &["removed", "this", "way"];
const CHOSEN_COLOR_PHRASE: &[&str] = &["chosen", "color"];
const CHOSEN_COLOR_TAIL_PREFIX: &[&str] = &["or", "one", "mana", "of", "the", "chosen", "color"];
const FOR_EACH_COLOR_AMONG_PHRASE: &[&str] = &["for", "each", "color", "among"];
const ADD_ONE_MANA_OF_THAT_COLOR_PHRASE: &[&str] = &["add", "one", "mana", "of", "that", "color"];
const ANY_COMBINATION_OF_PHRASE: &[&str] = &["any", "combination", "of"];
const TO_WORD: &str = "to";
const STRIKE_WORD: &str = "strike";
const ANOTHER_WORD: &str = "another";
const STRIKE_COUNTER_PREFIXES: &[(&str, CounterType)] = &[
    ("double", CounterType::DoubleStrike),
    ("first", CounterType::FirstStrike),
];
const CHOSEN_BY_PLAYER_TAILS: &[&[&str]] = &[
    &["they", "choose"],
    &["that", "player", "chooses"],
    &["they", "choose", "to", "their", "mana", "pool"],
    &["that", "player", "chooses", "to", "their", "mana", "pool"],
];
const MANA_POOL_TAIL_WORDS: &[&str] = &[
    "to", "your", "their", "its", "that", "player", "players", "mana", "pool",
];
const MANA_OPTION_SEPARATOR_WORDS: &[&str] = &[
    "and", "or", "and/or", "mana", "to", "your", "their", "its", "pool",
];
const COLOR_OR_COLORS_WORDS: &[&str] = &["color", "colors"];
const PUBLIC_REVEALED_TAG: &str = "__public_revealed";

fn bind_revealed_this_way_count_to_last_object(value: Value) -> Value {
    match value {
        Value::Count(mut filter) => {
            for constraint in &mut filter.tagged_constraints {
                if constraint.tag.as_str() == PUBLIC_REVEALED_TAG {
                    constraint.tag = TagKey::from(IT_TAG);
                }
            }
            Value::Count(filter)
        }
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_revealed_this_way_count_to_last_object(*value)),
            hints,
        },
        other => other,
    }
}

fn mana_token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some() && token.parser_text() == expected
}

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
        let tail_words = crate::runtime_backend::token_word_refs(tail_tokens);
        if tail_words.get(0) != Some(&INSTEAD_WORD) || tail_words.get(1) != Some(&IF_WORD) {
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
        .any(|word| CARD_OR_CARDS_WORDS.contains(word));
    if let Some(colors_among) = parse_add_mana_colors_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_colors_among(
            player,
            colors_among,
        ));
    }
    if has_card_word && word_slice_contains_all_words(&clause_words, ADD_MANA_IMPRINTED_COLOR_WORDS)
    {
        return Ok(EffectAst::subject_verb_add_mana_imprinted_colors());
    }

    if word_slice_contains_any_word(&clause_words, ADD_MANA_COMMANDER_WORDS)
        && word_slice_contains_all_words(&clause_words, ADD_MANA_COMMANDER_IDENTITY_WORDS)
    {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_commander_identity(
            player, amount,
        ));
    }

    if word_slice_contains_phrase(&clause_words, DIFFERENT_COLORS_PHRASE) {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_any_color_with_distinct(
            player, amount, None, true,
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
        && let Some(chosen_idx) = word_slice_find_phrase_start(&clause_words, CHOSEN_COLOR_PHRASE)
    {
        let prefix_clause = clause_word_storage
            .token_index_after_words(chosen_idx)
            .map(|idx| crate::runtime_backend::token_word_refs(&tokens[..idx]))
            .unwrap_or_default();
        let references_mana_of_chosen_color =
            word_slice_ends_with_any(&prefix_clause, MANA_OF_CHOSEN_COLOR_SUFFIXES);
        if references_mana_of_chosen_color {
            let tail_words = &clause_words[chosen_idx + 2..];
            let has_only_pool_tail = tail_words.is_empty()
                || tail_words
                    .iter()
                    .all(|word| MANA_POOL_TAIL_WORDS.contains(word));
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
    if word_slice_starts_with(&clause_words, ADD_MANA_ONE_THAT_COLOR_PREFIX) {
        let tail_tokens =
            grammar::words_match_prefix(tokens, &["one", "mana", "of", "that", "color"])
                .unwrap_or(&[]);
        let tail_tokens = trim_leading_commas(tail_tokens);
        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player,
                Value::Fixed(1),
                None,
            ));
        }
        if let Some(amount) = parse_dynamic_cost_modifier_value(tail_tokens)? {
            let amount = bind_revealed_this_way_count_to_last_object(amount);
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player, amount, None,
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic chosen-color mana amount (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if word_slice_starts_with(&clause_words, ADD_MANA_THAT_COLOR_AMOUNT_PREFIX) {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }

    let any_one = word_slice_contains_any_phrase(&clause_words, ANY_ONE_COLOR_OR_TYPE_PHRASES);
    let any_color = word_slice_contains_any_phrase(&clause_words, ANY_COLOR_PHRASES);
    let any_type = word_slice_contains_any_phrase(&clause_words, ANY_TYPE_PHRASES);
    if any_color || any_type {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let allow_colorless = any_type;
        let phrase_end = crate::slice_primitives::find_index(tokens, |token| {
            token.as_word().is_some_and(|word| {
                (word == COLOR_WORD && any_color) || (word == TYPE_WORD && any_type)
            })
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

        let chosen_by_player_tail = word_slice_eq_any(
            &crate::runtime_backend::token_word_refs(tail_tokens),
            CHOSEN_BY_PLAYER_TAILS,
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
        let tail_words = crate::runtime_backend::token_word_refs(tail_tokens);
        if word_slice_starts_with(&tail_words, FOR_EACH_REMOVED_THIS_WAY_PREFIX)
            && word_slice_ends_with_any(&tail_words, &[FOR_EACH_REMOVED_THIS_WAY_SUFFIX])
            && let Some(dynamic_amount) = parse_dynamic_cost_modifier_value(tail_tokens)?
        {
            amount = bind_revealed_this_way_count_to_last_object(dynamic_amount);
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

        if tail_words.first().is_some_and(|word| *word == AMONG_WORD) {
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
        word_slice_eq(
            &crate::runtime_backend::token_word_refs(window),
            FOR_EACH_PHRASE,
        )
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
            if MANA_POOL_TAIL_WORDS.contains(&word) {
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
            let amount = bind_revealed_this_way_count_to_last_object(amount);
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
        let trailing_tokens = if let Some(last_idx) = last_mana_idx {
            &tokens[last_idx + 1..]
        } else {
            &[]
        };
        let trailing_words = crate::runtime_backend::token_word_refs(trailing_tokens);
        if !trailing_words.is_empty() {
            let chosen_color_tail =
                word_slice_starts_with(&trailing_words, CHOSEN_COLOR_TAIL_PREFIX);
            let pool_tail = if chosen_color_tail {
                trailing_words[7..].to_vec()
            } else {
                Vec::new()
            };
            let has_only_pool_tail = chosen_color_tail
                && (pool_tail.is_empty()
                    || pool_tail
                        .iter()
                        .all(|word| MANA_POOL_TAIL_WORDS.contains(word)));
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
                .all(|word| MANA_POOL_TAIL_WORDS.contains(word));
        let has_only_instead_tail = word_slice_eq(&trailing_words, &[INSTEAD_WORD]);
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
    if word_slice_find_phrase_start(&words, FOR_EACH_COLOR_AMONG_PHRASE).is_none()
        || word_slice_find_phrase_start(&words, ADD_ONE_MANA_OF_THAT_COLOR_PHRASE).is_none()
    {
        return Ok(None);
    }

    let Some(among_idx) = find_index(tokens, |token| mana_token_is_word(token, AMONG_WORD)) else {
        return Ok(None);
    };
    let Some(add_idx) = find_index(tokens, |token| mana_token_is_word(token, ADD_WORD)) else {
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
        word_slice_find_phrase_start(&clause_words, ANY_COMBINATION_OF_PHRASE)
    else {
        return Ok(None);
    };

    let color_words = clause_words[combination_idx + 3..]
        .iter()
        .take_while(|w| **w != WHERE_WORD);

    let mut colors = Vec::new();
    for word in color_words {
        if MANA_OPTION_SEPARATOR_WORDS.contains(word) {
            continue;
        }
        if COLOR_OR_COLORS_WORDS.contains(word) {
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
        || words[0] != TO_WORD
        || !grammar::contains_word(tokens, "mana")
        || !grammar::contains_word(tokens, "pool")
    {
        return false;
    }
    words.iter().all(|word| MANA_POOL_TAIL_WORDS.contains(word))
}

pub(crate) fn parse_counter_type_from_descriptor_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterType> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let last = *words.last()?;
    if let Some(counter_type) = parse_counter_type_word(last) {
        return Some(counter_type);
    }
    if last == STRIKE_WORD && words.len() >= 2 {
        return strike_counter_type_from_prefix(words[words.len() - 2]);
    }
    if last == ANOTHER_WORD || ironsmith_core::parse_cardinal_word(last).is_some() {
        return None;
    }
    if last.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(CounterType::Named(intern_counter_name(last)));
    }
    None
}

fn strike_counter_type_from_prefix(word: &str) -> Option<CounterType> {
    STRIKE_COUNTER_PREFIXES
        .iter()
        .find_map(|(prefix, counter_type)| (*prefix == word).then(|| counter_type.clone()))
}
