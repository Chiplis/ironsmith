use winnow::combinator::alt;
use winnow::prelude::*;

use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;

use super::trim_shape_edges;

fn contains_counter_on_each(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["counter", "on", "each"]),
            primitives::phrase(&["counters", "on", "each"]),
        ))
        .void()
    })
    .is_some()
}

fn has_static_counter_placement_head(tokens: &[OwnedLexToken]) -> bool {
    let body = primitives::parse_prefix(tokens, primitives::kw("put"))
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    let Some((on_index, _, target)) = primitives::find_prefix(body, || primitives::kw("on")) else {
        return false;
    };
    !trim_shape_edges(target).is_empty()
        && !body[..on_index]
            .iter()
            .any(|token| token.is_any_word(&["and", "or", "then"]))
        && super::parse_counter_descriptor_shape(&body[..on_index]).is_some()
}

pub(super) fn has_repeated_counter_on_each(tokens: &[OwnedLexToken]) -> bool {
    let mut remaining = tokens;
    let mut found = false;
    while let Some((_, (), tail)) = primitives::find_prefix(remaining, || {
        alt((
            primitives::phrase(&["counter", "on", "each"]),
            primitives::phrase(&["counters", "on", "each"]),
        ))
        .void()
    }) {
        if found {
            return true;
        }
        found = true;
        remaining = tail;
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeatedCounterPlacementShape<'a> {
    pub first_tokens: &'a [OwnedLexToken],
    pub second_tokens: &'a [OwnedLexToken],
}

/// Splits peer counter placements, including distinct single recipients,
/// while leaving conjunctions inside either object filter alone.
pub fn parse_repeated_counter_placement_shape(
    tokens: &[OwnedLexToken],
) -> Option<RepeatedCounterPlacementShape<'_>> {
    primitives::parse_prefix(tokens, primitives::kw("put"))?;
    let mut search_start = 0usize;
    loop {
        let (relative_index, (), _) =
            primitives::find_prefix(&tokens[search_start..], || primitives::kw("and").void())?;
        let separator_index = search_start + relative_index;
        let first_tokens = trim_shape_edges(&tokens[..separator_index]);
        let second_tokens = trim_shape_edges(&tokens[separator_index + 1..]);
        if (contains_counter_on_each(first_tokens) && contains_counter_on_each(second_tokens))
            || (has_static_counter_placement_head(first_tokens)
                && has_static_counter_placement_head(second_tokens))
        {
            return Some(RepeatedCounterPlacementShape {
                first_tokens,
                second_tokens,
            });
        }
        search_start = separator_index + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_counter_split_keeps_conjunctions_inside_the_first_filter() {
        let tokens = crate::lexer::lex_line("Put a +1/+1 counter on each artifact and creature you control and a shield counter on target creature.", 0).unwrap();
        let shape = parse_repeated_counter_placement_shape(&tokens).unwrap();
        assert_eq!(
            shape
                .first_tokens
                .iter()
                .filter(|token| token.is_word("and"))
                .count(),
            1
        );
        assert!(shape.second_tokens.first().unwrap().is_word("a"));
        let shared_target = crate::lexer::lex_line(
            "Put a +1/+1 counter and a shield counter on target creature.",
            0,
        )
        .unwrap();
        assert!(parse_repeated_counter_placement_shape(&shared_target).is_none());
    }
}
