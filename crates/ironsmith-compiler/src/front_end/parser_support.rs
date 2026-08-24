use super::lexer::{
    OwnedLexToken, TokenWordView, lex_line, render_token_slice, split_lexed_sentences,
};
use crate::model::{ParsedRestrictions, RestrictionBucket};

/// Result of splitting a line-oriented oracle text fragment for parsing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SentenceSplitResult {
    pub sentences: Vec<String>,
    pub parenthetical_sentences: Vec<String>,
}

impl SentenceSplitResult {
    pub fn all_sentences(&self) -> Vec<String> {
        self.sentences
            .iter()
            .chain(self.parenthetical_sentences.iter())
            .cloned()
            .collect()
    }
}

pub fn split_text_for_parse(
    raw_text: &str,
    normalized_text: &str,
    line_index: usize,
) -> SentenceSplitResult {
    SentenceSplitResult {
        sentences: split_sentences_for_parse(normalized_text, line_index)
            .into_iter()
            .filter(|sentence| !is_standalone_parenthetical_sentence(sentence))
            .collect(),
        parenthetical_sentences: extract_parenthetical_sentences(raw_text),
    }
}

pub fn split_text_for_parse_with_restrictions(
    raw_text: &str,
    normalized_text: &str,
    line_index: usize,
    mut classify: impl FnMut(&str, &[OwnedLexToken]) -> Option<RestrictionBucket>,
) -> (Vec<String>, ParsedRestrictions) {
    let split = split_text_for_parse(raw_text, normalized_text, line_index);
    let mut parsed_portion = Vec::new();
    let mut restrictions = ParsedRestrictions::default();

    for sentence in split.sentences {
        if sentence.is_empty() {
            continue;
        }

        let tokens = lex_line(&sentence, line_index).unwrap_or_default();
        if let Some(bucket) = classify(&sentence, &tokens) {
            restrictions.push(bucket, normalize_restriction_text(&sentence));
        } else {
            parsed_portion.push(sentence);
        }
    }

    for sentence in split.parenthetical_sentences {
        if sentence.is_empty() {
            continue;
        }

        let normalized = normalize_restriction_text(&sentence);
        if normalized.is_empty() {
            continue;
        }

        let tokens = lex_line(&normalized, line_index).unwrap_or_default();
        if let Some(bucket) = classify(&normalized, &tokens) {
            restrictions.push(bucket, normalized);
        }
    }

    (parsed_portion, restrictions)
}

pub fn split_sentences_for_parse(line: &str, line_index: usize) -> Vec<String> {
    if let Ok(tokens) = lex_line(line, line_index) {
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

pub fn split_sentences_for_parse_fallback(line: &str) -> Vec<String> {
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
            paren_depth = paren_depth.saturating_sub(1);
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

pub fn normalize_restriction_text(text: &str) -> String {
    text.trim().trim_end_matches('.').trim().to_string()
}

fn is_standalone_parenthetical_sentence(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('(') && trimmed.ends_with(')')
}

pub fn extract_parenthetical_sentences(line: &str) -> Vec<String> {
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
                if paren_depth == 1
                    && let Some(start_idx) = start.take()
                {
                    let inside = &line[start_idx..byte_idx];
                    for sentence in split_sentences_for_parse(inside, 0) {
                        let normalized = normalize_restriction_text(&sentence);
                        if !normalized.is_empty() {
                            restrictions.push(normalized);
                        }
                    }
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    restrictions
}

pub fn is_at_trigger_intro_lexed(tokens: &[OwnedLexToken], idx: usize) -> bool {
    let words = TokenWordView::new(tokens.get(idx..).unwrap_or_default());
    words.parses_any_prefix(&[
        &["at", "beginning"],
        &["at", "the", "beginning"],
        &["at", "end"],
        &["at", "the", "end"],
    ])
}

pub fn looks_like_spell_resolution_followup_intro_lexed(tokens: &[OwnedLexToken]) -> bool {
    looks_like_delayed_next_turn_intro_lexed(tokens)
        || looks_like_reflexive_followup_intro_lexed(tokens)
}

pub fn looks_like_reflexive_followup_intro_lexed(tokens: &[OwnedLexToken]) -> bool {
    looks_like_when_one_or_more_this_way_followup_lexed(tokens)
        || looks_like_when_you_pay_this_cost_followup_lexed(tokens)
        || looks_like_when_you_do_followup_lexed(tokens)
        || looks_like_if_no_one_does_followup_lexed(tokens)
        || looks_like_otherwise_followup_lexed(tokens)
}

fn looks_like_when_you_pay_this_cost_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.parses_prefix(&["when", "you", "pay", "this", "cost"])
}

fn looks_like_delayed_next_turn_intro_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.parses_any_prefix(&[
        &["at", "beginning", "of", "next", "end", "step"],
        &["at", "the", "beginning", "of", "next", "end", "step"],
        &[
            "at",
            "the",
            "beginning",
            "of",
            "your",
            "next",
            "end",
            "step",
        ],
        &["at", "the", "beginning", "of", "your", "next", "upkeep"],
        &["at", "beginning", "of", "your", "next", "upkeep"],
    ])
}

fn looks_like_when_one_or_more_this_way_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.parses_any_prefix(&[
        &["when", "one", "or", "more"],
        &["whenever", "one", "or", "more"],
    ]) && words.parses_phrase_anywhere(&["this", "way"])
}

fn looks_like_when_you_do_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.parses_any_prefix(&[&["when", "you", "do"], &["whenever", "you", "do"]])
}

