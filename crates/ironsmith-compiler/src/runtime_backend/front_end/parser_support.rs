use crate::cards::builders::CardDefinitionBuilder;
use crate::runtime_backend::semantic::ParsedRestrictions;
use crate::types::CardType;
use winnow::combinator::{alt, opt};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use super::grammar::document_shapes;
use super::grammar::primitives as grammar;
use super::grammar::restriction_facts::{
    parse_activation_restriction_tokens, parse_trigger_restriction_tokens,
};
use super::lexer::{
    LexStream, OwnedLexToken, TokenKind, lex_line, render_token_slice, split_lexed_sentences,
};

pub(crate) fn split_text_for_parse(
    raw_text: &str,
    normalized_text: &str,
    line_index: usize,
) -> (Vec<String>, ParsedRestrictions) {
    let line_sentences = split_sentences_for_parse(normalized_text, line_index);
    let mut restrictions = ParsedRestrictions::default();
    let mut parsed_portion = Vec::new();
    for sentence in line_sentences {
        if sentence.text.is_empty() {
            continue;
        }

        if queue_restriction(&sentence.tokens, &mut restrictions) {
            continue;
        }

        parsed_portion.push(sentence.text);
    }

    for restriction in extract_parenthetical_restrictions(raw_text) {
        let _ = queue_restriction(&restriction.tokens, &mut restrictions);
    }

    (parsed_portion, restrictions)
}

/// Splits an already-lexed line into semantic sentences and typed restriction
/// facts without rendering and lexing the source a second time.
pub(crate) fn split_tokens_for_parse(
    tokens: &[OwnedLexToken],
) -> (Vec<Vec<OwnedLexToken>>, ParsedRestrictions) {
    let mut restrictions = ParsedRestrictions::default();
    let mut parsed_portion = Vec::new();
    for sentence in split_lexed_sentences(tokens) {
        if sentence.is_empty() {
            continue;
        }
        if queue_restriction(sentence, &mut restrictions) {
            continue;
        }
        parsed_portion.push(sentence.to_vec());
    }

    for parenthetical in parenthetical_token_slices(tokens) {
        for sentence in split_lexed_sentences(parenthetical) {
            let _ = queue_restriction(sentence, &mut restrictions);
        }
    }

    (parsed_portion, restrictions)
}

fn parenthetical_token_slices(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut slices = Vec::new();
    let mut depth = 0u32;
    let mut start = None;
    for (idx, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LParen => {
                if depth == 0 {
                    start = Some(idx + 1);
                }
                depth = depth.saturating_add(1);
            }
            TokenKind::RParen => {
                if depth == 1
                    && let Some(start) = start.take()
                    && start <= idx
                {
                    slices.push(&tokens[start..idx]);
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    slices
}

pub(crate) fn spell_card_prefers_resolution_line_merge(builder: &CardDefinitionBuilder) -> bool {
    builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery))
}

pub(crate) fn looks_like_spell_resolution_followup_intro_lexed(tokens: &[OwnedLexToken]) -> bool {
    looks_like_delayed_next_turn_intro_lexed(tokens)
        || looks_like_reflexive_followup_intro_lexed(tokens)
}

pub(crate) fn looks_like_reflexive_followup_intro_lexed(tokens: &[OwnedLexToken]) -> bool {
    looks_like_when_one_or_more_this_way_followup_lexed(tokens)
        || looks_like_when_it_connives_this_way_followup_lexed(tokens)
        || looks_like_when_you_do_followup_lexed(tokens)
        || looks_like_if_no_one_does_followup_lexed(tokens)
        || looks_like_otherwise_followup_lexed(tokens)
}

struct ParsedSentenceSurface {
    text: String,
    tokens: Vec<OwnedLexToken>,
}

fn split_sentences_for_parse(line: &str, line_index: usize) -> Vec<ParsedSentenceSurface> {
    if let Ok(tokens) = lex_line(line, line_index) {
        let sentences = split_lexed_sentences(&tokens)
            .into_iter()
            .map(|tokens| ParsedSentenceSurface {
                text: render_token_slice(tokens).trim().to_string(),
                tokens: tokens.to_vec(),
            })
            .filter(|sentence| !sentence.text.is_empty())
            .collect::<Vec<_>>();
        if !sentences.is_empty() {
            return sentences;
        }
    }

    split_sentences_for_parse_fallback(line)
        .into_iter()
        .map(|text| ParsedSentenceSurface {
            tokens: lex_line(&text, line_index).unwrap_or_default(),
            text,
        })
        .collect()
}

fn split_sentences_for_parse_fallback(line: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0u32;
    let mut quote_depth = 0u32;

    for ch in line.chars() {
        if ch == '(' {
            paren_depth = paren_depth.saturating_add(1);
            current.push(ch);
            continue;
        }
        if ch == ')' {
            if paren_depth > 0 {
                paren_depth -= 1;
            }
            current.push(ch);
            continue;
        }
        if ch == '"' || ch == '“' || ch == '”' {
            quote_depth = if quote_depth == 0 { 1 } else { 0 };
            current.push(ch);
            continue;
        }
        if ch == '.' && paren_depth == 0 && quote_depth == 0 {
            let sentence = current.trim();
            if !sentence.is_empty() {
                sentences.push(sentence.to_string());
            }
            current.clear();
            continue;
        }
        current.push(ch);
    }

    let sentence = current.trim();
    if !sentence.is_empty() {
        sentences.push(sentence.to_string());
    }

    sentences
}

fn parse_at_trigger_intro_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        grammar::kw("at"),
        opt(grammar::kw("the")),
        alt((grammar::kw("beginning"), grammar::kw("end"))),
    )
        .void()
        .parse_next(input)
}

