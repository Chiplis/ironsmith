use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::TargetSetPredicateAst;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PossessionAction {
    Control,
    ControlOrControlled,
    Has,
    Own,
    Copula,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PossessionRelationShape<'a> {
    pub(super) subject_tokens: &'a [OwnedLexToken],
    pub(super) prefix_tokens: &'a [OwnedLexToken],
    pub(super) tail_tokens: &'a [OwnedLexToken],
    pub(super) has_different_powers_modifier: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PossessionRelationWords<'a> {
    pub(super) subject_words: &'a [&'a str],
    pub(super) prefix_words: &'a [&'a str],
    pub(super) tail_words: &'a [&'a str],
    pub(super) has_different_powers_modifier: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PrepositionalCopulaShape<'a> {
    pub(super) subject_tokens: &'a [OwnedLexToken],
    pub(super) preposition_tokens: &'a [OwnedLexToken],
    pub(super) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct NegatedControlShape<'a> {
    pub(super) subject_tokens: &'a [OwnedLexToken],
    pub(super) negation_tokens: &'a [OwnedLexToken],
    pub(super) tail_tokens: &'a [OwnedLexToken],
}

pub(super) fn parse_possession_relation(
    tokens: &[OwnedLexToken],
    action: PossessionAction,
    allow_different_powers: bool,
) -> Option<PossessionRelationShape<'_>> {
    let tokens = trim_clause(tokens);
    if allow_different_powers {
        let mut input = LexStream::new(tokens);
        if let Ok(shape) = parse_possession_with_modifier(&mut input, tokens, action)
            && input.is_empty()
        {
            return Some(shape);
        }
    }
    let mut input = LexStream::new(tokens);
    let shape = crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_possession_basic(input, tokens, action)
    })?;
    input.is_empty().then_some(shape)
}

pub(super) fn parse_control_relation_words<'a>(
    words: &'a [&'a str],
    allow_different_powers: bool,
) -> Option<PossessionRelationWords<'a>> {
    if allow_different_powers {
        let mut input: primitives::WordSliceInput<'a> = words;
        if let Ok(shape) = parse_control_words_with_modifier(&mut input)
            && input.is_empty()
        {
            return Some(shape);
        }
    }
    let mut input: primitives::WordSliceInput<'a> = words;
    let shape = crate::grammar::primitives::take_leaf(&mut input, parse_control_words_basic)?;
    input.is_empty().then_some(shape)
}

pub(super) fn parse_prepositional_copula<'a, 'p>(
    tokens: &'a [OwnedLexToken],
    preposition_words: &'p [&'p str],
) -> Option<PrepositionalCopulaShape<'a>> {
    let tokens = trim_clause(tokens);
    if tokens.first().is_some_and(|token| token.is_word("it's"))
        && tokens.get(1).is_some_and(|token| {
            token
                .as_word()
                .is_some_and(|word| crate::slice_primitives::contains(preposition_words, &word))
        })
        && tokens.len() > 2
    {
        return Some(PrepositionalCopulaShape {
            subject_tokens: &tokens[..1],
            preposition_tokens: &tokens[1..2],
            tail_tokens: &tokens[2..],
        });
    }
    let mut input = LexStream::new(tokens);
    let subject_tokens = crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        take_until_action(input, PossessionAction::Copula)
    })?;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_action(input, PossessionAction::Copula)
    })?;
    let preposition_tokens = crate::grammar::primitives::take_leaf(
        &mut input,
        (|input: &mut LexStream<'a>| -> WResult<()> {
            expected_any_word(input, preposition_words)?;
            Ok(())
        })
        .take(),
    )?;
    let tail_tokens = crate::grammar::primitives::take_leaf(&mut input, take_remaining)?;
    (!tail_tokens.is_empty()).then_some(PrepositionalCopulaShape {
        subject_tokens,
        preposition_tokens,
        tail_tokens,
    })
}