fn looks_like_if_no_one_does_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    TokenWordView::new(tokens).parses_prefix(&["if", "no", "one", "does"])
}

fn looks_like_otherwise_followup_lexed(tokens: &[OwnedLexToken]) -> bool {
    TokenWordView::new(tokens).parses_prefix(&["otherwise"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front_end::lexer::lex_line;
    use crate::model::RestrictionBucket;

    #[test]
    fn split_text_for_parse_extracts_parenthetical_sentences_separately() {
        let split = split_text_for_parse(
            "Draw a card. (Activate only as a sorcery.)",
            "Draw a card. (Activate only as a sorcery.)",
            0,
        );

        assert_eq!(split.sentences, vec!["Draw a card"]);
        assert_eq!(
            split.parenthetical_sentences,
            vec!["Activate only as a sorcery"]
        );
    }

    #[test]
    fn fallback_sentence_splitting_ignores_quoted_periods() {
        let sentences = split_sentences_for_parse_fallback(
            "Gain \"Draw a card.\" Then discard a card. Untap this creature.",
        );

        assert_eq!(
            sentences,
            vec![
                "Gain \"Draw a card.\" Then discard a card".to_string(),
                "Untap this creature".to_string()
            ]
        );
    }

    #[test]
    fn at_trigger_intro_recognizes_beginning_and_end_forms() {
        let beginning = lex_line("At the beginning of your upkeep", 0).expect("lex");
        let ending = lex_line("At end of combat", 0).expect("lex");

        assert!(is_at_trigger_intro_lexed(&beginning, 0));
        assert!(is_at_trigger_intro_lexed(&ending, 0));
    }

    #[test]
    fn followup_intro_detection_covers_reflexive_and_next_turn_cases() {
        let delayed = lex_line("At the beginning of your next end step", 0).expect("lex");
        let reflexive = lex_line("When you do, draw a card", 0).expect("lex");
        let declined = lex_line("If no one does, draw a card", 0).expect("lex");
        let otherwise = lex_line("Otherwise, sacrifice this creature", 0).expect("lex");

        assert!(looks_like_spell_resolution_followup_intro_lexed(&delayed));
        assert!(looks_like_reflexive_followup_intro_lexed(&reflexive));
        assert!(looks_like_reflexive_followup_intro_lexed(&declined));
        assert!(looks_like_reflexive_followup_intro_lexed(&otherwise));
    }

    #[test]
    fn split_text_for_parse_with_restrictions_buckets_classified_sentences() {
        let (parsed, restrictions) = split_text_for_parse_with_restrictions(
            "Draw a card. Activate only as a sorcery. (This ability triggers only once each turn.)",
            "Draw a card. Activate only as a sorcery. (This ability triggers only once each turn.)",
            0,
            |sentence, _tokens| {
                if sentence.starts_with("Activate only") {
                    Some(RestrictionBucket::Activation)
                } else if sentence.starts_with("This ability triggers only") {
                    Some(RestrictionBucket::Trigger)
                } else {
                    None
                }
            },
        );

        assert_eq!(parsed, vec!["Draw a card"]);
        assert_eq!(restrictions.activation, vec!["Activate only as a sorcery"]);
        assert_eq!(
            restrictions.trigger,
            vec!["This ability triggers only once each turn"]
        );
    }
}
