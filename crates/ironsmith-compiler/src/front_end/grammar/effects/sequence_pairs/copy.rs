use std::ops::Range;

use winnow::prelude::*;

use crate::front_end::lexer::{LexStream, OwnedLexToken};

use super::{
    contains_sequence_phrase, contains_sequence_word, matches_complete_content_sequence,
    seek_sequence_phrase, sequence_any_phrase, starts_sequence,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackObjectReferenceShape {
    Source,
    PreviousChosen,
    Triggering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyCandidateKind {
    Object,
    Player,
    PlayerOrPermanent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyCandidateShape {
    pub(crate) candidate: Range<usize>,
    pub(crate) kind: CopyCandidateKind,
    pub(crate) exclude_current_targets: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CopyForEachLayout {
    CopyThenForEach {
        subject: Range<usize>,
        target: Range<usize>,
        candidate: Range<usize>,
    },
    ForEachThenPutCopy {
        target: Range<usize>,
        candidate: Range<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyForEachShape {
    pub(crate) wrap_if_result: bool,
    pub(crate) layout: CopyForEachLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaggedCopyRetargetShape {
    pub(crate) wrap_if_result: bool,
    pub(crate) copy_target: Range<usize>,
}

const COULD_TARGET: &[&[&str]] = &[
    &["that", "spell", "could", "target"],
    &["that", "ability", "could", "target"],
    &["that", "spell", "or", "ability", "could", "target"],
    &["the", "spell", "could", "target"],
    &["the", "ability", "could", "target"],
    &["it", "could", "target"],
];

fn candidate_suffix_start(tokens: &[OwnedLexToken]) -> usize {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut previous_start = 0usize;
    let mut previous_was_that = false;
    while !input.is_empty() {
        let current_start = initial_len.saturating_sub(input.len());
        let mut probe = input.clone();
        if sequence_any_phrase(COULD_TARGET)
            .parse_next(&mut probe)
            .is_ok()
        {
            return if previous_was_that {
                previous_start
            } else {
                current_start
            };
        }
        let before_word = initial_len.saturating_sub(input.len());
        let Ok(token) = super::next_word(&mut input) else {
            break;
        };
        previous_start = before_word;
        previous_was_that = token.is_word("that");
    }
    tokens.len()
}

pub(crate) fn parse_copy_candidate_shape(tokens: &[OwnedLexToken]) -> Option<CopyCandidateShape> {
    let end = candidate_suffix_start(tokens);
    let mut start = 0usize;
    let mut input = LexStream::new(&tokens[..end]);
    let exclude_current_targets = if sequence_any_phrase(&[&["other"], &["another"]])
        .parse_next(&mut input)
        .is_ok()
    {
        start = end.saturating_sub(input.len());
        true
    } else {
        false
    };
    if start >= end {
        return None;
    }
    let candidate = start..end;
    let candidate_tokens = &tokens[candidate.clone()];
    let has_player = contains_sequence_word(candidate_tokens, "player")
        || contains_sequence_word(candidate_tokens, "players");
    let has_permanent = contains_sequence_word(candidate_tokens, "permanent")
        || contains_sequence_word(candidate_tokens, "permanents");
    let has_creature = contains_sequence_word(candidate_tokens, "creature");
    let kind = if has_player && has_permanent {
        CopyCandidateKind::PlayerOrPermanent
    } else if has_player && !has_creature {
        CopyCandidateKind::Player
    } else {
        CopyCandidateKind::Object
    };
    Some(CopyCandidateShape {
        candidate,
        kind,
        exclude_current_targets,
    })
}

fn previous_chose_stack_object(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(tokens);
    while let Ok(token) = super::next_word(&mut input) {
        if !token.is_word("target") {
            continue;
        }
        let mut probe = input.clone();
        let mut remaining = 5usize;
        while remaining > 0 {
            let Ok(tail) = super::next_word(&mut probe) else {
                break;
            };
            if tail.is_word("spell") || tail.is_word("ability") {
                return true;
            }
            remaining -= 1;
        }
    }
    false
}

pub(crate) fn parse_stack_object_reference_shape(
    target: &[OwnedLexToken],
    previous: Option<&[OwnedLexToken]>,
) -> StackObjectReferenceShape {
    if matches_complete_content_sequence(target, &[&["this", "spell"], &["this", "ability"]]) {
        StackObjectReferenceShape::Source
    } else if previous.is_some_and(previous_chose_stack_object) {
        StackObjectReferenceShape::PreviousChosen
    } else {
        StackObjectReferenceShape::Triggering
    }
}

pub(crate) fn is_tempting_offer_copy_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> bool {
    contains_sequence_phrase(
        first,
        &[&["choose", "target", "instant", "or", "sorcery", "spell"]],
    ) && contains_sequence_phrase(
        second,
        &[&["each", "opponent", "may", "copy", "that", "spell"]],
    ) && contains_sequence_phrase(second, &[&["choose", "new", "targets"]])
        && starts_sequence(third, &[&["you", "copy", "that", "spell"]])
        && contains_sequence_phrase(third, &[&["once", "plus", "an", "additional", "time"]])
        && contains_sequence_phrase(
            third,
            &[&[
                "each", "opponent", "who", "copied", "the", "spell", "this", "way",
            ]],
        )
        && starts_sequence(fourth, &[&["you", "may", "choose", "new", "targets"]])
        && (contains_sequence_word(fourth, "copy") || contains_sequence_word(fourth, "copies"))
}

fn first_phrase_offset(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    seek_sequence_phrase(&mut input, alternatives).ok()
}

fn after_phrase_offset(
    tokens: &[OwnedLexToken],
    start: usize,
    alternatives: &[&'static [&'static str]],
) -> Option<usize> {
    let mut input = LexStream::new(&tokens[start..]);
    sequence_any_phrase(alternatives)
        .parse_next(&mut input)
        .ok()?;
    Some(tokens.len().saturating_sub(input.len()))
}

pub(crate) fn parse_copy_for_each_shape(tokens: &[OwnedLexToken]) -> Option<CopyForEachShape> {
    let wrap_if_result = starts_sequence(tokens, &[&["if", "you", "do"]]);
    let for_each = first_phrase_offset(tokens, &[&["for", "each"]])?;
    let copy = first_phrase_offset(tokens, &[&["copy"], &["copies"]])?;
    if copy < for_each {
        let target_start = after_phrase_offset(tokens, copy, &[&["copy"], &["copies"]])?;
        let candidate_start = after_phrase_offset(tokens, for_each, &[&["for", "each"]])?;
        if target_start > for_each || candidate_start >= tokens.len() {
            return None;
        }
        return Some(CopyForEachShape {
            wrap_if_result,
            layout: CopyForEachLayout::CopyThenForEach {
                subject: 0..copy,
                target: target_start..for_each,
                candidate: candidate_start..tokens.len(),
            },
        });
    }

    let put_copy = first_phrase_offset(tokens, &[&["put", "a", "copy"]])?;
    let candidate_start = after_phrase_offset(tokens, for_each, &[&["for", "each"]])?;
    if candidate_start > put_copy {
        return None;
    }
    let after_copy = after_phrase_offset(tokens, put_copy, &[&["put", "a", "copy"]])?;
    let target_start = if let Some(of_at) = first_phrase_offset(&tokens[after_copy..], &[&["of"]]) {
        after_phrase_offset(tokens, after_copy + of_at, &[&["of"]])?
    } else {
        after_copy
    };
    let target_end = first_phrase_offset(&tokens[target_start..], &[&["onto", "the", "stack"]])
        .map(|relative| target_start + relative)
        .unwrap_or(tokens.len());
    if target_start >= target_end {
        return None;
    }
    Some(CopyForEachShape {
        wrap_if_result,
        layout: CopyForEachLayout::ForEachThenPutCopy {
            target: target_start..target_end,
            candidate: candidate_start..put_copy,
        },
    })
}

pub(crate) fn each_copy_targets_different_shape(tokens: &[OwnedLexToken]) -> bool {
    contains_sequence_phrase(
        tokens,
        &[&[
            "each",
            "copy",
            "targets",
            "a",
            "different",
            "one",
            "of",
            "those",
        ]],
    )
}

pub(crate) fn parse_tagged_copy_retarget_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<TaggedCopyRetargetShape> {
    let tagged_iteration = contains_sequence_phrase(
        first,
        &[
            &["for", "each", "of", "those"],
            &["for", "each", "of", "them"],
        ],
    ) || (contains_sequence_phrase(first, &[&["for", "each"]])
        && contains_sequence_phrase(first, &[&["chosen", "this", "way"]]));
    if !tagged_iteration
        || (!contains_sequence_word(first, "copy") && !contains_sequence_word(first, "copies"))
        || !starts_sequence(
            second,
            &[
                &["the", "copy", "targets", "that"],
                &["the", "copy", "targets", "the", "chosen"],
            ],
        )
    {
        return None;
    }
    let copy_at = first_phrase_offset(first, &[&["copy"], &["copies"]])?;
    let copy_target = after_phrase_offset(first, copy_at, &[&["copy"], &["copies"]])?..first.len();
    Some(TaggedCopyRetargetShape {
        wrap_if_result: starts_sequence(first, &[&["if", "you", "do"]]),
        copy_target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_copy_candidate_and_for_each_layout() {
        let candidate =
            parse_copy_candidate_shape(&lex("another player or permanent that spell could target"))
                .unwrap();
        assert_eq!(candidate.kind, CopyCandidateKind::PlayerOrPermanent);
        assert!(candidate.exclude_current_targets);

        let shape = parse_copy_for_each_shape(&lex(
            "You copy that spell for each other player it could target",
        ))
        .unwrap();
        assert!(matches!(
            shape.layout,
            CopyForEachLayout::CopyThenForEach { .. }
        ));
    }
}
