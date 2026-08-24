use winnow::combinator::alt;
use winnow::prelude::*;

use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, TokenWordView};

use super::common::{
    first_choice_damage_word_is, has_all_or_each_at, has_choice_damage_condition_boundary,
    has_if_or_unless_shape, is_up_to_one_target_shape,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DestroyMultiTargetShape {
    pub target_start_word: usize,
    pub repeated_target_words: bool,
    pub has_followup_tail: bool,
}

pub fn up_to_one_target_word_starts(words: &[&str]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut offset = 0usize;
    while offset + 4 <= words.len() {
        if is_up_to_one_target_shape(&words[offset..offset + 4]) {
            starts.push(offset);
        }
        offset += 1;
    }
    starts
}

fn has_target_separator(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        alt((primitives::comma().void(), primitives::kw("and").void()))
    })
    .is_some()
}

fn has_target_and_attached_set(words: &[&str]) -> bool {
    let Some(and_all) = crate::word_primitives::parse_sequence_start(words, &["and", "all"]) else {
        return false;
    };
    let attached_tail = &words[and_all + 2..];
    let Some(attached_to) =
        crate::word_primitives::parse_sequence_start(attached_tail, &["attached", "to"])
    else {
        return false;
    };
    let reference = &attached_tail[attached_to + 2..];
    crate::word_primitives::parse_any_sequence_complete(reference, &[&["it"], &["them"]])
        || (reference.len() == 2 && crate::word_primitives::first_is(reference, "that"))
}

pub fn parse_destroy_multi_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyMultiTargetShape> {
    let words = TokenWordView::new(tokens).to_word_refs();
    if !first_choice_damage_word_is(&words, "destroy")
        || has_all_or_each_at(&words, 1)
        || has_if_or_unless_shape(&words)
        || words.len() <= 1
    {
        return None;
    }
    // "target X and all other Ys with the same name ..." is the same-name
    // fanout family (one target + a mass action), not a multi-target list.
    if crate::word_primitives::sequence_occurs(&words, &["and", "all", "other"]) {
        return None;
    }
    // A declared target plus the complete set attached to that declaration
    // is one linked destroy program, not a list of independent targets.
    // Leave it for the typed target-and-attached destroy parser, which binds
    // both actions through a stable object tag.
    if has_target_and_attached_set(&words) {
        return None;
    }
    let target_words = &words[1..];
    let repeated_up_to_one = up_to_one_target_word_starts(target_words).len() >= 2;
    if !has_target_separator(tokens) && !repeated_up_to_one {
        return None;
    }
    let mut target_count = 0usize;
    let mut offset = 0usize;
    while offset < target_words.len() {
        if target_words.get(offset).copied() == Some("target") {
            target_count += 1;
        }
        offset += 1;
    }
    Some(DestroyMultiTargetShape {
        target_start_word: 1,
        repeated_target_words: target_count > 1,
        has_followup_tail: has_choice_damage_condition_boundary(target_words),
    })
}

#[cfg(test)]
#[path = "destroy_inline_tests.rs"]
mod tests;
