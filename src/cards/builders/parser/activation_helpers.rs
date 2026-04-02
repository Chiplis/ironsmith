use crate::cards::builders::{CardTextError, EffectAst, OwnedLexToken, PlayerAst, SubjectAst};
use crate::effect::Value;
use crate::mana::ManaSymbol;
use crate::target::ObjectFilter;

use super::activation_and_restrictions::parse_devotion_value_from_add_clause;
use super::effect_sentences::clause_pattern_helpers::extract_subject_player;
use super::effect_sentences::conditionals::parse_predicate;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_add_mana_that_much_value,
    parse_dynamic_cost_modifier_value, parse_where_x_is_number_of_filter_value,
};
use super::native_tokens::LowercaseWordView;
pub(crate) use super::object_filters::is_comparison_or_delimiter;
use super::object_filters::parse_object_filter;
pub(crate) use super::util::{
    contains_discard_source_phrase, contains_source_from_your_graveyard_phrase,
    contains_source_from_your_hand_phrase, find_activation_cost_start, is_article,
    is_basic_color_word, is_source_from_your_graveyard_words, join_sentences_with_period,
    mana_pips_from_token, parse_mana_symbol, parse_next_end_step_token_delay_flags,
    parse_subtype_flexible, parse_value, split_cost_segments, token_index_for_word_index,
    trim_commas, value_contains_unbound_x,
};
pub(crate) use super::value_helpers::parse_filter_comparison_tokens;

fn word_slice_starts_with(words: &[&str], expected: &[&str]) -> bool {
    if words.len() < expected.len() {
        return false;
    }

    let mut idx = 0usize;
    while idx < expected.len() {
        if words[idx] != expected[idx] {
            return false;
        }
        idx += 1;
    }

    true
}

fn word_slice_ends_with(words: &[&str], expected: &[&str]) -> bool {
    if words.len() < expected.len() {
        return false;
    }

    let start = words.len() - expected.len();
    let mut idx = 0usize;
    while idx < expected.len() {
        if words[start + idx] != expected[idx] {
            return false;
        }
        idx += 1;
    }

    true
}

fn word_slice_contains(words: &[&str], expected: &str) -> bool {
    let mut idx = 0usize;
    while idx < words.len() {
        if words[idx] == expected {
            return true;
        }
        idx += 1;
    }

    false
}

fn find_word_sequence_index(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() || words.len() < expected.len() {
        return None;
    }

    let mut start = 0usize;
    while start + expected.len() <= words.len() {
        if word_slice_starts_with(&words[start..], expected) {
            return Some(start);
        }
        start += 1;
    }

    None
}

fn push_unique_color(colors: &mut Vec<crate::color::Color>, color: crate::color::Color) {
    let mut idx = 0usize;
    while idx < colors.len() {
        if colors[idx] == color {
            return;
        }
        idx += 1;
    }

    colors.push(color);
}

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> usize {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if !tokens[idx].is_comma() {
            return idx;
        }
        idx += 1;
    }

    tokens.len()
}

