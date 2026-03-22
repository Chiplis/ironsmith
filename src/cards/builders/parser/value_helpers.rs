#![allow(dead_code)]

use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::Value;
use crate::target::{ChooseSpec, PlayerFilter};

use super::effect_sentences::trim_edge_punctuation;
use super::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use super::native_tokens::LowercaseWordView;
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::util::{
    is_article, parse_counter_type_word, parse_number, parse_number_word_i32, parse_value,
    parse_value_expr_words, token_index_for_word_index, trim_commas, words,
};

fn parse_spells_cast_this_turn_matching_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = LowercaseWordView::new(tokens);
    let filter_words = word_view.to_word_refs();
    if !(filter_words.contains(&"spell") || filter_words.contains(&"spells"))
        || !(filter_words.contains(&"cast") || filter_words.contains(&"casts"))
        || !filter_words.contains(&"this")
        || !filter_words.contains(&"turn")
    {
        return None;
    }

    let suffix_patterns: &[(&[&str], PlayerFilter)] = &[
        (
            &["theyve", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["they", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["that", "player", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (&["youve", "cast", "this", "turn"], PlayerFilter::You),
        (&["you", "cast", "this", "turn"], PlayerFilter::You),
        (
            &["an", "opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponents", "have", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (&["cast", "this", "turn"], PlayerFilter::Any),
    ];

    for (suffix, player) in suffix_patterns {
        if !filter_words.ends_with(suffix) {
            continue;
        }
        let filter_word_len = filter_words.len().saturating_sub(suffix.len());
        let filter_token_end =
            token_index_for_word_index(tokens, filter_word_len).unwrap_or(tokens.len());
        let filter_tokens = trim_commas(&tokens[..filter_token_end]);
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        let exclude_source = filter_tokens.iter().any(|token| token.is_word("other"));
        return Some(Value::SpellsCastThisTurnMatching {
            player: player.clone(),
            filter,
            exclude_source,
        });
    }

    None
}

fn trim_lexed_edge_punctuation(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        end -= 1;
    }
    &tokens[start..end]
}

fn lower_words_end_with(words: &LowercaseWordView, suffix: &[&str]) -> bool {
    words.len() >= suffix.len() && words.slice_eq(words.len() - suffix.len(), suffix)
}

pub(crate) fn parse_number_from_lexed(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    let trimmed = trim_lexed_edge_punctuation(tokens);
    let first_word = trimmed.first()?.as_word()?.to_ascii_lowercase();
    let value: u32 = parse_number_word_i32(&first_word).and_then(|value| value.try_into().ok())?;
    Some((value, 1))
}

pub(crate) fn parse_value_from_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let trimmed = trim_lexed_edge_punctuation(tokens);
    let words = LowercaseWordView::new(trimmed);
    let word_refs = words.to_word_refs();
    let (value, used_words) = parse_value_expr_words(&word_refs)?;
    let used_tokens = words
        .token_index_for_word_index(used_words)
        .unwrap_or(trimmed.len());
    Some((value, used_tokens))
}

pub(crate) fn parse_equal_to_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = LowercaseWordView::new(tokens);
    let words_all = word_view.to_word_refs();
    let equal_idx = words_all
        .windows(2)
        .position(|window| window == ["equal", "to"])?;
    let mut number_word_idx = equal_idx + 2;
    if words_all.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if words_all.get(number_word_idx).copied() != Some("number")
        || words_all.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let value_start_token_idx = token_index_for_word_index(tokens, number_word_idx)?;
    let value_tokens = trim_edge_punctuation(&tokens[value_start_token_idx..]);
    if let Some((value, used)) = parse_value(&value_tokens)
        && words(&value_tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_start_token_idx..]);
    let filter_word_view = LowercaseWordView::new(&filter_tokens);
    let filter_words = filter_word_view.to_word_refs();
    if filter_words.contains(&"cards")
        && filter_words.contains(&"in")
        && (filter_words.contains(&"hand") || filter_words.contains(&"hands"))
    {
        if filter_words.contains(&"your") {
            return Some(Value::CardsInHand(PlayerFilter::You));
        }
        if filter_words.contains(&"their")
            || filter_words.windows(2).any(|window| {
                matches!(
                    window,
                    ["that", "player"] | ["that", "players"] | ["the", "chosen"]
                )
            })
        {
            return Some(Value::CardsInHand(PlayerFilter::IteratedPlayer));
        }
        if filter_words.contains(&"opponent") || filter_words.contains(&"opponents") {
            return Some(Value::CardsInHand(PlayerFilter::Opponent));
        }
    }
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        return Some(value);
    }
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = LowercaseWordView::new(tokens);
    let clause_words = word_view.to_word_refs();
    if !clause_words.starts_with(&["equal", "to"]) {
        return None;
    }

    let mut number_word_idx = 2usize;
    if clause_words.get(number_word_idx).copied() == Some("the") {
        number_word_idx += 1;
    }
    if clause_words.get(number_word_idx).copied() != Some("number")
        || clause_words.get(number_word_idx + 1).copied() != Some("of")
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx = (filter_start_word_idx + 1..clause_words.len())
        .find(|idx| matches!(clause_words[*idx], "plus" | "minus"))?;
    let operator = clause_words[operator_word_idx];

    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let operator_token_idx = token_index_for_word_index(tokens, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let base_value =
        if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
            value
        } else {
            Value::Count(parse_object_filter(&filter_tokens, false).ok()?)
        };

    let offset_start_token_idx = token_index_for_word_index(tokens, operator_word_idx + 1)?;
    let offset_tokens = trim_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    if !words(&offset_tokens[used..]).is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(base_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn parse_equal_to_number_of_opponents_you_have_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = words(tokens);
    if matches!(
        clause_words.as_slice(),
        [
            "equal",
            "to",
            "the",
            "number",
            "of",
            "opponents",
            "you",
            "have"
        ] | ["equal", "to", "number", "of", "opponents", "you", "have"]
    ) {
        return Some(Value::CountPlayers(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["equal", "to"]) {
        return None;
    }

    let mut idx = 2usize;
    if clause_words.get(idx).copied() == Some("the") {
        idx += 1;
    }
    if clause_words.get(idx).copied() != Some("number")
        || clause_words.get(idx + 1).copied() != Some("of")
    {
        return None;
    }
    idx += 2;

    if clause_words
        .get(idx)
        .is_some_and(|word| is_article(word) || *word == "one")
    {
        idx += 1;
    }

    let mut counter_type = None;
    if let Some(word) = clause_words.get(idx).copied()
        && let Some(parsed) = parse_counter_type_word(word)
    {
        counter_type = Some(parsed);
        idx += 1;
    }

    if !matches!(clause_words.get(idx).copied(), Some("counter" | "counters")) {
        return None;
    }
    idx += 1;

    if clause_words.get(idx).copied() != Some("on") {
        return None;
    }
    idx += 1;

    let reference = &clause_words[idx..];
    if reference.is_empty() {
        return None;
    }

    if matches!(
        reference,
        ["it"] | ["this"] | ["this", "creature"] | ["this", "permanent"] | ["this", "source"]
    ) {
        return Some(match counter_type {
            Some(counter_type) => Value::CountersOnSource(counter_type),
            None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
        });
    }

    if matches!(
        reference,
        ["that"]
            | ["that", "creature"]
            | ["that", "permanent"]
            | ["that", "object"]
            | ["those"]
            | ["those", "creatures"]
            | ["those", "permanents"]
    ) {
        return Some(Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            counter_type,
        ));
    }

    None
}

pub(crate) fn parse_equal_to_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = words(tokens);
    let equal_idx = clause_words
        .windows(2)
        .position(|window| window == ["equal", "to"])?;

    let mut idx = equal_idx + 2;
    if clause_words.get(idx).copied() == Some("the") {
        idx += 1;
    }

    let aggregate = match clause_words.get(idx).copied() {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if clause_words.get(idx).copied() == Some("power") {
        idx += 1;
        "power"
    } else if clause_words.get(idx).copied() == Some("toughness") {
        idx += 1;
        "toughness"
    } else if clause_words.get(idx).copied() == Some("mana")
        && clause_words.get(idx + 1).copied() == Some("value")
    {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !matches!(clause_words.get(idx).copied(), Some("of" | "among")) {
        return None;
    }
    idx += 1;

    let object_start_token_idx = token_index_for_word_index(tokens, idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "toughness") => Some(Value::GreatestToughness(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_equal_to_number_of_filter_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words_all = LowercaseWordView::new(tokens);
    let words_refs = words_all.to_word_refs();
    let equal_idx = words_refs
        .windows(2)
        .position(|window| window == ["equal", "to"])?;
    let mut number_word_idx = equal_idx + 2;
    if words_all.get(number_word_idx) == Some("the") {
        number_word_idx += 1;
    }
    if words_all.get(number_word_idx) != Some("number")
        || words_all.get(number_word_idx + 1) != Some("of")
    {
        return None;
    }

    let value_start_token_idx = words_all.token_index_for_word_index(number_word_idx)?;
    let value_tokens = trim_lexed_edge_punctuation(&tokens[value_start_token_idx..]);
    if let Some((value, used)) = parse_value_from_lexed(value_tokens) {
        if words(&value_tokens[used..]).is_empty() {
            return Some(value);
        }
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_start_token_idx = words_all.token_index_for_word_index(filter_start_word_idx)?;
    let filter_tokens = trim_lexed_edge_punctuation(&tokens[filter_start_token_idx..]);
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value_lexed(filter_tokens) {
        return Some(value);
    }
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = LowercaseWordView::new(tokens);
    if !clause_words.starts_with(&["equal", "to"]) {
        return None;
    }

    let mut number_word_idx = 2usize;
    if clause_words.get(number_word_idx) == Some("the") {
        number_word_idx += 1;
    }
    if clause_words.get(number_word_idx) != Some("number")
        || clause_words.get(number_word_idx + 1) != Some("of")
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx = (filter_start_word_idx + 1..clause_words.len())
        .find(|idx| matches!(clause_words.get(*idx), Some("plus" | "minus")))?;
    let operator = clause_words.get(operator_word_idx)?;

    let filter_start_token_idx = clause_words.token_index_for_word_index(filter_start_word_idx)?;
    let operator_token_idx = clause_words.token_index_for_word_index(operator_word_idx)?;
    let filter_tokens = trim_lexed_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let base_value = if let Some(value) =
        parse_spells_cast_this_turn_matching_count_value_lexed(filter_tokens)
    {
        value
    } else {
        Value::Count(parse_object_filter_lexed(filter_tokens, false).ok()?)
    };

    let offset_start_token_idx = clause_words.token_index_for_word_index(operator_word_idx + 1)?;
    let offset_tokens = trim_lexed_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number_from_lexed(offset_tokens)?;
    if !words(&offset_tokens[used..]).is_empty() {
        return None;
    }

    let signed_offset = if operator == "minus" {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(base_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn parse_equal_to_number_of_opponents_you_have_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = LowercaseWordView::new(tokens);
    if clause_words.starts_with(&[
        "equal",
        "to",
        "the",
        "number",
        "of",
        "opponents",
        "you",
        "have",
    ]) || clause_words.starts_with(&["equal", "to", "number", "of", "opponents", "you", "have"])
    {
        return Some(Value::CountPlayers(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = LowercaseWordView::new(tokens);
    if !clause_words.starts_with(&["equal", "to"]) {
        return None;
    }

    let mut idx = 2usize;
    if clause_words.get(idx) == Some("the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some("number") || clause_words.get(idx + 1) != Some("of") {
        return None;
    }
    idx += 2;

    if clause_words
        .get(idx)
        .is_some_and(|word| is_article(word) || word == "one")
    {
        idx += 1;
    }

    let mut counter_type = None;
    if let Some(word) = clause_words.get(idx)
        && let Some(parsed) = parse_counter_type_word(word)
    {
        counter_type = Some(parsed);
        idx += 1;
    }

    if !matches!(clause_words.get(idx), Some("counter" | "counters")) {
        return None;
    }
    idx += 1;

    if clause_words.get(idx) != Some("on") {
        return None;
    }
    idx += 1;

    let reference = &clause_words.to_word_refs()[idx..];
    if reference.is_empty() {
        return None;
    }

    if matches!(
        reference,
        ["it"] | ["this"] | ["this", "creature"] | ["this", "permanent"] | ["this", "source"]
    ) {
        return Some(match counter_type {
            Some(counter_type) => Value::CountersOnSource(counter_type),
            None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
        });
    }

    if matches!(
        reference,
        ["that"]
            | ["that", "creature"]
            | ["that", "permanent"]
            | ["that", "object"]
            | ["those"]
            | ["those", "creatures"]
            | ["those", "permanents"]
    ) {
        return Some(Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            counter_type,
        ));
    }

    None
}

pub(crate) fn parse_equal_to_aggregate_filter_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = LowercaseWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let equal_idx = clause_refs
        .windows(2)
        .position(|window| window == ["equal", "to"])?;

    let mut idx = equal_idx + 2;
    if clause_words.get(idx) == Some("the") {
        idx += 1;
    }

    let aggregate = match clause_words.get(idx) {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if clause_words.get(idx) == Some("power") {
        idx += 1;
        "power"
    } else if clause_words.get(idx) == Some("toughness") {
        idx += 1;
        "toughness"
    } else if clause_words.get(idx) == Some("mana") && clause_words.get(idx + 1) == Some("value") {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !matches!(clause_words.get(idx), Some("of" | "among")) {
        return None;
    }
    idx += 1;

    let object_start_token_idx = clause_words.token_index_for_word_index(idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "toughness") => Some(Value::GreatestToughness(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_spells_cast_this_turn_matching_count_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_words = LowercaseWordView::new(tokens);
    let filter_refs = filter_words.to_word_refs();
    if !(filter_refs.contains(&"spell") || filter_refs.contains(&"spells"))
        || !(filter_refs.contains(&"cast") || filter_refs.contains(&"casts"))
        || !filter_refs.contains(&"this")
        || !filter_refs.contains(&"turn")
    {
        return None;
    }

    let suffix_patterns: &[(&[&str], PlayerFilter)] = &[
        (
            &["theyve", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["they", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["that", "player", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (&["youve", "cast", "this", "turn"], PlayerFilter::You),
        (&["you", "cast", "this", "turn"], PlayerFilter::You),
        (
            &["an", "opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponents", "have", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (&["cast", "this", "turn"], PlayerFilter::Any),
    ];

    for (suffix, player) in suffix_patterns {
        if !lower_words_end_with(&filter_words, suffix) {
            continue;
        }
        let filter_word_len = filter_words.len().saturating_sub(suffix.len());
        let filter_token_end = filter_words
            .token_index_for_word_index(filter_word_len)
            .unwrap_or(tokens.len());
        let filter_tokens = trim_lexed_commas(&tokens[..filter_token_end]);
        let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
        let exclude_source = filter_tokens.iter().any(|token| token.is_word("other"));
        return Some(Value::SpellsCastThisTurnMatching {
            player: player.clone(),
            filter,
            exclude_source,
        });
    }

    None
}

pub(crate) fn parse_filter_comparison_tokens(
    axis: &str,
    tokens: &[&str],
    clause_words: &[&str],
) -> Result<Option<(crate::filter::Comparison, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let to_comparison = |kind: &str, operand: Value| -> crate::filter::Comparison {
        use crate::filter::Comparison;

        match (kind, operand) {
            ("eq", Value::Fixed(value)) => Comparison::Equal(value),
            ("neq", Value::Fixed(value)) => Comparison::NotEqual(value),
            ("lt", Value::Fixed(value)) => Comparison::LessThan(value),
            ("lte", Value::Fixed(value)) => Comparison::LessThanOrEqual(value),
            ("gt", Value::Fixed(value)) => Comparison::GreaterThan(value),
            ("gte", Value::Fixed(value)) => Comparison::GreaterThanOrEqual(value),
            ("eq", operand) => Comparison::EqualExpr(Box::new(operand)),
            ("neq", operand) => Comparison::NotEqualExpr(Box::new(operand)),
            ("lt", operand) => Comparison::LessThanExpr(Box::new(operand)),
            ("lte", operand) => Comparison::LessThanOrEqualExpr(Box::new(operand)),
            ("gt", operand) => Comparison::GreaterThanExpr(Box::new(operand)),
            ("gte", operand) => Comparison::GreaterThanOrEqualExpr(Box::new(operand)),
            _ => unreachable!("unsupported comparison kind"),
        }
    };

    let parse_operand = |operand_tokens: &[&str],
                         comparison_kind: &str|
     -> Result<(crate::filter::Comparison, usize), CardTextError> {
        let Some((operand, used)) = parse_value_expr_words(operand_tokens) else {
            let quoted = operand_tokens
                .first()
                .copied()
                .unwrap_or_default()
                .to_string();
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        Ok((to_comparison(comparison_kind, operand), used))
    };

    let parse_numeric_token = |word: &str| -> Option<i32> {
        if let Ok(value) = word.parse::<i32>() {
            return Some(value);
        }
        parse_number_word_i32(word)
    };

    let first = tokens[0];
    if let Some(value) = parse_numeric_token(first) {
        if tokens
            .get(1)
            .is_some_and(|word| matches!(*word, "plus" | "minus"))
        {
            let (cmp, used) = parse_operand(tokens, "eq")?;
            return Ok(Some((cmp, used)));
        }
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "greater" | "more"))
        {
            return Ok(Some((
                crate::filter::Comparison::GreaterThanOrEqual(value),
                3,
            )));
        }
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "less" | "fewer"))
        {
            return Ok(Some((crate::filter::Comparison::LessThanOrEqual(value), 3)));
        }
        let mut values = vec![value];
        let mut consumed = 1usize;
        while consumed < tokens.len() {
            let token = tokens[consumed];
            if matches!(token, "and" | "or" | "and/or") {
                consumed += 1;
                continue;
            }
            if let Some(next_value) = parse_numeric_token(token) {
                values.push(next_value);
                consumed += 1;
                continue;
            }
            break;
        }
        if values.len() > 1 {
            return Ok(Some((crate::filter::Comparison::OneOf(values), consumed)));
        }
        return Ok(Some((crate::filter::Comparison::Equal(value), 1)));
    }

    if first == "equal" && tokens.get(1) == Some(&"to") {
        if tokens.get(2).is_none() {
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after 'equal to' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (cmp, used) = parse_operand(&tokens[2..], "eq")?;
        return Ok(Some((cmp, 2 + used)));
    }

    if matches!(first, "less" | "greater") && tokens.get(1) == Some(&"than") {
        let mut operand_idx = 2usize;
        let mut inclusive = false;
        if tokens.get(operand_idx) == Some(&"or")
            && tokens.get(operand_idx + 1) == Some(&"equal")
            && tokens.get(operand_idx + 2) == Some(&"to")
        {
            inclusive = true;
            operand_idx += 3;
        }
        if tokens.get(operand_idx).is_none() {
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{first} than' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (cmp, used) = parse_operand(
            &tokens[operand_idx..],
            match (first, inclusive) {
                ("less", true) => "lte",
                ("less", false) => "lt",
                ("greater", true) => "gte",
                ("greater", false) => "gt",
                _ => unreachable!("first is constrained above"),
            },
        )?;
        let parsed = match (first, inclusive, cmp) {
            ("less", true, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::LessThanOrEqual(v)
            }
            ("less", false, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::LessThan(v)
            }
            ("greater", true, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::GreaterThanOrEqual(v)
            }
            ("greater", false, crate::filter::Comparison::Equal(v)) => {
                crate::filter::Comparison::GreaterThan(v)
            }
            (_, _, other) => other,
        };
        return Ok(Some((parsed, operand_idx + used)));
    }

    if first == "not" && tokens.get(1) == Some(&"equal") && tokens.get(2) == Some(&"to") {
        let (cmp, used) = parse_operand(&tokens[3..], "neq")?;
        return Ok(Some((cmp, 3 + used)));
    }

    if first == "equal" && tokens.get(1) == Some(&"to") && tokens.get(2) == Some(&"x") {
        return Ok(Some((crate::filter::Comparison::GreaterThanOrEqual(0), 3)));
    }

    if let Some((value, used)) = parse_value_expr_words(tokens) {
        if let Value::Fixed(fixed) = value
            && used == 1
        {
            return Ok(Some((crate::filter::Comparison::Equal(fixed), used)));
        }
        return Ok(Some((
            crate::filter::Comparison::EqualExpr(Box::new(value)),
            used,
        )));
    }

    if first == "x" {
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "less" | "fewer" | "greater" | "more"))
        {
            return Ok(Some((crate::filter::Comparison::GreaterThanOrEqual(0), 3)));
        }
        return Ok(Some((crate::filter::Comparison::GreaterThanOrEqual(0), 1)));
    }

    Ok(None)
}