pub(super) fn parse_existential_object(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, primitives::kw("there"))?;
    crate::grammar::primitives::take_leaf(
        &mut input,
        opt(alt((primitives::kw("is"), primitives::kw("are")))),
    )?;
    let object_tokens = crate::grammar::primitives::take_leaf(&mut input, take_remaining)?;
    (!object_tokens.is_empty()).then_some(object_tokens)
}

pub(super) fn parse_negated_control(tokens: &[OwnedLexToken]) -> Option<NegatedControlShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    let subject_tokens = crate::grammar::primitives::take_leaf(&mut input, take_until_negation)?;
    let negation_tokens =
        crate::grammar::primitives::take_leaf(&mut input, parse_control_negation.take())?;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_action(input, PossessionAction::Control)
    })?;
    let tail_tokens = crate::grammar::primitives::take_leaf(&mut input, take_remaining)?;
    (!tail_tokens.is_empty()).then_some(NegatedControlShape {
        subject_tokens,
        negation_tokens,
        tail_tokens,
    })
}

pub(super) fn parse_target_set_predicate(
    tokens: &[OwnedLexToken],
) -> Option<TargetSetPredicateAst> {
    let tokens = trim_clause(tokens);
    crate::grammar::primitives::probe_all(
        tokens,
        parse_different_color_sets,
        "target-set color relation",
    )
}

fn parse_different_color_sets(input: &mut LexStream<'_>) -> WResult<TargetSetPredicateAst> {
    primitives::phrase(&["either", "one", "is", "a", "color", "the", "other"]).parse_next(input)?;
    alt((
        primitives::kw("isnt").void(),
        primitives::kw("isn't").void(),
        primitives::phrase(&["is", "not"]),
    ))
    .parse_next(input)?;
    Ok(TargetSetPredicateAst::DifferentColorSets)
}

fn parse_possession_basic<'a>(
    input: &mut LexStream<'a>,
    full_tokens: &'a [OwnedLexToken],
    action: PossessionAction,
) -> WResult<PossessionRelationShape<'a>> {
    let initial_len = input.len();
    let subject_tokens = take_until_action(input, action)?;
    parse_action(input, action)?;
    let prefix_len = initial_len.saturating_sub(input.len());
    let tail_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    if tail_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "condition possession relation",
            "nonempty object phrase",
        ));
    }
    let prefix_tokens = full_tokens
        .get(..prefix_len)
        .ok_or_else(|| primitives::backtrack_err("condition capture", "valid prefix range"))?;
    Ok(PossessionRelationShape {
        subject_tokens,
        prefix_tokens,
        tail_tokens,
        has_different_powers_modifier: false,
    })
}

fn parse_possession_with_modifier<'a>(
    input: &mut LexStream<'a>,
    full_tokens: &'a [OwnedLexToken],
    action: PossessionAction,
) -> WResult<PossessionRelationShape<'a>> {
    let initial_len = input.len();
    let subject_tokens = take_until_action(input, action)?;
    parse_action(input, action)?;
    let prefix_len = initial_len.saturating_sub(input.len());
    let tail_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_different_powers_suffix))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    parse_different_powers_suffix(input)?;
    eof.parse_next(input)?;
    let prefix_tokens = full_tokens
        .get(..prefix_len)
        .ok_or_else(|| primitives::backtrack_err("condition capture", "valid prefix range"))?;
    Ok(PossessionRelationShape {
        subject_tokens,
        prefix_tokens,
        tail_tokens,
        has_different_powers_modifier: true,
    })
}

fn parse_control_words_basic<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<PossessionRelationWords<'a>> {
    let initial = *input;
    let subject_words = take_until_control_word(input)?;
    parse_control_word(input)?;
    let prefix_len = initial.len().saturating_sub(input.len());
    let tail_words = *input;
    if tail_words.is_empty() {
        return Err(primitives::backtrack_err(
            "control words",
            "nonempty object phrase",
        ));
    }
    *input = &[];
    Ok(PossessionRelationWords {
        subject_words,
        prefix_words: &initial[..prefix_len],
        tail_words,
        has_different_powers_modifier: false,
    })
}

