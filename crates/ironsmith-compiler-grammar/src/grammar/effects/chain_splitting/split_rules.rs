use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::recognition::{
    comma_boundary_facts, has_extended_effect_head_tokens, preserve_and_reason,
    starts_with_each_player_or_opponent, starts_with_player_may_tokens, then_followup_facts,
};
use super::verbs::find_chain_verb_tokens;

pub fn split_effect_chain_on_and_tokens(
    tokens: &[OwnedLexToken],
    extended: bool,
) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut input = LexStream::new(tokens);
    let mut inside_quotes = false;
    while !input.is_empty() {
        let idx = tokens.len().saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            break;
        };
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        if !is_word(token, "and") {
            continue;
        }
        let current = trim_lexed_commas(tokens.get(start..idx).unwrap_or_default());
        let remaining = trim_lexed_commas(tokens.get(idx + 1..).unwrap_or_default());
        if preserve_and_reason(current, remaining, extended).is_some() {
            continue;
        }
        let current_words = crate::lexer::parser_token_word_refs(current);
        let remaining_words = crate::lexer::parser_token_word_refs(remaining);
        if crate::word_primitives::contains_all_words(&current_words, &["damage", "to"])
            && crate::word_primitives::parse_sequence_prefix(&remaining_words, &["each"])
            && crate::word_primitives::parse_sequence_suffix(
                &remaining_words,
                &["that", "player", "controls"],
            )
        {
            continue;
        }
        let remaining_starts_action = find_chain_verb_tokens(remaining).is_some()
            || has_extended_effect_head_tokens(remaining)
            || starts_with_player_may_tokens(remaining)
            || remaining
                .first()
                .is_some_and(|token| is_word(token, "choose"))
            || (current.iter().any(|token| is_word(token, "damage"))
                && remaining
                    .first()
                    .is_some_and(|token| is_word(token, "each")));
        if !remaining_starts_action {
            continue;
        }
        if !current.is_empty() {
            segments.push(current);
        }
        start = idx + 1;
    }
    let tail = trim_lexed_commas(tokens.get(start..).unwrap_or_default());
    if !tail.is_empty() {
        segments.push(tail);
    }
    segments
}

pub fn split_segments_on_comma_then_tokens(
    segments: Vec<&[OwnedLexToken]>,
    mut is_ability_head: impl FnMut(&[OwnedLexToken]) -> bool,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        // A source sentence may contain more than one authored `, then`
        // boundary. Keep splitting the unconsumed tail so an n-ary ordered
        // chain does not leave its final actions inside a prefix-tolerant
        // parser for the second arm.
        let mut remaining = segment;
        while let Some(split) = find_then_split(remaining, &mut is_ability_head) {
            let first = trim_lexed_commas(remaining.get(..split.separator_idx).unwrap_or_default());
            let second = trim_lexed_commas(remaining.get(split.then_idx + 1..).unwrap_or_default());
            if !first.is_empty() {
                result.push(first);
            }
            if second.is_empty() || second.len() >= remaining.len() {
                remaining = &[];
                break;
            }
            remaining = second;
        }
        if !remaining.is_empty() {
            result.push(remaining);
        }
    }
    result
}

/// Return whether the generic chain grammar accepts an authored `, then`
/// boundary. A bare same-sentence `then`, sentence-leading `Then`, and quoted
/// text remain distinct surfaces.
pub fn has_explicit_comma_then_boundary_tokens(
    tokens: &[OwnedLexToken],
    mut is_ability_head: impl FnMut(&[OwnedLexToken]) -> bool,
) -> bool {
    find_then_split(tokens, &mut is_ability_head).is_some_and(|split| split.explicit_comma_then)
}

#[derive(Debug, Clone, Copy)]
struct ThenSplit {
    separator_idx: usize,
    then_idx: usize,
    explicit_comma_then: bool,
}

#[cfg(test)]
#[path = "split_rules_inline_tests.rs"]
mod tests;

#[path = "split_rules/core.rs"]
mod core_programs;
use core_programs::{find_then_split, is_word};
#[path = "split_rules/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::{
    has_authored_comma_then_surface_tokens, split_segments_on_comma_effect_head_tokens,
};
