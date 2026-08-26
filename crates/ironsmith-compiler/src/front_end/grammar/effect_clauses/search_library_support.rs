use super::grammar::primitives::{self as grammar, split_lexed_slices_on_or};
use super::grammar::values::parse_value_comparison_tokens;
use super::lexer::{
    OwnedLexToken, TokenKind, find_token_word_sequence_span, token_word_refs, trim_lexed_commas,
};
use super::object_filters::parse_object_filter;
use super::token_primitives::{
    find_window_by, parse_simple_restriction_duration_prefix,
    parse_simple_restriction_duration_suffix,
};
use super::util::{parse_number, trim_commas};
use crate::cards::builders::CardTextError;
use crate::effect::Value;
use crate::keyword_static::parse_value_binding_clause;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchLibraryManaConstraint {
    Equal(u32),
    LessThanOrEqual(u32),
    GreaterThanOrEqual(u32),
    OneOf(Vec<u32>),
    ExactCost(crate::mana::ManaCost),
    OneOfExactCosts(Vec<crate::mana::ManaCost>),
}

pub fn word_slice_mentions_nth_from_top(words: &[&str]) -> bool {
    find_window_by(words, 4, |window| {
        window[1] == "from" && window[2] == "the" && window[3] == "top"
    })
    .is_some()
}

fn card_type_set_includes(card_types: &[CardType], expected: CardType) -> bool {
    for card_type in card_types {
        if *card_type == expected {
            return true;
        }
    }
    false
}

pub fn parse_search_library_disjunction_filter(
    filter_tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let uses_and_or = filter_tokens.iter().any(|token| token.is_word("and/or"));
    let segments = if uses_and_or {
        let mut segments = Vec::new();
        let mut start = 0usize;
        for (idx, token) in filter_tokens.iter().enumerate() {
            if token.is_word("and/or") {
                segments.push(trim_commas(&filter_tokens[start..idx]));
                start = idx + 1;
            }
        }
        segments.push(trim_commas(&filter_tokens[start..]));
        segments
    } else {
        split_lexed_slices_on_or(filter_tokens)
            .into_iter()
            .map(trim_commas)
            .collect()
    };
    if segments.len() < 2 {
        return None;
    }

    let mut branches = Vec::new();
    for segment in segments {
        let trimmed = trim_commas(&segment);
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
    if uses_and_or {
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    }
    Some(filter)
}

pub fn parse_restriction_duration_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    use crate::effect::Until;

    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some((duration, rest)) = parse_simple_restriction_duration_prefix(tokens) {
        return Ok(Some((duration, trim_lexed_commas(rest).to_vec())));
    }

    if token_word_refs(tokens).len() < 2 {
        return Ok(None);
    }

    if grammar::parse_prefix(tokens, grammar::phrase(&["for", "as", "long", "as"])).is_some() {
        if !matches!(
            super::grammar::leaf::parse_leaf_conditional_duration_kind_tokens(tokens),
            Some(super::grammar::leaf::LeafConditionalDurationKind::YouControlSource)
        ) {
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

    if let Some((token_idx, _)) =
        find_token_word_sequence_span(tokens, &["for", "as", "long", "as"])
    {
        let suffix_tokens = &tokens[token_idx..];
        if matches!(
            super::grammar::leaf::parse_leaf_conditional_duration_kind_tokens(suffix_tokens),
            Some(super::grammar::leaf::LeafConditionalDurationKind::SourceRemainsTapped)
        ) {
            let remainder = trim_lexed_commas(&tokens[..token_idx]).to_vec();
            if !remainder.is_empty() {
                return Ok(Some((Until::SourceUntaps, remainder)));
            }
        }
    }

    let cleaned_tokens = super::grammar::leaf::strip_leaf_this_turn_tokens(tokens);
    if let Some((rest, duration)) = parse_simple_restriction_duration_suffix(&cleaned_tokens) {
        let remainder = trim_lexed_commas(rest).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((duration, remainder)));
        }
    }

    Ok(None)
}

