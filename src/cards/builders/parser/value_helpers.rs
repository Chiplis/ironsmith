#![allow(dead_code)]

use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::target::{ChooseSpec, PlayerFilter};

use super::effect_sentences::trim_edge_punctuation;
use super::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use super::native_tokens::LowercaseWordView;
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::token_primitives::parse_value_comparison_tokens;
use super::util::{
    is_article, parse_counter_type_word, parse_number, parse_number_word_i32, parse_value,
    parse_value_expr_words, token_index_for_word_index, trim_commas,
};

fn word_refs_have(words: &[&str], expected: &str) -> bool {
    let mut idx = 0usize;
    while idx < words.len() {
        if words[idx] == expected {
            return true;
        }
        idx += 1;
    }

    false
}

fn word_refs_have_prefix(words: &[&str], prefix: &[&str]) -> bool {
    words.len() >= prefix.len() && words[..prefix.len()] == *prefix
}

fn word_refs_have_suffix(words: &[&str], suffix: &[&str]) -> bool {
    words.len() >= suffix.len() && words[words.len() - suffix.len()..] == *suffix
}

fn word_refs_find_sequence(words: &[&str], phrase: &[&str]) -> Option<usize> {
    if phrase.is_empty() || words.len() < phrase.len() {
        return None;
    }

    let mut idx = 0usize;
    while idx + phrase.len() <= words.len() {
        if word_refs_have_prefix(&words[idx..], phrase) {
            return Some(idx);
        }
        idx += 1;
    }

    None
}

fn lower_words_have(words: &LowercaseWordView, expected: &str) -> bool {
    let mut idx = 0usize;
    while idx < words.len() {
        if words.get(idx) == Some(expected) {
            return true;
        }
        idx += 1;
    }

    false
}

fn lower_words_have_any(words: &LowercaseWordView, expected: &[&str]) -> bool {
    let mut idx = 0usize;
    while idx < words.len() {
        if let Some(word) = words.get(idx)
            && expected.iter().any(|candidate| *candidate == word)
        {
            return true;
        }
        idx += 1;
    }

    false
}

fn lower_words_start_with(words: &LowercaseWordView, prefix: &[&str]) -> bool {
    words.len() >= prefix.len() && words.slice_eq(0, prefix)
}

fn lower_words_find_first_of(
    words: &LowercaseWordView,
    candidates: &[&str],
    start: usize,
) -> Option<usize> {
    let mut idx = start;
    while idx < words.len() {
        if let Some(word) = words.get(idx)
            && candidates.iter().any(|candidate| *candidate == word)
        {
            return Some(idx);
        }
        idx += 1;
    }

    None
}

fn token_slice_has_no_words(tokens: &[OwnedLexToken]) -> bool {
    LowercaseWordView::new(tokens).len() == 0
}

