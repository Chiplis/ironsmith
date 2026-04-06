use crate::cards::builders::{CardDefinitionBuilder, ParsedRestrictions};
use crate::types::CardType;
use winnow::combinator::{alt, opt};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use super::activation_and_restrictions::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
use super::grammar::primitives as grammar;
use super::lexer::{LexStream, OwnedLexToken, lex_line, render_token_slice, split_lexed_sentences};

pub(crate) fn split_text_for_parse(
    raw_text: &str,
    normalized_text: &str,
    line_index: usize,
) -> (Vec<String>, ParsedRestrictions) {
    let line_sentences = split_sentences_for_parse(normalized_text, line_index);
    let mut restrictions = ParsedRestrictions::default();
    let mut parsed_portion = Vec::new();
    for sentence in line_sentences {
        if sentence.is_empty() {
            continue;
        }

        if queue_restriction(&sentence, line_index, &mut restrictions) {
            continue;
        }

        parsed_portion.push(sentence);
    }

    for restriction in extract_parenthetical_restrictions(raw_text) {
        let _ = queue_restriction(&restriction, line_index, &mut restrictions);
    }

    (parsed_portion, restrictions)
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
        || looks_like_when_you_do_followup_lexed(tokens)
        || looks_like_otherwise_followup_lexed(tokens)
}

fn split_sentences_for_parse(line: &str, _line_index: usize) -> Vec<String> {
    if let Ok(tokens) = lex_line(line, _line_index) {
        let sentences = split_lexed_sentences(&tokens)
            .into_iter()
            .map(render_token_slice)
            .map(|sentence| sentence.trim().to_string())
            .filter(|sentence| !sentence.is_empty())
            .collect::<Vec<_>>();
        if !sentences.is_empty() {
            return sentences;
        }
    }

    split_sentences_for_parse_fallback(line)
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

pub(crate) fn is_at_trigger_intro(tokens: &[OwnedLexToken], idx: usize) -> bool {
    starts_with_lexed_parser(tokens, idx, parse_at_trigger_intro_inner)
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

fn parse_when_one_or_more_followup_head_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        alt((grammar::kw("when"), grammar::kw("whenever"))),
        grammar::kw("one"),
        grammar::kw("or"),
        grammar::kw("more"),
    )
        .void()
        .parse_next(input)
}

fn token_slice_contains_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    tokens
        .iter()
        .enumerate()
        .any(|(idx, _)| grammar::parse_prefix(&tokens[idx..], grammar::phrase(phrase)).is_some())
}

fn looks_like_when_one_or_more_this_way_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    starts_with_lexed_parser(tokens, 0, parse_when_one_or_more_followup_head_inner)
        && token_slice_contains_phrase(tokens, &["this", "way"])
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

fn parse_otherwise_followup_intro_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    grammar::kw("otherwise").void().parse_next(input)
}

fn looks_like_otherwise_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    starts_with_lexed_parser(tokens, 0, parse_otherwise_followup_intro_inner)
}

fn queue_restriction(
    restriction: &str,
    line_index: usize,
    pending: &mut ParsedRestrictions,
) -> bool {
    let normalized = normalize_restriction_text(restriction);
    if normalized.is_empty() {
        return false;
    }

    let tokens = lex_line(&normalized, line_index).unwrap_or_default();
    if is_activate_only_restriction_sentence_lexed(&tokens) {
        pending.activation.push(normalized);
        true
    } else if is_trigger_only_restriction_sentence_lexed(&tokens) {
        pending.trigger.push(normalized);
        true
    } else {
        false
    }
}

fn extract_parenthetical_restrictions(line: &str) -> Vec<String> {
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
                        for sentence in split_sentences_for_parse(inside, 0) {
                            restrictions.push(sentence);
                        }
                    }
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    restrictions
        .into_iter()
        .map(|restriction| normalize_restriction_text(&restriction))
        .filter(|restriction| !restriction.is_empty())
        .collect()
}

fn normalize_restriction_text(text: &str) -> String {
    text.trim().trim_end_matches('.').trim().to_string()
}
