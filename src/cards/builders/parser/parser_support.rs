use crate::cards::builders::{CardDefinitionBuilder, ParsedRestrictions};
use crate::types::CardType;

use super::activation_and_restrictions::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
use super::lexer::{OwnedLexToken, lex_line};

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
        || looks_like_when_one_or_more_this_way_followup_lexed(tokens)
        || looks_like_when_you_do_followup_lexed(tokens)
        || looks_like_otherwise_followup_lexed(tokens)
}

fn split_sentences_for_parse(line: &str, _line_index: usize) -> Vec<String> {
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

pub(crate) fn is_at_trigger_intro(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }

    let second = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
    let third = tokens.get(idx + 2).and_then(OwnedLexToken::as_word);
    matches!(
        (second, third),
        (Some("beginning"), _)
            | (Some("end"), _)
            | (Some("the"), Some("beginning"))
            | (Some("the"), Some("end"))
    )
}

pub(crate) fn is_at_trigger_intro_lexed(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }

    let second = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
    let third = tokens.get(idx + 2).and_then(OwnedLexToken::as_word);
    matches!(
        (second, third),
        (Some("beginning"), _)
            | (Some("end"), _)
            | (Some("the"), Some("beginning"))
            | (Some("the"), Some("end"))
    )
}

fn looks_like_delayed_next_turn_intro_lexed(tokens: &[OwnedLexToken]) -> bool {
    let mut idx = 0usize;
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("beginning"))
    {
        return false;
    }
    idx += 1;
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return false;
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("your")) {
        idx += 1;
    }

    if !tokens.get(idx).is_some_and(|token| token.is_word("next")) {
        return false;
    }

    if tokens
        .get(idx + 1)
        .is_some_and(|token| token.is_word("end"))
        && tokens
            .get(idx + 2)
            .is_some_and(|token| token.is_word("step"))
    {
        return true;
    }

    tokens
        .get(idx + 1)
        .is_some_and(|token| token.is_word("upkeep"))
}

fn looks_like_when_one_or_more_this_way_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    (starts_with_lexed_words(tokens, &["when", "one", "or", "more"])
        || starts_with_lexed_words(tokens, &["whenever", "one", "or", "more"]))
        && tokens
            .windows(2)
            .any(|window| window[0].is_word("this") && window[1].is_word("way"))
}

fn looks_like_when_you_do_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    starts_with_lexed_words(tokens, &["when", "you", "do"])
        || starts_with_lexed_words(tokens, &["whenever", "you", "do"])
}

fn looks_like_otherwise_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
}

fn starts_with_lexed_words(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    tokens.len() >= expected.len()
        && tokens
            .iter()
            .zip(expected.iter())
            .all(|(token, expected)| token.is_word(expected))
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
