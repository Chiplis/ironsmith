use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;

use crate::cards::builders::CardTextError;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::common::{finish_text_parse, text_phrase_words};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafDurationPhrase {
    ThisTurn,
    UntilEndOfTurn,
    UntilEndOfCombat,
    UntilYourNextTurn,
    UntilYourNextTurnEnd,
    UntilYourNextUpkeep,
    ControllersNextUntapStep,
    Forever,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafTurnDurationPhrase {
    ThisTurn,
    UntilEndOfTurn,
    UntilYourNextTurn,
    UntilYourNextTurnEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafDurationPrefix<'a, T> {
    pub(crate) duration: T,
    pub(crate) rest: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafDurationSuffix<'a, T> {
    pub(crate) rest: &'a [OwnedLexToken],
    pub(crate) duration: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafDurationWordSpan {
    pub(crate) duration: LeafDurationPhrase,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafConditionalDurationKind {
    YouControlSource,
    SourceRemainsTapped,
}

const LEAF_DURATION_PHRASE_VALUES: &[(&[&str], LeafDurationPhrase)] = &[
    (
        &["until", "the", "end", "of", "your", "next", "turn"],
        LeafDurationPhrase::UntilYourNextTurnEnd,
    ),
    (
        &["until", "end", "of", "your", "next", "turn"],
        LeafDurationPhrase::UntilYourNextTurnEnd,
    ),
    (
        &["until", "your", "next", "end", "step"],
        LeafDurationPhrase::UntilYourNextTurnEnd,
    ),
    (
        &["until", "your", "next", "turn"],
        LeafDurationPhrase::UntilYourNextTurn,
    ),
    (
        &["until", "your", "next", "upkeep"],
        LeafDurationPhrase::UntilYourNextUpkeep,
    ),
    (
        &["until", "your", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["until", "the", "end", "of", "combat"],
        LeafDurationPhrase::UntilEndOfCombat,
    ),
    (
        &["until", "end", "of", "combat"],
        LeafDurationPhrase::UntilEndOfCombat,
    ),
    (
        &["until", "the", "end", "of", "turn"],
        LeafDurationPhrase::UntilEndOfTurn,
    ),
    (
        &["until", "end", "of", "turn"],
        LeafDurationPhrase::UntilEndOfTurn,
    ),
    (&["this", "turn"], LeafDurationPhrase::ThisTurn),
    (
        &["during", "your", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["during", "its", "controller", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["during", "its", "controller's", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["during", "its", "controllers", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["during", "their", "controller", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["during", "their", "controller's", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["during", "their", "controllers", "next", "untap", "step"],
        LeafDurationPhrase::ControllersNextUntapStep,
    ),
    (
        &["for", "the", "rest", "of", "the", "game"],
        LeafDurationPhrase::Forever,
    ),
];

pub(crate) fn parse_leaf_duration_phrase(input: &mut &str) -> WResult<LeafDurationPhrase> {
    parse_leaf_duration_phrase_words
        .context(StrContext::Label("duration phrase"))
        .context(StrContext::Expected(StrContextValue::Description(
            "turn, combat, upkeep, untap-step, or game duration phrase",
        )))
        .parse_next(input)
}

pub(crate) fn parse_leaf_duration_phrase_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafDurationPhrase> {
    for (words, value) in LEAF_DURATION_PHRASE_VALUES {
        let mut probe = input.clone();
        if primitives::phrase(words).parse_next(&mut probe).is_ok() {
            *input = probe;
            return Ok(*value);
        }
    }

    Err(primitives::backtrack_err(
        "duration phrase",
        "turn, combat, upkeep, untap-step, or game duration phrase",
    ))
}

pub(crate) fn parse_leaf_turn_duration_phrase_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafTurnDurationPhrase> {
    let checkpoint = input.checkpoint();
    let parsed = parse_leaf_duration_phrase_lexed.parse_next(input)?;
    if let Some(turn_duration) = leaf_turn_duration_from_duration(parsed) {
        return Ok(turn_duration);
    }
    input.reset(&checkpoint);
    Err(primitives::backtrack_err(
        "turn duration phrase",
        "turn duration phrase",
    ))
}

pub(crate) fn parse_leaf_turn_duration_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LeafDurationPrefix<'a, LeafTurnDurationPhrase>> {
    let (duration, rest) = primitives::parse_prefix(tokens, parse_leaf_turn_duration_phrase_lexed)?;
    Some(LeafDurationPrefix { duration, rest })
}

pub(crate) fn parse_leaf_turn_duration_suffix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LeafDurationSuffix<'a, LeafTurnDurationPhrase>> {
    parse_leaf_duration_suffix(tokens, parse_leaf_turn_duration_phrase_lexed)
}

pub(crate) fn parse_leaf_restriction_duration_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LeafDurationPrefix<'a, LeafDurationPhrase>> {
    let (duration, rest) = primitives::parse_prefix(tokens, parse_leaf_duration_phrase_lexed)?;
    Some(LeafDurationPrefix { duration, rest })
}

pub(crate) fn parse_leaf_restriction_duration_suffix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LeafDurationSuffix<'a, LeafDurationPhrase>> {
    parse_leaf_duration_suffix(tokens, parse_leaf_duration_phrase_lexed)
}

pub(crate) fn parse_leaf_duration_prefix_words(words: &[&str]) -> Option<LeafDurationWordSpan> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let duration = parse_leaf_duration_phrase_word_slice
        .parse_next(&mut input)
        .ok()?;
    Some(LeafDurationWordSpan {
        duration,
        start: 0,
        end: words.len().checked_sub(input.len())?,
    })
}

pub(crate) fn find_leaf_duration_words(words: &[&str]) -> Option<LeafDurationWordSpan> {
    for start in 0..words.len() {
        let Some(parsed) = parse_leaf_duration_prefix_words(&words[start..]) else {
            continue;
        };
        return Some(LeafDurationWordSpan {
            duration: parsed.duration,
            start,
            end: start + parsed.end,
        });
    }
    None
}

pub(crate) fn find_leaf_canonical_until_end_of_turn_words(
    words: &[&str],
) -> Option<LeafDurationWordSpan> {
    for start in 0..words.len() {
        let Some(parsed) = parse_leaf_duration_prefix_words(&words[start..]) else {
            continue;
        };
        if parsed.duration == LeafDurationPhrase::UntilEndOfTurn && parsed.end == 4 {
            return Some(LeafDurationWordSpan {
                duration: parsed.duration,
                start,
                end: start + parsed.end,
            });
        }
    }
    None
}

fn has_lexed_word(tokens: &[OwnedLexToken], word: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(word)).is_some()
}

fn has_source_reference_word(tokens: &[OwnedLexToken]) -> bool {
    [
        "this",
        "thiss",
        "source",
        "artifact",
        "creature",
        "permanent",
    ]
    .into_iter()
    .any(|word| has_lexed_word(tokens, word))
}

pub(crate) fn parse_leaf_conditional_duration_kind_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafConditionalDurationKind> {
    primitives::find_prefix(tokens, || primitives::phrase(&["for", "as", "long", "as"]))?;
    if has_lexed_word(tokens, "remains")
        && has_lexed_word(tokens, "tapped")
        && has_source_reference_word(tokens)
    {
        return Some(LeafConditionalDurationKind::SourceRemainsTapped);
    }
    if has_lexed_word(tokens, "you")
        && has_lexed_word(tokens, "control")
        && has_source_reference_word(tokens)
    {
        return Some(LeafConditionalDurationKind::YouControlSource);
    }
    None
}

pub(crate) fn strip_leaf_this_turn_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut cleaned = Vec::with_capacity(tokens.len());
    let mut index = 0usize;
    while index < tokens.len() {
        if let Some((_matched, rest)) =
            primitives::parse_prefix(&tokens[index..], primitives::phrase(&["this", "turn"]))
        {
            index = tokens.len().saturating_sub(rest.len());
            continue;
        }
        cleaned.push(tokens[index].clone());
        index += 1;
    }
    cleaned
}

pub(crate) fn parse_duration_phrase_complete(
    raw: &str,
) -> Result<LeafDurationPhrase, CardTextError> {
    finish_text_parse(raw, parse_leaf_duration_phrase, "leaf-duration")
}

fn leaf_turn_duration_from_duration(
    duration: LeafDurationPhrase,
) -> Option<LeafTurnDurationPhrase> {
    match duration {
        LeafDurationPhrase::ThisTurn => Some(LeafTurnDurationPhrase::ThisTurn),
        LeafDurationPhrase::UntilEndOfTurn => Some(LeafTurnDurationPhrase::UntilEndOfTurn),
        LeafDurationPhrase::UntilYourNextTurn => Some(LeafTurnDurationPhrase::UntilYourNextTurn),
        LeafDurationPhrase::UntilYourNextTurnEnd => {
            Some(LeafTurnDurationPhrase::UntilYourNextTurnEnd)
        }
        LeafDurationPhrase::UntilEndOfCombat
        | LeafDurationPhrase::UntilYourNextUpkeep
        | LeafDurationPhrase::ControllersNextUntapStep
        | LeafDurationPhrase::Forever => None,
    }
}

fn parse_leaf_duration_suffix<'a, O>(
    tokens: &'a [OwnedLexToken],
    mut parser: impl Parser<LexStream<'a>, O, winnow::error::ErrMode<winnow::error::ContextError>>,
) -> Option<LeafDurationSuffix<'a, O>> {
    for suffix_start in 0..tokens.len() {
        let mut input = LexStream::new(&tokens[suffix_start..]);
        let Ok(duration) = parser.parse_next(&mut input) else {
            continue;
        };
        if primitives::sentence_end().parse_next(&mut input).is_ok() {
            return Some(LeafDurationSuffix {
                rest: &tokens[..suffix_start],
                duration,
            });
        }
    }
    None
}

fn parse_leaf_duration_phrase_words(input: &mut &str) -> WResult<LeafDurationPhrase> {
    for (words, value) in LEAF_DURATION_PHRASE_VALUES {
        let checkpoint = *input;
        if text_phrase_words(words).parse_next(input).is_ok() {
            return Ok(*value);
        }
        *input = checkpoint;
    }

    Err(primitives::backtrack_err(
        "duration phrase",
        "turn, combat, upkeep, untap-step, or game duration phrase",
    ))
}

fn parse_leaf_duration_phrase_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<LeafDurationPhrase> {
    for (words, value) in LEAF_DURATION_PHRASE_VALUES {
        let mut probe = *input;
        let mut matched = true;
        for word in *words {
            if primitives::word_slice_exact(word)
                .parse_next(&mut probe)
                .is_err()
            {
                matched = false;
                break;
            }
        }
        if matched {
            *input = probe;
            return Ok(*value);
        }
    }

    Err(primitives::backtrack_err(
        "duration phrase",
        "turn, combat, upkeep, untap-step, or game duration phrase",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_word_spans_are_typed_and_preserve_canonical_eot_compatibility() {
        let words = ["creature", "until", "end", "of", "turn", "instead"];
        assert_eq!(
            find_leaf_duration_words(&words),
            Some(LeafDurationWordSpan {
                duration: LeafDurationPhrase::UntilEndOfTurn,
                start: 1,
                end: 5,
            })
        );
        assert_eq!(
            find_leaf_canonical_until_end_of_turn_words(&words),
            Some(LeafDurationWordSpan {
                duration: LeafDurationPhrase::UntilEndOfTurn,
                start: 1,
                end: 5,
            })
        );

        let with_article = ["until", "the", "end", "of", "turn"];
        assert_eq!(
            parse_leaf_duration_prefix_words(&with_article)
                .expect("general duration")
                .duration,
            LeafDurationPhrase::UntilEndOfTurn
        );
        assert!(find_leaf_canonical_until_end_of_turn_words(&with_article).is_none());
    }
}