fn parse_spells_cast_this_turn_matching_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = LowercaseWordView::new(tokens);
    let filter_words = word_view.to_word_refs();
    if !lower_words_have_any(&word_view, &["spell", "spells"])
        || !lower_words_have_any(&word_view, &["cast", "casts"])
        || !lower_words_have(&word_view, "this")
        || !lower_words_have(&word_view, "turn")
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
        if !word_refs_have_suffix(&filter_words, suffix) {
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
    let equal_idx = word_refs_find_sequence(&words_all, &["equal", "to"])?;
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
        && token_slice_has_no_words(&value_tokens[used..])
    {
        return Some(value);
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_start_token_idx..]);
    let filter_word_view = LowercaseWordView::new(&filter_tokens);
    let filter_words = filter_word_view.to_word_refs();
    if lower_words_have(&filter_word_view, "cards")
        && lower_words_have(&filter_word_view, "in")
        && lower_words_have_any(&filter_word_view, &["hand", "hands"])
    {
        if lower_words_have(&filter_word_view, "your") {
            return Some(Value::CardsInHand(PlayerFilter::You));
        }
        if lower_words_have(&filter_word_view, "their")
            || word_refs_find_sequence(&filter_words, &["that", "player"]).is_some()
            || word_refs_find_sequence(&filter_words, &["that", "players"]).is_some()
            || word_refs_find_sequence(&filter_words, &["the", "chosen"]).is_some()
        {
            return Some(Value::CardsInHand(PlayerFilter::IteratedPlayer));
        }
        if lower_words_have_any(&filter_word_view, &["opponent", "opponents"]) {
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
    if !word_refs_have_prefix(&clause_words, &["equal", "to"]) {
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
    let operator_word_idx =
        lower_words_find_first_of(&word_view, &["plus", "minus"], filter_start_word_idx + 1)?;
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
    if !token_slice_has_no_words(&offset_tokens[used..]) {
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
    let clause_words = LowercaseWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if matches!(
        clause_refs.as_slice(),
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
    let clause_words = LowercaseWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if !word_refs_have_prefix(&clause_refs, &["equal", "to"]) {
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

    let reference = &clause_refs[idx..];
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
    let clause_words = LowercaseWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let equal_idx = word_refs_find_sequence(&clause_refs, &["equal", "to"])?;

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
    let equal_idx = word_refs_find_sequence(&words_refs, &["equal", "to"])?;
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
        if token_slice_has_no_words(&value_tokens[used..]) {
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
    if !lower_words_start_with(&clause_words, &["equal", "to"]) {
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
    let operator_word_idx =
        lower_words_find_first_of(&clause_words, &["plus", "minus"], filter_start_word_idx + 1)?;
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
    if !token_slice_has_no_words(&offset_tokens[used..]) {
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
    if lower_words_start_with(
        &clause_words,
        &[
            "equal",
            "to",
            "the",
            "number",
            "of",
            "opponents",
            "you",
            "have",
        ],
    ) || lower_words_start_with(
        &clause_words,
        &["equal", "to", "number", "of", "opponents", "you", "have"],
    ) {
        return Some(Value::CountPlayers(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = LowercaseWordView::new(tokens);
    if !lower_words_start_with(&clause_words, &["equal", "to"]) {
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
    let equal_idx = word_refs_find_sequence(&clause_refs, &["equal", "to"])?;

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
    if !lower_words_have_any(&filter_words, &["spell", "spells"])
        || !lower_words_have_any(&filter_words, &["cast", "casts"])
        || !lower_words_have(&filter_words, "this")
        || !lower_words_have(&filter_words, "turn")
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

    if matches!(axis, "power" | "toughness") && matches!(tokens, ["or", "power" | "toughness", ..])
    {
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

    let map_operator =
        |operator: ValueComparisonOperator, operand: Value| -> crate::filter::Comparison {
            match operator {
                ValueComparisonOperator::Equal => to_comparison("eq", operand),
                ValueComparisonOperator::NotEqual => to_comparison("neq", operand),
                ValueComparisonOperator::LessThan => to_comparison("lt", operand),
                ValueComparisonOperator::LessThanOrEqual => to_comparison("lte", operand),
                ValueComparisonOperator::GreaterThan => to_comparison("gt", operand),
                ValueComparisonOperator::GreaterThanOrEqual => to_comparison("gte", operand),
            }
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
        if tokens.len() == 1 {
            return Ok(Some((crate::filter::Comparison::Equal(value), 1)));
        }
    }

    let synthetic_tokens = tokens
        .iter()
        .map(|word| OwnedLexToken::synthetic_word(*word))
        .collect::<Vec<_>>();
    if let Some((operator, operand_tokens)) = parse_value_comparison_tokens(&synthetic_tokens) {
        let operand_len = operand_tokens.len();
        let operand_start = if operand_len == 0
            || std::ptr::eq(operand_tokens.as_ptr(), synthetic_tokens.as_ptr())
        {
            0
        } else {
            synthetic_tokens.len().saturating_sub(operand_len)
        };
        let operand_words = operand_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        if operand_words.is_empty() {
            let consumed_phrase = if operand_start == 0 {
                synthetic_tokens.len()
            } else {
                operand_start
            };
            let phrase = tokens[..consumed_phrase].join(" ");
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{phrase}' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (operand, used) = parse_value_expr_words(&operand_words).ok_or_else(|| {
            let quoted = operand_words.first().copied().unwrap_or_default();
            CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let consumed = if operand_start == 0 {
            used + (synthetic_tokens.len().saturating_sub(operand_len))
        } else {
            operand_start + used
        };
        return Ok(Some((map_operator(operator, operand), consumed)));
    }

    if let Some((value, used)) = parse_value_expr_words(tokens) {
        if tokens.get(used) == Some(&"or")
            && let Some(next) = tokens.get(used + 1)
            && matches!(*next, "less" | "fewer" | "greater" | "more")
        {
            let kind = if matches!(*next, "less" | "fewer") {
                "lte"
            } else {
                "gte"
            };
            return Ok(Some((to_comparison(kind, value), used + 2)));
        }
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

    Ok(None)
}
