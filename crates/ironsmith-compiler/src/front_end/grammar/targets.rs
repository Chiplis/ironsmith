use std::ops::Range;

use winnow::combinator::{alt, eof, opt, peek};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::{ChoiceCount, Value};
use ironsmith_core::{EffectMetric, EffectMetricSource};

use super::super::lexer::{LexStream, OwnedLexToken, TokenWordView};
use super::leaf::{
    parse_leaf_choice_count_prefix_lexed, parse_leaf_number_prefix_lexed,
    parse_leaf_target_count_range_prefix_lexed,
};
use super::primitives;

#[path = "targets/shapes.rs"]
mod shapes;
pub(crate) use shapes::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TargetRecoveryCandidate<'a> {
    pub(crate) tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct TargetParseEnvelope<'a> {
    pub(crate) counted_any_target: Option<ChoiceCount>,
    pub(crate) recovery_candidates: Vec<TargetRecoveryCandidate<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReferencedTargetPrefix<'a> {
    pub(crate) count: u32,
    pub(crate) object_tokens: &'a [OwnedLexToken],
    pub(crate) other: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TargetPreparationFacts {
    pub(crate) clear_source_linked_exile: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DynamicTargetCountPrefix<'a> {
    pub(crate) count: ChoiceCount,
    pub(crate) value: Value,
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

fn parse_dynamic_target_count_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(ChoiceCount, Value)> {
    let up_to = opt(primitives::phrase(&["up", "to"]))
        .parse_next(input)?
        .is_some();
    let multiplier = alt((
        primitives::kw("twice").value(2i32),
        (
            parse_leaf_number_prefix_lexed.verify_map(|amount| i32::try_from(amount).ok()),
            alt((primitives::kw("time"), primitives::kw("times"))),
        )
            .map(|(amount, _)| amount),
    ))
    .parse_next(input)?;
    primitives::kw("x").parse_next(input)?;
    peek(alt((
        primitives::kw("target").void(),
        primitives::phrase(&["other", "target"]).void(),
    )))
    .parse_next(input)?;

    Ok((
        if up_to {
            ChoiceCount::up_to_dynamic_x()
        } else {
            ChoiceCount::dynamic_x()
        },
        Value::XTimes(multiplier),
    ))
}

fn parse_that_many_target_count_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(ChoiceCount, Value)> {
    let up_to = opt(primitives::phrase(&["up", "to"]))
        .parse_next(input)?
        .is_some();
    primitives::phrase(&["that", "many"]).parse_next(input)?;
    peek(alt((
        primitives::kw("target").void(),
        primitives::phrase(&["other", "target"]).void(),
    )))
    .parse_next(input)?;

    Ok((
        if up_to {
            ChoiceCount::up_to_dynamic_x()
        } else {
            ChoiceCount::dynamic_x()
        },
        Value::PendingEffectMetric {
            source: EffectMetricSource::Outcome,
            metric: EffectMetric::Count,
        },
    ))
}

pub(crate) fn parse_dynamic_target_count_prefix(
    tokens: &[OwnedLexToken],
) -> Option<DynamicTargetCountPrefix<'_>> {
    let ((count, value), target_tokens) = primitives::parse_prefix(
        tokens,
        alt((
            parse_that_many_target_count_prefix_lexed,
            parse_dynamic_target_count_prefix_lexed,
        )),
    )?;
    Some(DynamicTargetCountPrefix {
        count,
        value,
        target_tokens,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetControllerSetConstraint {
    None,
    SameController,
    DifferentControllers,
}

#[derive(Debug, Clone)]
pub(crate) struct TargetControllerSetSplit {
    pub(crate) core_tokens: Vec<OwnedLexToken>,
    pub(crate) constraint: TargetControllerSetConstraint,
}

pub(crate) fn parse_target_envelope(tokens: &[OwnedLexToken]) -> TargetParseEnvelope<'_> {
    TargetParseEnvelope {
        counted_any_target: parse_counted_any_target(tokens),
        recovery_candidates: parse_target_recovery_candidates(tokens),
    }
}

pub(crate) fn parse_referenced_target_prefix(
    tokens: &[OwnedLexToken],
) -> Option<ReferencedTargetPrefix<'_>> {
    if TokenWordView::new(tokens).len() < 4 {
        return None;
    }
    let mut input = LexStream::new(tokens);
    let before_count = input.len();
    let count = parse_leaf_number_prefix_lexed.parse_next(&mut input).ok()?;
    if before_count.saturating_sub(input.len()) != 1 {
        return None;
    }
    primitives::kw("of").parse_next(&mut input).ok()?;
    alt((primitives::kw("those"), primitives::kw("them")))
        .parse_next(&mut input)
        .ok()?;
    let consumed = tokens.len().checked_sub(input.len())?;
    let object_tokens = trim_comma_edges(tokens.get(consumed..)?);
    if object_tokens.is_empty() {
        return None;
    }
    let mut object_input = LexStream::new(object_tokens);
    let other = alt((primitives::kw("other"), primitives::kw("another")))
        .parse_next(&mut object_input)
        .is_ok();
    Some(ReferencedTargetPrefix {
        count,
        object_tokens,
        other,
    })
}

pub(crate) fn parse_target_preparation_facts(
    tokens: &[OwnedLexToken],
    explicit_target: bool,
) -> TargetPreparationFacts {
    if !explicit_target {
        return TargetPreparationFacts::default();
    }
    let words = TokenWordView::new(tokens).to_word_refs();
    TargetPreparationFacts {
        clear_source_linked_exile: parse_word_phrase_range(&words, &["exiled"]).is_some()
            && parse_word_phrase_range(&words, &["card"]).is_some()
            && parse_word_phrase_range(&words, &["exiled", "with"]).is_none()
            && parse_word_phrase_range(&words, &["used", "to", "craft"]).is_none(),
    }
}

pub(crate) fn parse_target_controller_set_suffix(
    tokens: &[OwnedLexToken],
) -> TargetControllerSetSplit {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    for tail_words in [5usize, 4] {
        let Some(tail_start) = words.len().checked_sub(tail_words) else {
            continue;
        };
        let Some(constraint) = parse_controller_set_constraint_words(&words[tail_start..]) else {
            continue;
        };
        let Some(token_end) = view.token_start_indices().get(tail_start).copied() else {
            break;
        };
        return TargetControllerSetSplit {
            core_tokens: trim_comma_edges(tokens.get(..token_end).unwrap_or_default()).to_vec(),
            constraint,
        };
    }

    TargetControllerSetSplit {
        core_tokens: tokens.to_vec(),
        constraint: TargetControllerSetConstraint::None,
    }
}

fn parse_controller_set_constraint_words(words: &[&str]) -> Option<TargetControllerSetConstraint> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let constraint = alt((
        (
            primitives::word_slice_exact("controlled"),
            primitives::word_slice_exact("by"),
            primitives::word_slice_exact("the"),
            primitives::word_slice_exact("same"),
            primitives::word_slice_exact("player"),
        )
            .value(TargetControllerSetConstraint::SameController),
        (
            primitives::word_slice_exact("controlled"),
            primitives::word_slice_exact("by"),
            primitives::word_slice_exact("same"),
            primitives::word_slice_exact("player"),
        )
            .value(TargetControllerSetConstraint::SameController),
        (
            primitives::word_slice_exact("controlled"),
            primitives::word_slice_exact("by"),
            primitives::word_slice_exact("different"),
            primitives::word_slice_exact("players"),
        )
            .value(TargetControllerSetConstraint::DifferentControllers),
    ))
    .parse_next(&mut input)
    .ok()?;
    primitives::word_slice_eof.parse_next(&mut input).ok()?;
    Some(constraint)
}

