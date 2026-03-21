use crate::cards::builders::{
    CardDefinitionBuilder, ParsedRestrictions, Token, parse_if_result_predicate,
};
use crate::types::CardType;

use super::ported_activation_and_restrictions::{
    is_activate_only_restriction_sentence, is_trigger_only_restriction_sentence,
};
use super::util::tokenize_line;

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

pub(crate) fn looks_like_spell_resolution_followup_intro(tokens: &[Token]) -> bool {
    looks_like_delayed_next_turn_intro(tokens)
        || looks_like_when_one_or_more_this_way_followup(tokens)
        || looks_like_when_you_do_followup(tokens)
        || looks_like_if_result_followup(tokens)
        || looks_like_otherwise_followup(tokens)
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

fn sentence_starts_with_trigger_intro(sentence: &str, line_index: usize) -> bool {
    let tokens = tokenize_line(sentence, line_index);
    if looks_like_delayed_next_turn_intro(&tokens) {
        return false;
    }
    if looks_like_when_one_or_more_this_way_followup(&tokens) {
        return false;
    }
    if looks_like_when_you_do_followup(&tokens) {
        return false;
    }
    tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        || is_at_trigger_intro(&tokens, 0)
}

pub(crate) fn is_at_trigger_intro(tokens: &[Token], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }

    let second = tokens.get(idx + 1).and_then(Token::as_word);
    let third = tokens.get(idx + 2).and_then(Token::as_word);
    matches!(
        (second, third),
        (Some("beginning"), _)
            | (Some("end"), _)
            | (Some("the"), Some("beginning"))
            | (Some("the"), Some("end"))
    )
}

fn looks_like_delayed_next_turn_intro(tokens: &[Token]) -> bool {
    let mut idx = 0usize;
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if !tokens.get(idx).is_some_and(|token| token.is_word("beginning")) {
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

    if tokens.get(idx + 1).is_some_and(|token| token.is_word("end"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("step"))
    {
        return true;
    }

    tokens.get(idx + 1).is_some_and(|token| token.is_word("upkeep"))
}

fn looks_like_when_one_or_more_this_way_followup(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    (clause_words.starts_with(&["when", "one", "or", "more"])
        || clause_words.starts_with(&["whenever", "one", "or", "more"]))
        && clause_words.windows(2).any(|window| window == ["this", "way"])
}

fn looks_like_when_you_do_followup(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    clause_words.starts_with(&["when", "you", "do"])
        || clause_words.starts_with(&["whenever", "you", "do"])
}

fn looks_like_if_result_followup(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    if !first.is_word("if") {
        return false;
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .unwrap_or(tokens.len());
    if comma_idx <= 1 {
        return false;
    }

    parse_if_result_predicate(&tokens[1..comma_idx]).is_some()
}

fn looks_like_otherwise_followup(tokens: &[Token]) -> bool {
    tokens.first().is_some_and(|token| token.is_word("otherwise"))
}

#[allow(dead_code)]
fn split_trigger_sentence_chunks(sentences: &[String], line_index: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_starts_with_trigger = false;

    for sentence in sentences {
        let sentence_starts_with_trigger = sentence_starts_with_trigger_intro(sentence, line_index);
        if !current.is_empty() && current_starts_with_trigger && sentence_starts_with_trigger {
            chunks.push(current.join(". "));
            current.clear();
            current_starts_with_trigger = false;
        }
        if current.is_empty() {
            current_starts_with_trigger = sentence_starts_with_trigger;
        }
        current.push(sentence.clone());
    }

    if !current.is_empty() {
        chunks.push(current.join(". "));
    }

    chunks
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

    let tokens = tokenize_line(&normalized, line_index);
    if is_activate_only_restriction_sentence(&tokens) {
        pending.activation.push(normalized);
        true
    } else if is_trigger_only_restriction_sentence(&tokens) {
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

fn words(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().filter_map(Token::as_word).collect()
}