fn starts_with_lexed_parser<'a>(
    tokens: &'a [OwnedLexToken],
    start_idx: usize,
    parser: impl Parser<LexStream<'a>, (), ErrMode<ContextError>>,
) -> bool {
    tokens
        .get(start_idx..)
        .is_some_and(|tail| grammar::parse_prefix(tail, parser).is_some())
}

pub(crate) fn is_at_trigger_intro_lexed(tokens: &[OwnedLexToken], idx: usize) -> bool {
    starts_with_lexed_parser(tokens, idx, parse_at_trigger_intro_inner)
}

fn parse_delayed_next_turn_intro_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        grammar::kw("at"),
        opt(grammar::kw("the")),
        grammar::kw("beginning"),
        grammar::kw("of"),
        opt(grammar::kw("the")),
        opt(grammar::kw("your")),
        grammar::kw("next"),
        alt((
            grammar::phrase(&["end", "step"]),
            grammar::kw("upkeep").void(),
        )),
    )
        .void()
        .parse_next(input)
}

fn looks_like_delayed_next_turn_intro_lexed(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(tokens, parse_delayed_next_turn_intro_inner).is_some()
}

fn looks_like_when_one_or_more_this_way_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    document_shapes::parse_when_one_or_more_this_way_followup_surface(tokens).is_some()
}

fn parse_when_it_connives_this_way_followup_intro_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        alt((grammar::kw("when"), grammar::kw("whenever"))),
        grammar::kw("it"),
        alt((grammar::kw("connive"), grammar::kw("connives"))),
        grammar::kw("this"),
        grammar::kw("way"),
    )
        .void()
        .parse_next(input)
}

fn looks_like_when_it_connives_this_way_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    starts_with_lexed_parser(
        tokens,
        0,
        parse_when_it_connives_this_way_followup_intro_inner,
    )
}

fn parse_when_you_do_followup_intro_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    alt((
        grammar::phrase(&["when", "you", "do"]),
        grammar::phrase(&["whenever", "you", "do"]),
    ))
    .void()
    .parse_next(input)
}

fn looks_like_when_you_do_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    starts_with_lexed_parser(tokens, 0, parse_when_you_do_followup_intro_inner)
}

fn parse_if_no_one_does_followup_intro_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    grammar::phrase(&["if", "no", "one", "does"])
        .void()
        .parse_next(input)
}

fn looks_like_if_no_one_does_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    starts_with_lexed_parser(tokens, 0, parse_if_no_one_does_followup_intro_inner)
}

fn parse_otherwise_followup_intro_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    grammar::kw("otherwise").void().parse_next(input)
}

fn looks_like_otherwise_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    starts_with_lexed_parser(tokens, 0, parse_otherwise_followup_intro_inner)
}

fn queue_restriction(tokens: &[OwnedLexToken], pending: &mut ParsedRestrictions) -> bool {
    if let Some(parsed) = parse_activation_restriction_tokens(tokens) {
        pending.activation.push(parsed);
        true
    } else if let Some(parsed) = parse_trigger_restriction_tokens(tokens) {
        pending.trigger.push(parsed);
        true
    } else {
        false
    }
}

fn extract_parenthetical_restrictions(line: &str) -> Vec<ParsedSentenceSurface> {
    let mut restrictions = Vec::new();
    let mut paren_depth = 0u32;
    let mut start = None::<usize>;

    for (byte_idx, ch) in line.char_indices() {
        match ch {
            '(' => {
                if paren_depth == 0 {
                    start = Some(byte_idx + ch.len_utf8());
                }
                paren_depth = paren_depth.saturating_add(1);
            }
            ')' => {
                if paren_depth == 1 {
                    if let Some(start_idx) = start.take() {
                        let inside = &line[start_idx..byte_idx];
                        restrictions.extend(split_sentences_for_parse(inside, 0));
                    }
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    restrictions
}
