use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnthemPrefixConditionKind {
    DuringTurnsOtherThanYours,
    DuringYourTurn,
    AsLongAs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnthemPrefixConditionShape {
    pub(crate) kind: AnthemPrefixConditionKind,
    pub(crate) prefix_end: usize,
    pub(crate) comma_subject_start: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FixedAnthemPrefixConditionShape<'a> {
    pub(crate) kind: AnthemPrefixConditionKind,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnthemModifierShape<'a> {
    pub(crate) modifier_word: &'a str,
    pub(crate) modifier_token: usize,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnthemTailShape<'a> {
    ForEach(&'a [OwnedLexToken]),
    WhereX(&'a [OwnedLexToken]),
    AsLongAs {
        condition_tokens: &'a [OwnedLexToken],
    },
}

pub(crate) fn parse_prefix_condition_shape(
    tokens: &[OwnedLexToken],
    action_token: usize,
) -> Option<AnthemPrefixConditionShape> {
    let (kind, rest) = primitives::parse_prefix(tokens, parse_prefix_condition_kind)?;
    let prefix_end = tokens.len().saturating_sub(rest.len());
    let comma_subject_start = first_comma_token(tokens.get(..action_token)?).map(|idx| idx + 1);
    Some(AnthemPrefixConditionShape {
        kind,
        prefix_end,
        comma_subject_start,
    })
}

pub(crate) fn parse_fixed_prefix_condition_shape(
    tokens: &[OwnedLexToken],
) -> Option<FixedAnthemPrefixConditionShape<'_>> {
    let shape = parse_prefix_condition_shape(tokens, tokens.len())?;
    if shape.kind == AnthemPrefixConditionKind::AsLongAs {
        return None;
    }
    let subject_start = shape.comma_subject_start?;
    let subject_tokens = tokens.get(subject_start..)?;
    (!subject_tokens.is_empty()).then_some(FixedAnthemPrefixConditionShape {
        kind: shape.kind,
        subject_tokens,
    })
}

pub(crate) fn parse_modifier_shape(
    tokens: &[OwnedLexToken],
    action_token: usize,
    tail_end: usize,
) -> Option<AnthemModifierShape<'_>> {
    if action_token >= tail_end || tail_end > tokens.len() {
        return None;
    }
    let modifier_start = action_token + 1;
    let modifier_tokens = tokens.get(modifier_start..tail_end)?;
    let (article_additional, modifier_tokens) = primitives::parse_prefix(
        modifier_tokens,
        (
            alt((primitives::kw("a"), primitives::kw("an"))),
            primitives::kw("additional"),
        )
            .void(),
    )
    .map(|(_, rest)| (true, rest))
    .unwrap_or((false, modifier_tokens));
    let modifier_token = modifier_start + usize::from(article_additional) * 2;
    let mut input = LexStream::new(modifier_tokens);
    let modifier_word = primitives::word_text(&mut input).ok()?;
    let consumed = modifier_tokens.len().saturating_sub(input.len());
    let tail_tokens = modifier_tokens.get(consumed..)?;
    Some(AnthemModifierShape {
        modifier_word,
        modifier_token,
        tail_tokens,
    })
}

pub(crate) fn parse_tail_shape(tokens: &[OwnedLexToken]) -> Option<AnthemTailShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if primitives::parse_prefix(tokens, primitives::phrase(&["for", "each"])).is_some() {
        return Some(AnthemTailShape::ForEach(tokens));
    }
    if primitives::parse_prefix(tokens, primitives::phrase(&["where", "x", "is"])).is_some() {
        return Some(AnthemTailShape::WhereX(tokens));
    }
    let (_, condition_tokens) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))?;
    Some(AnthemTailShape::AsLongAs { condition_tokens })
}

pub(crate) fn split_trailing_modifier_maximum(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<i32>) {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let Some((body, maximum)) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || parse_modifier_maximum)
    else {
        return (tokens, None);
    };
    (
        super::trim_anthem_clause_tokens(body),
        i32::try_from(maximum).ok(),
    )
}

fn parse_modifier_maximum(input: &mut LexStream<'_>) -> WResult<u32> {
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["to", "a", "maximum", "of"]).parse_next(input)?;
    leaf::parse_leaf_number_prefix_lexed(input)
}

pub(crate) fn parse_word_token_candidates(
    tokens: &[OwnedLexToken],
    start: usize,
    end: usize,
) -> Vec<usize> {
    let Some(search) = tokens.get(start..end.min(tokens.len())) else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    let mut input = LexStream::new(search);
    let initial_len = input.len();
    loop {
        let relative = initial_len.saturating_sub(input.len());
        let mut word_probe = input.clone();
        if primitives::word_text(&mut word_probe).is_ok() {
            candidates.push(start + relative);
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        if parsed.is_err() {
            break;
        }
    }
    candidates
}

fn parse_prefix_condition_kind(input: &mut LexStream<'_>) -> WResult<AnthemPrefixConditionKind> {
    alt((
        primitives::phrase(&["during", "turns", "other", "than", "yours"])
            .value(AnthemPrefixConditionKind::DuringTurnsOtherThanYours),
        primitives::phrase(&["during", "your", "turn"])
            .value(AnthemPrefixConditionKind::DuringYourTurn),
        primitives::phrase(&["as", "long", "as"]).value(AnthemPrefixConditionKind::AsLongAs),
    ))
    .parse_next(input)
}

fn first_comma_token(tokens: &[OwnedLexToken]) -> Option<usize> {
    primitives::find_prefix(tokens, || primitives::comma().void()).map(|(idx, _, _)| idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_leading_as_long_as_boundary() {
        let tokens = lex("As long as you control a Forest, creatures you control get +1/+1.");
        let get = primitives::find_prefix(&tokens, || primitives::kw("get").void())
            .expect("get")
            .0;
        let shape = parse_prefix_condition_shape(&tokens, get).expect("prefix");
        assert_eq!(shape.kind, AnthemPrefixConditionKind::AsLongAs);
        assert!(shape.comma_subject_start.is_some());
    }

    #[test]
    fn splits_fixed_turn_prefix_only_at_an_authored_comma() {
        let tokens = lex("During turns other than yours, this Vehicle is an artifact creature.");
        let shape = parse_fixed_prefix_condition_shape(&tokens).expect("fixed prefix");
        assert_eq!(
            shape.kind,
            AnthemPrefixConditionKind::DuringTurnsOtherThanYours
        );
        assert_eq!(
            crate::runtime_backend::lexer::render_token_slice(shape.subject_tokens),
            "this Vehicle is an artifact creature."
        );

        let no_comma = lex("During turns other than yours this Vehicle is an artifact creature.");
        assert!(parse_fixed_prefix_condition_shape(&no_comma).is_none());
        let as_long_as = lex("As long as it is your turn, this Vehicle is an artifact creature.");
        assert!(parse_fixed_prefix_condition_shape(&as_long_as).is_none());
    }

    #[test]
    fn parses_typed_anthem_tail() {
        let tokens = lex("for each creature you control");
        assert!(matches!(
            parse_tail_shape(&tokens),
            Some(AnthemTailShape::ForEach(_))
        ));
    }

    #[test]
    fn splits_authored_modifier_maximum_from_count_body() {
        let tokens = lex("for each of its creature types, to a maximum of 10.");
        let (body, maximum) = split_trailing_modifier_maximum(&tokens);

        assert_eq!(maximum, Some(10));
        assert_eq!(
            crate::runtime_backend::lexer::render_token_slice(body),
            "for each of its creature types"
        );
    }
}