pub fn extract_search_library_mana_constraint(
    filter_tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, SearchLibraryManaConstraint)> {
    let (clause_token_start, clause_token_end) =
        find_token_word_sequence_span(filter_tokens, &["with", "mana", "cost"])
            .or_else(|| find_token_word_sequence_span(filter_tokens, &["with", "mana", "value"]))?;
    let base_filter_tokens = trim_commas(&filter_tokens[..clause_token_start]);
    if base_filter_tokens.is_empty() {
        return None;
    }

    let clause_tokens = trim_lexed_commas(&filter_tokens[clause_token_end..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let parse_single_u32_clause = |tokens: &[OwnedLexToken]| -> Option<u32> {
        if let Some((value, used)) = parse_number(tokens)
            && used == tokens.len()
        {
            return Some(value);
        }
        None
    };
    let parse_exact_mana_cost_clause = |tokens: &[OwnedLexToken]| -> Option<crate::mana::ManaCost> {
        let mana = super::grammar::leaf::parse_leaf_mana_cost_prefix_tokens(tokens)?;
        if mana.consumed != tokens.len() {
            return None;
        }
        Some(mana.cost)
    };
    let constraint = if let Some(cost) = parse_exact_mana_cost_clause(clause_tokens) {
        SearchLibraryManaConstraint::ExactCost(cost)
    } else if let Some(value) = parse_single_u32_clause(clause_tokens) {
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
        let left = std::slice::from_ref(left);
        let right = std::slice::from_ref(right);
        match (
            parse_exact_mana_cost_clause(left),
            parse_exact_mana_cost_clause(right),
        ) {
            (Some(left), Some(right)) => {
                SearchLibraryManaConstraint::OneOfExactCosts(vec![left, right])
            }
            (None, None) => SearchLibraryManaConstraint::OneOf(vec![
                parse_single_u32_clause(left)?,
                parse_single_u32_clause(right)?,
            ]),
            _ => return None,
        }
    };

    Some((base_filter_tokens, constraint))
}

pub fn apply_search_library_mana_constraint(
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
        SearchLibraryManaConstraint::ExactCost(cost) => {
            filter.exact_mana_cost = Some(cost);
        }
        SearchLibraryManaConstraint::OneOfExactCosts(costs) => {
            let base = filter.clone();
            *filter = ObjectFilter::default();
            filter.any_of = costs
                .into_iter()
                .map(|cost| {
                    let mut branch = base.clone();
                    branch.exact_mana_cost = Some(cost);
                    branch
                })
                .collect();
        }
    }
}

pub fn split_search_same_name_reference_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let (start_token_idx, end_token_idx) =
        find_token_word_sequence_span(tokens, &["with", "the", "same", "name", "as"])
            .or_else(|| find_token_word_sequence_span(tokens, &["with", "same", "name", "as"]))?;
    let base_filter_tokens = trim_commas(&tokens[..start_token_idx]);
    let reference_tokens = trim_commas(&tokens[end_token_idx..]);
    Some((base_filter_tokens, reference_tokens))
}

pub fn split_search_different_name_reference_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    const PATTERNS: &[&[&str]] = &[
        &["that", "doesn't", "have", "the", "same", "name", "as"],
        &["that", "doesnt", "have", "the", "same", "name", "as"],
        &["that", "does", "not", "have", "the", "same", "name", "as"],
        &["that", "don't", "have", "the", "same", "name", "as"],
        &["that", "dont", "have", "the", "same", "name", "as"],
        &["that", "do", "not", "have", "the", "same", "name", "as"],
        &["with", "a", "different", "name", "from"],
        &["with", "different", "name", "from"],
    ];

    for pattern in PATTERNS {
        if let Some((start_token_idx, end_token_idx)) =
            find_token_word_sequence_span(tokens, pattern)
        {
            let base_filter_tokens = trim_commas(&tokens[..start_token_idx]);
            let reference_tokens = trim_commas(&tokens[end_token_idx..]);
            return Some((base_filter_tokens, reference_tokens));
        }
    }

    None
}

pub fn normalize_search_library_filter(filter: &mut ObjectFilter) {
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
    }) && !card_type_set_includes(&filter.card_types, CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    for nested in &mut filter.any_of {
        normalize_search_library_filter(nested);
    }
}

pub fn split_search_library_count_value_clause_lexed(
    filter_tokens: &[OwnedLexToken],
) -> Result<Option<(Vec<OwnedLexToken>, Value)>, CardTextError> {
    let Some((where_idx, _, _)) =
        grammar::find_prefix(filter_tokens, || grammar::phrase(&["where", "x", "is"]))
    else {
        return Ok(None);
    };

    let count_value_tokens = trim_lexed_commas(&filter_tokens[where_idx..]).to_vec();
    let Some(count_value) = parse_value_binding_clause(&count_value_tokens).or_else(|| {
        super::grammar::values::parse_players_who_control_more_than_you_value_lexed(
            count_value_tokens.as_slice(),
        )
    }) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library count clause (clause: '{}')",
            token_word_refs(&count_value_tokens).join(" ")
        )));
    };

    let base_filter_tokens = trim_commas(&filter_tokens[..where_idx]).to_vec();
    if base_filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing search library filter before where-x clause".to_string(),
        ));
    }

    Ok(Some((base_filter_tokens, count_value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn generic_where_x_search_count_keeps_dynamic_subtraction() {
        let tokens = lex_line(
            "basic land cards, where X is five minus the number of lands they control",
            0,
        )
        .expect("search count clause should lex");
        let (filter, value) = split_search_library_count_value_clause_lexed(&tokens)
            .expect("search count clause should parse")
            .expect("where-X suffix should be recognized");

        assert_eq!(
            TokenWordView::new(&filter).to_word_refs(),
            vec!["basic", "land", "cards"]
        );
        assert!(matches!(value.unhinted(), Value::Add(_, _)), "{value:#?}");
    }

    #[test]
    fn battlefield_creature_count_does_not_invent_a_card_noun() {
        let tokens = lex_line(
            "basic land cards, where X is the number of tapped creatures you control",
            0,
        )
        .expect("search count clause should lex");
        let (_, value) = split_search_library_count_value_clause_lexed(&tokens)
            .expect("search count clause should parse")
            .expect("where-X suffix should be recognized");
        let Value::Count(filter) = value.unhinted() else {
            panic!("expected typed count, got {value:#?}");
        };

        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(filter.zone, Some(crate::Zone::Battlefield));
        assert_eq!(filter.controller, Some(crate::PlayerFilter::You));
        assert!(filter.tapped);
        assert!(!filter.has_explicit_card_noun(), "{filter:#?}");
    }
}
