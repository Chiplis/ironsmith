use super::grammar::primitives::{self as grammar, split_lexed_slices_on_or};
use super::grammar::values::parse_value_comparison_tokens;
use super::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::object_filters::parse_object_filter;
use super::token_primitives::{
    parse_simple_restriction_duration_prefix, parse_simple_restriction_duration_suffix,
    slice_starts_with as word_slice_starts_with,
};
use super::util::trim_commas;
use crate::cards::builders::CardTextError;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SearchLibraryManaConstraint {
    Equal(u32),
    LessThanOrEqual(u32),
    GreaterThanOrEqual(u32),
    OneOf(Vec<u32>),
}

pub(crate) fn word_slice_starts_with_any(words: &[&str], prefixes: &[&[&str]]) -> bool {
    prefixes
        .iter()
        .any(|prefix| word_slice_starts_with(words, prefix))
}

pub(crate) fn word_slice_mentions_nth_from_top(words: &[&str]) -> bool {
    let mut idx = 0usize;
    while idx + 3 < words.len() {
        if words[idx + 1] == "from" && words[idx + 2] == "the" && words[idx + 3] == "top" {
            return true;
        }
        idx += 1;
    }
    false
}

fn token_slice_contains_word(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    tokens
        .iter()
        .enumerate()
        .any(|(idx, _)| grammar::parse_prefix(&tokens[idx..], grammar::kw(expected)).is_some())
}

fn token_slice_contains_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    grammar::find_prefix(tokens, || grammar::phrase(phrase)).is_some()
}

fn find_phrase_token_bounds(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<(usize, usize)> {
    if phrase.is_empty() {
        return None;
    }
    let (idx, _, rest) = grammar::find_prefix(tokens, || grammar::phrase(phrase))?;
    Some((idx, tokens.len() - rest.len()))
}

fn token_words<'a>(tokens: &'a [OwnedLexToken]) -> Vec<&'a str> {
    token_word_refs(tokens)
}

fn is_source_reference_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    [
        "this",
        "thiss",
        "source",
        "artifact",
        "creature",
        "permanent",
    ]
    .iter()
    .any(|word| token_slice_contains_word(tokens, word))
}

fn is_as_long_as_you_control_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    token_slice_contains_word(tokens, "you")
        && token_slice_contains_word(tokens, "control")
        && is_source_reference_duration_tokens(tokens)
}

fn is_source_remains_tapped_duration_tokens(tokens: &[OwnedLexToken]) -> bool {
    token_slice_contains_phrase(tokens, &["for", "as", "long", "as"])
        && token_slice_contains_word(tokens, "remains")
        && token_slice_contains_word(tokens, "tapped")
        && is_source_reference_duration_tokens(tokens)
}

fn remove_this_turn_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut cleaned = Vec::new();
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].is_word("this")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("turn"))
        {
            idx += 2;
            continue;
        }
        cleaned.push(tokens[idx].clone());
        idx += 1;
    }
    cleaned
}

pub(crate) fn zone_slice_contains(zones: &[Zone], expected: Zone) -> bool {
    zones.iter().any(|zone| *zone == expected)
}

fn card_type_slice_contains(card_types: &[CardType], expected: CardType) -> bool {
    card_types.iter().any(|card_type| *card_type == expected)
}

pub(crate) fn parse_search_library_disjunction_filter(
    filter_tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let segments = split_lexed_slices_on_or(filter_tokens);
    if segments.len() < 2 {
        return None;
    }

    let mut branches = Vec::new();
    for segment in segments {
        let trimmed = trim_commas(segment);
        if trimmed.is_empty() {
            return None;
        }
        let Ok(filter) = parse_object_filter(&trimmed, false) else {
            return None;
        };
        branches.push(filter);
    }

    if branches.len() < 2 {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Some(filter)
}

pub(crate) fn parse_restriction_duration_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    use crate::effect::Until;

    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some((duration, rest)) = parse_simple_restriction_duration_prefix(tokens) {
        return Ok(Some((duration, trim_lexed_commas(rest).to_vec())));
    }

    if token_words(tokens).len() < 2 {
        return Ok(None);
    }

    if grammar::parse_prefix(tokens, grammar::phrase(&["for", "as", "long", "as"])).is_some() {
        if !is_as_long_as_you_control_duration_tokens(tokens) {
            return Ok(None);
        }
        let Some((_before, after)) =
            grammar::split_lexed_once_on_delimiter(tokens, TokenKind::Comma)
        else {
            return Err(CardTextError::ParseError(
                "missing comma after duration prefix".to_string(),
            ));
        };
        let remainder = trim_lexed_commas(after).to_vec();
        return Ok(Some((Until::YouStopControllingThis, remainder)));
    }

    if let Some((rest, duration)) = parse_simple_restriction_duration_suffix(tokens) {
        let remainder = trim_lexed_commas(rest).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((duration, remainder)));
        }
    }

    if let Some((token_idx, _)) = find_phrase_token_bounds(tokens, &["for", "as", "long", "as"]) {
        let suffix_tokens = &tokens[token_idx..];
        if is_source_remains_tapped_duration_tokens(suffix_tokens) {
            let remainder = trim_lexed_commas(&tokens[..token_idx]).to_vec();
            if !remainder.is_empty() {
                return Ok(Some((Until::ThisLeavesTheBattlefield, remainder)));
            }
        }
    }

    let cleaned_tokens = remove_this_turn_tokens(tokens);
    if let Some((rest, duration)) = parse_simple_restriction_duration_suffix(&cleaned_tokens) {
        let remainder = trim_lexed_commas(rest).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((duration, remainder)));
        }
    }

    Ok(None)
}

