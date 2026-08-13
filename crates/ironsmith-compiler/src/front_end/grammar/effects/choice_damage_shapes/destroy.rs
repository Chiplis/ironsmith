use winnow::combinator::alt;
use winnow::prelude::*;

use crate::grammar::primitives;
use crate::front_end::lexer::{OwnedLexToken, TokenWordView};

use super::common::{
    first_choice_damage_word_is, has_all_or_each_at, has_choice_damage_condition_boundary,
    has_if_or_unless_shape, is_up_to_one_target_shape,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DestroyMultiTargetShape {
    pub(crate) target_start_word: usize,
    pub(crate) repeated_target_words: bool,
    pub(crate) has_followup_tail: bool,
}

pub(crate) fn up_to_one_target_word_starts(words: &[&str]) -> Vec<usize> {
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
    let Some(and_all) = words.windows(2).position(|window| window == ["and", "all"]) else {
        return false;
    };
    let attached_tail = &words[and_all + 2..];
    let Some(attached_to) = attached_tail
        .windows(2)
        .position(|window| window == ["attached", "to"])
    else {
        return false;
    };
    matches!(
        &attached_tail[attached_to + 2..],
        ["it"] | ["them"] | ["that", _]
    )
}

pub(crate) fn parse_destroy_multi_target_shape(
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
    if words
        .windows(3)
        .any(|window| window == ["and", "all", "other"])
    {
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
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn identifies_destroy_fanout_and_repeated_target_starts() {
        let tokens = lex_line(
            "Destroy up to one target artifact and up to one target enchantment.",
            0,
        )
        .unwrap();
        let shape = parse_destroy_multi_target_shape(&tokens).unwrap();
        assert!(shape.repeated_target_words);
        assert_eq!(
            up_to_one_target_word_starts(&TokenWordView::new(&tokens).to_word_refs()),
            [1, 7]
        );
    }

    #[test]
    fn leaves_target_and_attached_object_sets_for_the_linked_destroy_parser() {
        let tokens = lex_line(
            "Destroy target creature with flying and all Equipment attached to that creature.",
            0,
        )
        .unwrap();

        assert!(parse_destroy_multi_target_shape(&tokens).is_none());
    }
}
