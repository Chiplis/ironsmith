use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::primitives;
use crate::lexer::{LexStream, OwnedLexToken, TokenKind};
use crate::token_primitives::split_em_dash_label_prefix_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreservedKeywordLabelKind {
    CostOrCasting,
    Activated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelPrefixKind {
    PreservedKeyword(PreservedKeywordLabelKind),
    CouncilChoice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericResultPrefixShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementLabelSplitShape<'a> {
    pub label_tokens: &'a [OwnedLexToken],
    pub body_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementLabelStripShape<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub stripped_labels: usize,
}

pub fn parse_label_prefix_kind_tokens(tokens: &[OwnedLexToken]) -> Option<LabelPrefixKind> {
    primitives::parse_prefix(tokens, council_choice_label)
        .map(|((), _)| LabelPrefixKind::CouncilChoice)
        .or_else(|| {
            primitives::parse_prefix(tokens, preserved_keyword_label)
                .map(|(kind, _)| LabelPrefixKind::PreservedKeyword(kind))
        })
}

pub fn parse_preserved_keyword_label_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PreservedKeywordLabelKind> {
    match parse_label_prefix_kind_tokens(tokens)? {
        LabelPrefixKind::PreservedKeyword(kind) => Some(kind),
        LabelPrefixKind::CouncilChoice => None,
    }
}

pub fn parse_numeric_result_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NumericResultPrefixShape> {
    if matches!(tokens, [number, pipe, ..]
        if number.kind == TokenKind::Number && pipe.kind == TokenKind::Pipe)
    {
        return Some(NumericResultPrefixShape);
    }
    if tokens
        .first()
        .is_some_and(token_is_compact_ascii_numeric_range)
        && tokens
            .get(1)
            .is_some_and(|token| token.kind == TokenKind::Pipe)
    {
        return Some(NumericResultPrefixShape);
    }
    let (_, remaining) = primitives::parse_prefix(tokens, numeric_result_head)?;
    primitives::find_prefix(remaining, || primitives::token_kind(TokenKind::Pipe).void())?;
    Some(NumericResultPrefixShape)
}

fn token_is_compact_ascii_numeric_range(token: &OwnedLexToken) -> bool {
    if token.kind != TokenKind::Word {
        return false;
    }
    matches!(
        crate::word_primitives::parse_ascii_numeric_range(token.parser_text()),
        Some((min, max)) if min <= max
    )
}

pub fn parse_statement_label_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementLabelSplitShape<'_>> {
    if parse_numeric_result_prefix_tokens(tokens).is_some() {
        return None;
    }
    let (label_tokens, body_tokens) = split_em_dash_label_prefix_tokens(tokens)?;
    (!label_tokens.is_empty() && !body_tokens.is_empty()).then_some(StatementLabelSplitShape {
        label_tokens,
        body_tokens,
    })
}

pub fn parse_statement_label_strip_tokens(
    mut tokens: &[OwnedLexToken],
) -> StatementLabelStripShape<'_> {
    let mut stripped_labels = 0;
    while let Some(split) = parse_statement_label_split_tokens(tokens) {
        if parse_preserved_keyword_label_tokens(split.label_tokens).is_some() {
            break;
        }
        stripped_labels += 1;
        tokens = split.body_tokens;
    }
    StatementLabelStripShape {
        body_tokens: tokens,
        stripped_labels,
    }
}

fn numeric_result_head(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::token_kind(TokenKind::Number)
        .void()
        .parse_next(input)?;
    winnow::combinator::alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
    ))
    .void()
    .parse_next(input)?;
    primitives::token_kind(TokenKind::Number)
        .void()
        .parse_next(input)
}

fn council_choice_label(input: &mut LexStream<'_>) -> WResult<()> {
    winnow::combinator::alt((
        primitives::phrase(&["will", "of", "the", "council"]),
        primitives::phrase(&["council's", "dilemma"]),
        primitives::phrase(&["secret", "council"]),
    ))
    .parse_next(input)
}

fn preserved_keyword_label(input: &mut LexStream<'_>) -> WResult<PreservedKeywordLabelKind> {
    let head = primitives::word_parser_text.parse_next(input)?;
    match head {
        "buyback" | "blitz" | "bestow" | "cumulative" | "cycling" | "echo" | "equip" | "epic"
        | "escape" | "escalate" | "eternalize" | "evoke" | "flashback" | "kicker"
        | "multikicker" | "modular" | "morph" | "megamorph" | "prototype" | "replicate"
        | "reinforce" | "splice" | "squad" | "spectacle" | "strive" | "surge" | "suspend"
        | "ward" => Ok(PreservedKeywordLabelKind::CostOrCasting),
        "boast" | "renew" => Ok(PreservedKeywordLabelKind::Activated),
        _ => Err(primitives::backtrack_err(
            "keyword label",
            "known keyword label head",
        )),
    }
}

#[cfg(test)]
#[path = "labels_inline_tests.rs"]
mod tests;