pub(crate) fn extract_search_library_mana_constraint(
    filter_tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, SearchLibraryManaConstraint)> {
    let (clause_token_start, clause_token_end) =
        find_phrase_token_bounds(filter_tokens, &["with", "mana", "cost"])
            .or_else(|| find_phrase_token_bounds(filter_tokens, &["with", "mana", "value"]))?;
    let base_filter_tokens = trim_commas(&filter_tokens[..clause_token_start]);
    if base_filter_tokens.is_empty() {
        return None;
    }

    let clause_tokens = trim_lexed_commas(&filter_tokens[clause_token_end..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let parse_single_u32_clause = |tokens: &[OwnedLexToken]| -> Option<u32> {
        let [token] = tokens else {
            return None;
        };
        token.parser_text().parse::<u32>().ok()
    };
    let constraint = if let Some(value) = parse_single_u32_clause(clause_tokens) {
        SearchLibraryManaConstraint::Equal(value)
    } else if let Some((operator, value_tokens)) = parse_value_comparison_tokens(clause_tokens) {
        let value = parse_single_u32_clause(value_tokens)?;
        match operator {
            crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                SearchLibraryManaConstraint::LessThanOrEqual(value)
            }
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                SearchLibraryManaConstraint::GreaterThanOrEqual(value)
            }
            _ => return None,
        }
    } else {
        let [left, middle, right] = clause_tokens else {
            return None;
        };
        if !middle.is_word("or") {
            return None;
        }
        SearchLibraryManaConstraint::OneOf(vec![
            left.parser_text().parse::<u32>().ok()?,
            right.parser_text().parse::<u32>().ok()?,
        ])
    };

    Some((base_filter_tokens, constraint))
}

pub(crate) fn apply_search_library_mana_constraint(
    filter: &mut ObjectFilter,
    constraint: SearchLibraryManaConstraint,
) {
    if !filter.any_of.is_empty() {
        for nested in &mut filter.any_of {
            apply_search_library_mana_constraint(nested, constraint.clone());
        }
        return;
    }

    let build_branch = |base: &ObjectFilter, mana_value: crate::filter::Comparison| {
        let mut branch = base.clone();
        branch.has_mana_cost = true;
        branch.no_x_in_cost = true;
        branch.mana_value = Some(mana_value);
        branch
    };

    match constraint {
        SearchLibraryManaConstraint::Equal(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::Equal(value as i32));
        }
        SearchLibraryManaConstraint::LessThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::GreaterThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::GreaterThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::OneOf(values) => {
            let base = filter.clone();
            *filter = ObjectFilter::default();
            filter.any_of = values
                .into_iter()
                .map(|value| build_branch(&base, crate::filter::Comparison::Equal(value as i32)))
                .collect();
        }
    }
}

pub(crate) fn split_search_same_name_reference_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let (start_token_idx, end_token_idx) =
        find_phrase_token_bounds(tokens, &["with", "the", "same", "name", "as"])
            .or_else(|| find_phrase_token_bounds(tokens, &["with", "same", "name", "as"]))?;
    let base_filter_tokens = trim_commas(&tokens[..start_token_idx]);
    let reference_tokens = trim_commas(&tokens[end_token_idx..]);
    Some((base_filter_tokens, reference_tokens))
}

pub(crate) fn is_same_name_that_reference_words(words: &[&str]) -> bool {
    matches!(
        words,
        ["that", "card"]
            | ["that", "cards"]
            | ["that", "creature"]
            | ["that", "creatures"]
            | ["that", "artifact"]
            | ["that", "artifacts"]
            | ["that", "enchantment"]
            | ["that", "enchantments"]
            | ["that", "land"]
            | ["that", "lands"]
            | ["that", "permanent"]
            | ["that", "permanents"]
            | ["that", "spell"]
            | ["that", "spells"]
            | ["that", "object"]
            | ["that", "objects"]
            | ["those", "cards"]
            | ["those", "creatures"]
            | ["those", "artifacts"]
            | ["those", "enchantments"]
            | ["those", "lands"]
            | ["those", "permanents"]
            | ["those", "spells"]
            | ["those", "objects"]
    )
}

pub(crate) fn normalize_search_library_filter(filter: &mut ObjectFilter) {
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
                | Subtype::Desert
        )
    }) && !card_type_slice_contains(&filter.card_types, CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    for nested in &mut filter.any_of {
        normalize_search_library_filter(nested);
    }
}