fn parse_counted_any_target(tokens: &[OwnedLexToken]) -> Option<ChoiceCount> {
    let mut input = LexStream::new(tokens);
    parse_counted_any_target_lexed.parse_next(&mut input).ok()
}

fn parse_counted_any_target_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ChoiceCount> {
    opt(primitives::phrase(&["each", "of"])).parse_next(input)?;
    let count = alt((
        parse_leaf_target_count_range_prefix_lexed,
        parse_leaf_choice_count_prefix_lexed,
    ))
    .parse_next(input)?;
    alt((primitives::kw("target"), primitives::kw("targets"))).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(count)
}

fn parse_target_recovery_candidates(tokens: &[OwnedLexToken]) -> Vec<TargetRecoveryCandidate<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let mut candidates = Vec::new();

    if let Some(except_word) = parse_word_phrase_range(&words, &["except"]).map(|span| span.start)
        && except_word > 0
        && let Some(except_token) = view.token_start_indices().get(except_word).copied()
    {
        let before_except = trim_comma_edges(tokens.get(..except_token).unwrap_or_default());
        if !before_except.is_empty() {
            candidates.push(TargetRecoveryCandidate {
                tokens: before_except,
            });
        }

        let mut words_input: primitives::WordSliceInput<'_> = words.as_slice();
        if parse_copy_word(&mut words_input).is_ok()
            && let Some(copy_end) = view.token_start_indices().get(1).copied()
        {
            let without_copy =
                trim_comma_edges(tokens.get(copy_end..except_token).unwrap_or_default());
            if !without_copy.is_empty() {
                candidates.push(TargetRecoveryCandidate {
                    tokens: without_copy,
                });
            }
        }
    }

    let mut words_input: primitives::WordSliceInput<'_> = words.as_slice();
    if parse_leading_condition_word(&mut words_input).is_ok() {
        let mut word_start = words.len();
        while word_start > 1 {
            word_start -= 1;
            let Some(token_start) = view.token_start_indices().get(word_start).copied() else {
                continue;
            };
            let candidate = trim_comma_edges(tokens.get(token_start..).unwrap_or_default());
            if candidate.is_empty() {
                continue;
            }
            let candidate_words = TokenWordView::new(candidate).to_word_refs();
            let mut candidate_input: primitives::WordSliceInput<'_> = candidate_words.as_slice();
            if parse_split_prefix_word(&mut candidate_input).is_ok() {
                continue;
            }
            candidates.push(TargetRecoveryCandidate { tokens: candidate });
        }
    }

    candidates
}

