use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};

const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const ADD_MANA_IMPRINTED_COLORS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["exiled", "colors"]);
const ADD_MANA_COMMANDER_IDENTITY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words &[&["commander", "commanders"]];
    contains_words &["color", "identity"]
);
const MANA_OF_CHOSEN_COLOR_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["mana", "of", "the"], &["mana", "of"]]);
const ADD_MANA_THAT_COLOR_AMOUNT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["an", "amount", "of", "mana", "of", "that", "color"]);
const ANY_ONE_COLOR_OR_TYPE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["any", "one", "color"], &["any", "one", "type"]]]);
const ANY_COLOR_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["any", "color"], &["one", "color"]]]);
const ANY_TYPE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["any", "type"], &["one", "type"]]]);
const COLOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["color"]);
const TYPE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["type"]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const ADD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["add"]);
const WHERE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["where"]);
const FOR_EACH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["for", "each"]);
const FOR_EACH_REMOVED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["for", "each"]; suffix & ["removed", "this", "way"]);
const CHOSEN_COLOR_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["chosen", "color"]);
const CHOSEN_COLOR_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["or", "one", "mana", "of", "the", "chosen", "color"]);
const FOR_EACH_COLOR_AMONG_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["for", "each", "color", "among"]);
const ADD_ONE_MANA_OF_THAT_COLOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["add", "one", "mana", "of", "that", "color"]);
const ANY_COMBINATION_OF_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["any", "combination", "of"]);
const TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const STRIKE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["strike"]);
const ANOTHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["another"]);
const STRIKE_COUNTER_PREFIXES: &[(&str, CounterType)] = &[
    ("double", CounterType::DoubleStrike),
    ("first", CounterType::FirstStrike),
];
const CHOSEN_BY_PLAYER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["they", "choose"],
            &["that", "player", "chooses"],
            &["they", "choose", "to", "their", "mana", "pool"],
            &["that", "player", "chooses", "to", "their", "mana", "pool"],
        ]
);
const MANA_POOL_TAIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["to"],
            &["your"],
            &["their"],
            &["its"],
            &["that"],
            &["player"],
            &["players"],
            &["mana"],
            &["pool"],
        ]
);
const MANA_OPTION_SEPARATOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["and"],
            &["or"],
            &["and/or"],
            &["mana"],
            &["to"],
            &["your"],
            &["their"],
            &["its"],
            &["pool"],
        ]
);
const COLOR_OR_COLORS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["color"], &["colors"]]);

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
        if !INSTEAD_WORD_PATTERN.matches_word_at(&tail_words, 0)
            || !IF_WORD_PATTERN.matches_word_at(&tail_words, 1)
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
        .any(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word));
    if let Some(colors_among) = parse_add_mana_colors_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_colors_among(
            player,
            colors_among,
        ));
    }
    if has_card_word && ADD_MANA_IMPRINTED_COLORS_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_add_mana_imprinted_colors());
    }

    if ADD_MANA_COMMANDER_IDENTITY_PATTERN.matches_words(&clause_words) {
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
        && let Some(chosen_idx) = CHOSEN_COLOR_PATTERN.find_exact_window(&clause_words, 2)
    {
        let prefix = &clause_words[..chosen_idx];
        let references_mana_of_chosen_color =
            MANA_OF_CHOSEN_COLOR_PREFIX_PATTERN.matches_words(prefix);
        if references_mana_of_chosen_color {
            let tail_words = &clause_words[chosen_idx + 2..];
            let has_only_pool_tail = tail_words.is_empty()
                || tail_words
                    .iter()
                    .all(|word| MANA_POOL_TAIL_WORD_PATTERN.matches_word(word));
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
    if ADD_MANA_THAT_COLOR_AMOUNT_PATTERN.matches_words(&clause_words) {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }

    let any_one = ANY_ONE_COLOR_OR_TYPE_PATTERN.matches_words(&clause_words);
    let any_color = ANY_COLOR_PATTERN.matches_words(&clause_words);
    let any_type = ANY_TYPE_PATTERN.matches_words(&clause_words);
    if any_color || any_type {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let allow_colorless = any_type;
        let phrase_end = crate::slice_primitives::find_index(tokens, |token| {
            token.as_word().is_some_and(|word| {
                (COLOR_WORD_PATTERN.matches_word(word) && any_color)
                    || (TYPE_WORD_PATTERN.matches_word(word) && any_type)
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

        let tail_words = crate::runtime_backend::token_word_refs(tail_tokens);
        let chosen_by_player_tail = CHOSEN_BY_PLAYER_TAIL_PATTERN.matches_words(&tail_words);
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
        if FOR_EACH_REMOVED_THIS_WAY_PATTERN.matches_words(&tail_words)
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

        if tail_words
            .first()
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        {
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
        FOR_EACH_WORD_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(window))
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
            if MANA_POOL_TAIL_WORD_PATTERN.matches_word(word) {
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
            let chosen_color_tail = CHOSEN_COLOR_TAIL_PATTERN.matches_words(&trailing_words);
            let pool_tail = if chosen_color_tail {
                trailing_words[7..].to_vec()
            } else {
                Vec::new()
            };
            let has_only_pool_tail = chosen_color_tail
                && (pool_tail.is_empty()
                    || pool_tail
                        .iter()
                        .all(|word| MANA_POOL_TAIL_WORD_PATTERN.matches_word(word)));
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
                .all(|word| MANA_POOL_TAIL_WORD_PATTERN.matches_word(word));
        let has_only_instead_tail = INSTEAD_WORD_PATTERN.matches_words(&trailing_words);
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
    if FOR_EACH_COLOR_AMONG_PATTERN
        .find_exact_window(&words, 4)
        .is_none()
        || ADD_ONE_MANA_OF_THAT_COLOR_PATTERN
            .find_exact_window(&words, 6)
            .is_none()
    {
        return Ok(None);
    }

    let Some(among_idx) = find_index(tokens, |token| AMONG_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    let Some(add_idx) = find_index(tokens, |token| ADD_WORD_PATTERN.matches_token(token)) else {
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
    let Some(combination_idx) = ANY_COMBINATION_OF_PATTERN.find_exact_window(&clause_words, 3)
    else {
        return Ok(None);
    };

    let color_words = clause_words[combination_idx + 3..]
        .iter()
        .take_while(|w| !WHERE_WORD_PATTERN.matches_word(w));

    let mut colors = Vec::new();
    for word in color_words {
        if MANA_OPTION_SEPARATOR_WORD_PATTERN.matches_word(word) {
            continue;
        }
        if COLOR_OR_COLORS_WORD_PATTERN.matches_word(word) {
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
        || !TO_WORD_PATTERN.matches_word(words[0])
        || !grammar::contains_word(tokens, "mana")
        || !grammar::contains_word(tokens, "pool")
    {
        return false;
    }
    words
        .iter()
        .all(|word| MANA_POOL_TAIL_WORD_PATTERN.matches_word(word))
}

pub(crate) fn parse_counter_type_from_descriptor_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterType> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let last = *words.last()?;
    if let Some(counter_type) = parse_counter_type_word(last) {
        return Some(counter_type);
    }
    if STRIKE_WORD_PATTERN.matches_word(last) && words.len() >= 2 {
        return strike_counter_type_from_prefix(words[words.len() - 2]);
    }
    if ANOTHER_WORD_PATTERN.matches_word(last)
        || ironsmith_core::parse_cardinal_word(last).is_some()
    {
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
