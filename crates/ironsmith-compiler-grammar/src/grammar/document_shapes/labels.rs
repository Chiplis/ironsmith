use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::primitives;
use crate::effect::Comparison;
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
pub struct NumericResultPrefixShape {
    pub comparison: Comparison,
    pub body_start: usize,
}

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
    // Recognize the complete header, not a numeric prefix somewhere before
    // a pipe. Recognition, continuation attachment and lowering must agree.
    let pipe = tokens.iter().position(|token| token.kind == TokenKind::Pipe)?;
    let number = |token: &OwnedLexToken| {
        (token.kind == TokenKind::Number)
            .then(|| token.parser_text().parse::<i32>().ok())
            .flatten()
    };
    let comparison = match &tokens[..pipe] {
        [exact] if exact.kind == TokenKind::Number => Comparison::Equal(number(exact)?),
        [range] if range.kind == TokenKind::Word => {
            let (min, max) = crate::word_primitives::parse_ascii_numeric_range(range.parser_text())?;
            if min > max { return None; }
            Comparison::BetweenInclusive(min, max)
        }
        [min, plus] if plus.kind == TokenKind::Plus => {
            Comparison::GreaterThanOrEqual(number(min)?)
        }
        [max, or, less] if or.is_word("or") && less.is_word("less") => {
            Comparison::LessThanOrEqual(number(max)?)
        }
        [min, dash, max] if matches!(dash.kind, TokenKind::Dash | TokenKind::EmDash) => {
            let (min, max) = (number(min)?, number(max)?);
            if min > max { return None; }
            Comparison::BetweenInclusive(min, max)
        }
        _ => return None,
    };
    Some(NumericResultPrefixShape { comparison, body_start: pipe + 1 })
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
        | "escape" | "escalate" | "eternalize" | "evoke" | "flashback" | "freerunning"
        | "kicker" | "multikicker" | "modular" | "morph" | "megamorph" | "prototype"
        | "replicate" | "reinforce" | "splice" | "squad" | "spectacle" | "strive" | "surge"
        | "suspend" | "ward" => Ok(PreservedKeywordLabelKind::CostOrCasting),
        "boast" | "renew" | "reconfigure" => Ok(PreservedKeywordLabelKind::Activated),
        _ => Err(primitives::backtrack_err(
            "keyword label",
            "known keyword label head",
        )),
    }
}

#[cfg(test)]
#[path = "labels_inline_tests.rs"]
mod tests;