fn parse_leading_condition_word(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("during"),
        primitives::word_slice_exact("if"),
        primitives::word_slice_exact("until"),
    ))
    .void()
    .parse_next(input)
}

fn parse_split_prefix_word(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("and"),
        primitives::word_slice_exact("during"),
        primitives::word_slice_exact("if"),
        primitives::word_slice_exact("then"),
        primitives::word_slice_exact("until"),
    ))
    .void()
    .parse_next(input)
}

fn parse_copy_word(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    primitives::word_slice_exact("copy")
        .void()
        .parse_next(input)
}

fn parse_word_phrase_range(words: &[&str], expected: &[&'static str]) -> Option<Range<usize>> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let mut offset = 0;
    while !input.is_empty() {
        let mut probe = input;
        if parse_word_phrase(&mut probe, expected).is_ok() {
            return Some(offset..offset + expected.len());
        }
        parse_any_word_slice.parse_next(&mut input).ok()?;
        offset += 1;
    }
    None
}

fn parse_any_word_slice<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<&'a str> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err("word", "word"));
    };
    *input = rest;
    Ok(*word)
}

fn parse_word_phrase(
    input: &mut primitives::WordSliceInput<'_>,
    expected: &[&'static str],
) -> WResult<()> {
    for word in expected {
        primitives::word_slice_exact(*word)
            .void()
            .parse_next(input)?;
    }
    Ok(())
}

fn trim_comma_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0;
    let mut end = tokens.len();
    while start < end && tokens[start].is_comma() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_comma() {
        end -= 1;
    }
    tokens.get(start..end).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
        TokenWordView::new(tokens).to_word_refs()
    }

    #[test]
    fn envelope_recognizes_bare_counted_any_targets() {
        for raw in [
            "two targets",
            "one or two targets",
            "up to X target",
            "each of any number of targets",
        ] {
            let tokens = lex_line(raw, 0).expect("lex");
            assert!(
                parse_target_envelope(&tokens).counted_any_target.is_some(),
                "{raw}"
            );
        }

        let tokens = lex_line("two target creatures", 0).expect("lex");
        assert!(parse_target_envelope(&tokens).counted_any_target.is_none());
    }

    #[test]
    fn scaled_dynamic_target_prefix_preserves_count_and_value() {
        let tokens = lex_line("up to twice X target cards from graveyards", 0).unwrap();
        let parsed = parse_dynamic_target_count_prefix(&tokens).expect("dynamic target prefix");
        assert!(parsed.count.is_up_to_dynamic_x());
        assert_eq!(parsed.value, Value::XTimes(2));
        assert_eq!(
            words(parsed.target_tokens),
            ["target", "cards", "from", "graveyards"]
        );

        let tokens = lex_line("three times X target creatures", 0).unwrap();
        let parsed = parse_dynamic_target_count_prefix(&tokens).expect("scaled target prefix");
        assert!(parsed.count.is_dynamic_x());
        assert_eq!(parsed.value, Value::XTimes(3));

        let tokens = lex_line("up to that many other target creatures", 0).unwrap();
        let parsed = parse_dynamic_target_count_prefix(&tokens).expect("result-count prefix");
        assert!(parsed.count.is_up_to_dynamic_x());
        assert!(matches!(
            parsed.value,
            Value::PendingEffectMetric {
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::Count,
            }
        ));
        assert_eq!(
            words(parsed.target_tokens),
            ["other", "target", "creatures"]
        );
    }

    #[test]
    fn recovery_candidates_preserve_except_and_copy_order() {
        let tokens = lex_line("copy target artifact, except it is blue", 0).expect("lex");
        let envelope = parse_target_envelope(&tokens);
        assert_eq!(envelope.recovery_candidates.len(), 2);
        assert_eq!(
            words(envelope.recovery_candidates[0].tokens),
            ["copy", "target", "artifact"]
        );
        assert_eq!(
            words(envelope.recovery_candidates[1].tokens),
            ["target", "artifact"]
        );
    }

    #[test]
    fn conditional_recovery_candidates_exclude_split_prefixes() {
        let tokens = lex_line("if this is attacking, then target creature", 0).expect("lex");
        let envelope = parse_target_envelope(&tokens);
        assert!(
            envelope
                .recovery_candidates
                .iter()
                .any(|candidate| { words(candidate.tokens) == ["target", "creature"] })
        );
        assert!(envelope.recovery_candidates.iter().all(|candidate| {
            !matches!(
                words(candidate.tokens).first().copied(),
                Some("and" | "during" | "if" | "then" | "until")
            )
        }));
    }

    #[test]
    fn referenced_target_prefix_preserves_count_object_and_other() {
        let tokens = lex_line("two of those other creatures", 0).expect("lex");
        let parsed = parse_referenced_target_prefix(&tokens).expect("referenced target");
        assert_eq!(parsed.count, 2);
        assert!(parsed.other);
        assert_eq!(words(parsed.object_tokens), ["other", "creatures"]);

        let tokens = lex_line("twice of them, cards", 0).expect("lex");
        let parsed = parse_referenced_target_prefix(&tokens).expect("adverb count");
        assert_eq!(parsed.count, 2);
        assert_eq!(words(parsed.object_tokens), ["cards"]);
    }

    #[test]
    fn preparation_fact_preserves_explicit_exiled_card_exceptions() {
        let tokens = lex_line("face-up exiled card", 0).expect("lex");
        assert!(parse_target_preparation_facts(&tokens, true).clear_source_linked_exile);
        assert!(!parse_target_preparation_facts(&tokens, false).clear_source_linked_exile);

        for raw in [
            "card exiled with this",
            "card used to craft this",
            "exiled cards",
        ] {
            let tokens = lex_line(raw, 0).expect("lex");
            assert!(
                !parse_target_preparation_facts(&tokens, true).clear_source_linked_exile,
                "{raw}"
            );
        }
    }

    #[test]
    fn controller_set_suffix_returns_typed_constraint_and_core() {
        for (raw, expected) in [
            (
                "two creatures controlled by the same player",
                TargetControllerSetConstraint::SameController,
            ),
            (
                "two creatures controlled by same player",
                TargetControllerSetConstraint::SameController,
            ),
            (
                "two creatures controlled by different players",
                TargetControllerSetConstraint::DifferentControllers,
            ),
        ] {
            let tokens = lex_line(raw, 0).expect("lex");
            let split = parse_target_controller_set_suffix(&tokens);
            assert_eq!(split.constraint, expected, "{raw}");
            assert_eq!(words(&split.core_tokens), ["two", "creatures"], "{raw}");
        }
    }
}
