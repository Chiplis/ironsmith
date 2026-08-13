use crate::cards::builders::TextSpan;
use crate::recognition::{ParseDiagnostic, ParseExpectation, ParseOutcome, RuleId};

use super::super::super::lexer::{OwnedLexToken, TokenKind};
use super::activation_heads::{LeafActivationCostHead, parse_leaf_activation_cost_head_tokens};
use super::condition_prefixes::{
    LeafConditionIntroPrefix, parse_leaf_condition_intro_prefix_tokens,
};
use super::durations::{
    LeafDurationPhrase, LeafDurationPrefix, parse_leaf_restriction_duration_prefix_tokens,
};
use super::mana::{LeafManaCostPrefix, parse_leaf_mana_cost_prefix_tokens};
use super::numbers::{LeafNumberPrefix, parse_leaf_number_or_x_prefix_tokens};
use super::player_subjects::{LeafPlayerReferenceMode, parse_leaf_player_reference_tokens};
use super::references::LeafPlayerReference;
use super::targets::{LeafTargetHead, parse_leaf_target_head_tokens};

fn token_span(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    let first = tokens.first()?.span;
    let last = tokens.last()?.span;
    Some(TextSpan {
        line: first.line,
        start: first.start,
        end: last.end,
    })
}

fn head(tokens: &[OwnedLexToken]) -> &str {
    tokens.first().map(OwnedLexToken::parser_text).unwrap_or("")
}

fn committed_failure<T>(
    rule: RuleId,
    tokens: &[OwnedLexToken],
    expected: &'static str,
) -> ParseOutcome<T> {
    ParseOutcome::Error(ParseDiagnostic::malformed(
        rule,
        token_span(tokens),
        [ParseExpectation::new(expected)],
        format!("{rule} recognized its leaf head but could not complete the typed fact"),
    ))
}

pub(crate) fn recognize_activation_cost_head(
    tokens: &[OwnedLexToken],
) -> ParseOutcome<LeafActivationCostHead> {
    const RULE: RuleId = RuleId::new("leaf.activation-cost-head");
    let commits = tokens.first().is_some_and(|token| {
        matches!(token.kind, TokenKind::ManaGroup | TokenKind::LBracket)
            || matches!(
                token.parser_text(),
                "tap"
                    | "t"
                    | "pay"
                    | "discard"
                    | "mill"
                    | "sacrifice"
                    | "put"
                    | "remove"
                    | "exile"
                    | "return"
                    | "e"
            )
    });
    if !commits {
        return ParseOutcome::NoMatch;
    }
    match parse_leaf_activation_cost_head_tokens(tokens) {
        Some(value) => ParseOutcome::matched(value, token_span(tokens)),
        None => committed_failure(RULE, tokens, "complete activation-cost head"),
    }
}

pub(crate) fn recognize_mana_cost_prefix(
    tokens: &[OwnedLexToken],
) -> ParseOutcome<LeafManaCostPrefix> {
    const RULE: RuleId = RuleId::new("leaf.mana-cost-prefix");
    if !tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::ManaGroup)
    {
        return ParseOutcome::NoMatch;
    }
    match parse_leaf_mana_cost_prefix_tokens(tokens) {
        Some(value) => ParseOutcome::matched(value, token_span(tokens)),
        None => committed_failure(RULE, tokens, "one or more valid mana symbols"),
    }
}

pub(crate) fn recognize_number_prefix(tokens: &[OwnedLexToken]) -> ParseOutcome<LeafNumberPrefix> {
    const RULE: RuleId = RuleId::new("leaf.number-prefix");
    let commits = tokens.first().is_some_and(|token| {
        token.kind == TokenKind::Number
            || matches!(
                token.parser_text(),
                "x" | "a"
                    | "an"
                    | "one"
                    | "two"
                    | "three"
                    | "four"
                    | "five"
                    | "six"
                    | "seven"
                    | "eight"
                    | "nine"
                    | "ten"
                    | "once"
                    | "twice"
                    | "thrice"
            )
    });
    if !commits {
        return ParseOutcome::NoMatch;
    }
    match parse_leaf_number_or_x_prefix_tokens(tokens) {
        Some(value) => ParseOutcome::matched(value, token_span(tokens)),
        None => committed_failure(RULE, tokens, "cardinal number or X"),
    }
}

pub(crate) fn recognize_condition_intro(
    tokens: &[OwnedLexToken],
) -> ParseOutcome<LeafConditionIntroPrefix<'_>> {
    const RULE: RuleId = RuleId::new("leaf.condition-intro");
    if !matches!(head(tokens), "if" | "unless" | "as" | "for") {
        return ParseOutcome::NoMatch;
    }
    match parse_leaf_condition_intro_prefix_tokens(tokens) {
        Some(value) => ParseOutcome::matched(value, token_span(tokens)),
        None => committed_failure(RULE, tokens, "if, unless, or as-long-as introduction"),
    }
}

pub(crate) fn recognize_duration_prefix(
    tokens: &[OwnedLexToken],
) -> ParseOutcome<LeafDurationPrefix<'_, LeafDurationPhrase>> {
    const RULE: RuleId = RuleId::new("leaf.duration-prefix");
    if !matches!(head(tokens), "until" | "during" | "this" | "for" | "as") {
        return ParseOutcome::NoMatch;
    }
    match parse_leaf_restriction_duration_prefix_tokens(tokens) {
        Some(value) => ParseOutcome::matched(value, token_span(tokens)),
        None => committed_failure(RULE, tokens, "typed duration phrase"),
    }
}

pub(crate) fn recognize_player_reference(
    tokens: &[OwnedLexToken],
    mode: LeafPlayerReferenceMode,
) -> ParseOutcome<LeafPlayerReference> {
    const RULE: RuleId = RuleId::new("leaf.player-reference");
    if !matches!(
        head(tokens),
        "you"
            | "your"
            | "opponent"
            | "opponents"
            | "a"
            | "an"
            | "that"
            | "they"
            | "attacking"
            | "defending"
            | "player"
            | "players"
            | "its"
    ) {
        return ParseOutcome::NoMatch;
    }
    match parse_leaf_player_reference_tokens(tokens, mode) {
        Some(value) => ParseOutcome::matched(value, token_span(tokens)),
        None => committed_failure(RULE, tokens, "player reference valid in this role"),
    }
}

pub(crate) fn recognize_target_head(tokens: &[OwnedLexToken]) -> ParseOutcome<LeafTargetHead<'_>> {
    const RULE: RuleId = RuleId::new("leaf.target-head");
    let commits = tokens.iter().any(|token| token.parser_text() == "target")
        || matches!(
            head(tokens),
            "up" | "a"
                | "an"
                | "the"
                | "any"
                | "each"
                | "another"
                | "one"
                | "two"
                | "three"
                | "x"
                | "this"
                | "that"
                | "those"
                | "other"
                | "top"
        );
    if !commits {
        return ParseOutcome::NoMatch;
    }
    match parse_leaf_target_head_tokens(tokens) {
        Ok(value) => ParseOutcome::matched(value, token_span(tokens)),
        Err(error) => ParseOutcome::Error(ParseDiagnostic::from_legacy_error(
            RULE,
            token_span(tokens),
            error,
        )),
    }
}