fn parse_control_words_with_modifier<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<PossessionRelationWords<'a>> {
    let initial = *input;
    let subject_words = take_until_control_word(input)?;
    parse_control_word(input)?;
    let prefix_len = initial.len().saturating_sub(input.len());
    let tail_words = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(parse_different_powers_word_suffix),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    parse_different_powers_word_suffix(input)?;
    primitives::word_slice_eof(input)?;
    Ok(PossessionRelationWords {
        subject_words,
        prefix_words: &initial[..prefix_len],
        tail_words,
        has_different_powers_modifier: true,
    })
}

fn take_until_action<'a>(
    input: &mut LexStream<'a>,
    action: PossessionAction,
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(|input: &mut LexStream<'a>| parse_action(input, action)),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)
}

fn parse_action(input: &mut LexStream<'_>, action: PossessionAction) -> WResult<()> {
    match action {
        PossessionAction::Control => alt((primitives::kw("control"), primitives::kw("controls")))
            .void()
            .parse_next(input),
        PossessionAction::ControlOrControlled => {
            alt((primitives::kw("control"), primitives::kw("controlled")))
                .void()
                .parse_next(input)
        }
        PossessionAction::Has => alt((primitives::kw("has"), primitives::kw("have")))
            .void()
            .parse_next(input),
        PossessionAction::Own => alt((primitives::kw("own"), primitives::kw("owns")))
            .void()
            .parse_next(input),
        PossessionAction::Copula => alt((primitives::kw("is"), primitives::kw("are")))
            .void()
            .parse_next(input),
    }
}

fn take_until_negation<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_control_negation))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn parse_control_negation(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("dont").void(),
        primitives::kw("don't").void(),
        primitives::phrase(&["do", "not"]),
    ))
    .parse_next(input)
}

fn parse_different_powers_suffix(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("with").parse_next(input)?;
    primitives::kw("different").parse_next(input)?;
    alt((primitives::kw("powers"), primitives::kw("power")))
        .void()
        .parse_next(input)
}

fn take_until_control_word<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<&'a [&'a str]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_control_word))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn parse_control_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<&'a str> {
    alt((
        primitives::word_slice_exact("control"),
        primitives::word_slice_exact("controls"),
    ))
    .parse_next(input)
}

fn parse_different_powers_word_suffix(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    primitives::word_slice_exact("with").parse_next(input)?;
    primitives::word_slice_exact("different").parse_next(input)?;
    alt((
        primitives::word_slice_exact("powers"),
        primitives::word_slice_exact("power"),
    ))
    .void()
    .parse_next(input)
}

fn expected_any_word<'a, 'p>(
    input: &mut LexStream<'a>,
    expected: &'p [&'p str],
) -> WResult<&'a OwnedLexToken> {
    any.verify(|token: &&OwnedLexToken| token.is_any_word(expected))
        .parse_next(input)
}

fn take_remaining<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    rest.parse_next(input)
}

fn trim_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    super::super::super::util::trim_edge_punctuation_tokens(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn captures_typed_possession_and_negated_relations() {
        let control = lex("You control three creatures with different powers.");
        let shape = parse_possession_relation(&control, PossessionAction::Control, true)
            .expect("control relation");
        assert!(shape.has_different_powers_modifier);
        assert_eq!(shape.subject_tokens.len(), 1);
        assert_eq!(shape.tail_tokens.len(), 2);

        let negated = lex("You don't control a creature.");
        let shape = parse_negated_control(&negated).expect("negated control relation");
        assert_eq!(shape.subject_tokens.len(), 1);
        assert_eq!(shape.negation_tokens.len(), 1);
    }

    #[test]
    fn parses_target_set_color_difference_relation() {
        for text in [
            "either one is a color the other isn't",
            "either one is a color the other isnt",
            "either one is a color the other is not",
        ] {
            assert_eq!(
                parse_target_set_predicate(&lex(text)),
                Some(TargetSetPredicateAst::DifferentColorSets),
                "failed to parse {text:?}",
            );
        }

        assert_eq!(parse_target_set_predicate(&lex("either one is red")), None);
    }
}