pub(crate) fn parse_add_mana(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[OwnedLexToken]|
     -> Result<Option<EffectAst>, CardTextError> {
        let tail_word_view = LowercaseWordView::new(tail_tokens);
        let tail_words = tail_word_view.to_word_refs();
        if !word_slice_starts_with(&tail_words, &["instead", "if"]) {
            return Ok(None);
        }
        let predicate_tokens = trim_commas(&tail_tokens[2..]);
        if predicate_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let predicate = parse_predicate(&predicate_tokens).map_err(|_| {
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
    if word_slice_contains(&clause_words, "exiled")
        && has_card_word
        && word_slice_contains(&clause_words, "colors")
    {
        return Ok(EffectAst::AddManaImprintedColors);
    }

    if (word_slice_contains(&clause_words, "commander")
        || word_slice_contains(&clause_words, "commanders"))
        && word_slice_contains(&clause_words, "color")
        && word_slice_contains(&clause_words, "identity")
    {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaCommanderIdentity { amount, player });
    }

    if let Some(available_colors) = parse_any_combination_mana_colors(tokens)? {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaAnyColor {
            amount,
            player,
            available_colors: Some(available_colors),
        });
    }

    if let Some(available_colors) = parse_or_mana_color_choices(tokens)? {
        return Ok(EffectAst::AddManaAnyColor {
            amount: Value::Fixed(1),
            player,
            available_colors: Some(available_colors),
        });
    }

    let has_explicit_symbol = tokens
        .iter()
        .any(|token| mana_pips_from_token(token).is_some());
    if !has_explicit_symbol
        && let Some(chosen_idx) = find_word_sequence_index(&clause_words, &["chosen", "color"])
    {
        let prefix = &clause_words[..chosen_idx];
        let references_mana_of_chosen_color = word_slice_ends_with(prefix, &["mana", "of", "the"])
            || word_slice_ends_with(prefix, &["mana", "of"]);
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
                return Ok(EffectAst::AddManaChosenColor {
                    amount,
                    player,
                    fixed_option: None,
                });
            }
        }
    }
    if word_slice_starts_with(
        &clause_words,
        &["an", "amount", "of", "mana", "of", "that", "color"],
    ) {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaChosenColor {
            amount,
            player,
            fixed_option: None,
        });
    }

    let any_one = find_word_sequence_index(&clause_words, &["any", "one", "color"]).is_some()
        || find_word_sequence_index(&clause_words, &["any", "one", "type"]).is_some();
    let any_color = find_word_sequence_index(&clause_words, &["any", "color"]).is_some()
        || find_word_sequence_index(&clause_words, &["one", "color"]).is_some();
    let any_type = find_word_sequence_index(&clause_words, &["any", "type"]).is_some()
        || find_word_sequence_index(&clause_words, &["one", "type"]).is_some();
    if any_color || any_type {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let allow_colorless = any_type;
        let phrase_end = tokens
            .iter()
            .enumerate()
            .find_map(|(idx, token)| {
                let word = token.as_word()?;
                if (word == "color" && any_color) || (word == "type" && any_type) {
                    Some(idx + 1)
                } else {
                    None
                }
            })
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
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        if let Some(filter) = parse_land_could_produce_filter(tail_tokens)? {
            return Ok(EffectAst::AddManaFromLandCouldProduce {
                amount,
                player,
                land_filter: filter,
                allow_colorless,
                same_type: any_one,
            });
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
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        let tail_word_view = LowercaseWordView::new(tail_tokens);
        let tail_words = tail_word_view.to_word_refs();
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
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }
        if tail_words.first().copied() == Some("for")
            && tail_words.get(1).copied() == Some("each")
            && word_slice_ends_with(&tail_words, &["removed", "this", "way"])
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
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        if tail_words.first().copied() == Some("among") {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        let base_effect = if any_one {
            EffectAst::AddManaAnyOneColor { amount, player }
        } else {
            EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            }
        };
        if let Some(conditional) = wrap_instead_if_tail(base_effect, tail_tokens)? {
            return Ok(conditional);
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut for_each_idx = None;
    let mut token_idx = 0usize;
    while token_idx + 1 < tokens.len() {
        if tokens[token_idx].is_word("for") && tokens[token_idx + 1].is_word("each") {
            for_each_idx = Some(token_idx);
            break;
        }
        token_idx += 1;
    }
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
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(amount) = parse_devotion_value_from_add_clause(tokens)? {
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(for_each_idx) = for_each_idx {
            let amount_tokens = &tokens[for_each_idx..];
            let amount = parse_dynamic_cost_modifier_value(amount_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic mana amount (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(amount) = parse_add_mana_equal_amount_value(tokens) {
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        let trailing_word_view = last_mana_idx
            .map(|last_idx| LowercaseWordView::new(&tokens[last_idx + 1..]));
        let trailing_words = trailing_word_view
            .as_ref()
            .map(LowercaseWordView::to_word_refs)
            .unwrap_or_default();
        if !trailing_words.is_empty() {
            let chosen_color_tail = word_slice_starts_with(
                &trailing_words,
                &["or", "one", "mana", "of", "the", "chosen", "color"],
            );
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
                return Ok(EffectAst::AddManaChosenColor {
                    amount: Value::Fixed(1),
                    player,
                    fixed_option: Some(color),
                });
            }
        }
        let has_only_pool_tail = !trailing_words.is_empty()
            && trailing_words
                .iter()
                .all(|word| matches!(*word, "to" | "your" | "mana" | "pool"));
        let has_only_instead_tail = trailing_words.as_slice() == ["instead"];
        if !trailing_words.is_empty() && !has_only_pool_tail && !has_only_instead_tail {
            if let Some(last_idx) = last_mana_idx
                && let Some(conditional) = wrap_instead_if_tail(
                    EffectAst::AddMana {
                        mana: mana.clone(),
                        player,
                    },
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
        return Ok(EffectAst::AddMana { mana, player });
    }

    Err(CardTextError::ParseError(format!(
        "missing mana symbols (clause: '{}')",
        clause_words.join(" ")
    )))
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
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !word_slice_contains(&clause_words, "or") {
        return Ok(None);
    }

    let mut colors = Vec::new();
    let mut has_or = false;
    for token in tokens {
        if token.is_word("or") {
            has_or = true;
            continue;
        }
        if let Some(group) = mana_pips_from_token(token) {
            for symbol in group {
                let Some(color) = mana_symbol_to_color(symbol) else {
                    return Ok(None);
                };
                push_unique_color(&mut colors, color);
            }
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(
            word.to_ascii_lowercase().as_str(),
            "to" | "your" | "their" | "its" | "mana" | "pool"
        ) {
            continue;
        }
        return Ok(None);
    }

    if !has_or || colors.len() < 2 {
        return Ok(None);
    }

    Ok(Some(colors))
}

pub(crate) fn parse_any_combination_mana_colors(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let Some(combination_idx) = find_word_sequence_index(&clause_words, &["any", "combination", "of"]) else {
        return Ok(None);
    };

    let mut colors = Vec::new();
    for word in &clause_words[combination_idx + 3..] {
        if *word == "where" {
            break;
        }
        if matches!(
            *word,
            "and" | "or" | "and/or" | "mana" | "to" | "your" | "their" | "its" | "pool"
        ) {
            continue;
        }
        if matches!(*word, "color" | "colors") {
            for color in [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ] {
                push_unique_color(&mut colors, color);
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
        push_unique_color(&mut colors, color);
    }

    if colors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing color options in any-combination mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(colors))
}

pub(crate) fn trim_leading_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let start = first_non_comma_token_index(tokens);
    &tokens[start..]
}

pub(crate) fn is_mana_pool_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    let word_view = LowercaseWordView::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty()
        || words[0] != "to"
        || !word_slice_contains(&words, "mana")
        || !word_slice_contains(&words, "pool")
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

pub(crate) fn parse_land_could_produce_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let word_view = LowercaseWordView::new(tokens);
    let words = word_view.to_word_refs();
    if words.len() < 3 || words[0] != "that" {
        return Ok(None);
    }

    let marker_word_idx = if let Some(could_idx) = find_word_sequence_index(&words, &["could", "produce"]) {
        if could_idx + 2 != words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (tail: '{}')",
                words.join(" ")
            )));
        }
        could_idx
    } else {
        let mut produced_idx = None;
        let mut idx = 0usize;
        while idx < words.len() {
            if words[idx] == "produced" {
                produced_idx = Some(idx);
                break;
            }
            idx += 1;
        }
        let Some(produced_idx) = produced_idx else {
            return Ok(None);
        };
        if produced_idx + 1 != words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (tail: '{}')",
                words.join(" ")
            )));
        }
        produced_idx
    };

    let marker_token_idx = word_view
        .token_index_for_word_index(marker_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing mana production marker in tail '{}'",
                words.join(" ")
            ))
        })?;
    let filter_tokens = trim_leading_commas(&tokens[1..marker_token_idx]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing land filter in mana clause (tail: '{}')",
            words.join(" ")
        )));
    }
    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(filter))
}
