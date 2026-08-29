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

/// Splits peer `put ... counter on each ... and ... counter on each ...`
/// placements while leaving conjunctions inside either object filter alone.
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
        if contains_counter_on_each(first_tokens) && contains_counter_on_each(second_tokens) {
            return Some(RepeatedCounterPlacementShape {
                first_tokens,
                second_tokens,
            });
        }
        search_start = separator_index + 1;
    }
}
