use crate::cards::builders::TextSpan;

use super::lexer::OwnedLexToken;
use super::lexer::TokenKind;

pub(crate) type TokInput<'a> = &'a [OwnedLexToken];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompatWordPiece {
    pub(crate) text: String,
    pub(crate) span: TextSpan,
}

fn push_normalized_words(
    slice: &str,
    base_span: TextSpan,
    in_mana_braces: bool,
    out: &mut Vec<CompatWordPiece>,
) {
    let mut buffer = String::new();
    let mut piece_start: Option<usize> = None;
    let mut piece_end = base_span.start;
    let chars: Vec<(usize, char)> = slice.char_indices().collect();

    let flush = |buffer: &mut String,
                 out: &mut Vec<CompatWordPiece>,
                 piece_start: &mut Option<usize>,
                 piece_end: &mut usize| {
        if !buffer.is_empty() {
            out.push(CompatWordPiece {
                text: std::mem::take(buffer),
                span: TextSpan {
                    line: base_span.line,
                    start: piece_start.unwrap_or(base_span.start),
                    end: *piece_end,
                },
            });
        }
        *piece_start = None;
    };

    for (idx, (rel_idx, original_ch)) in chars.iter().copied().enumerate() {
        let mut normalized_ch = original_ch;
        if normalized_ch == '−' {
            normalized_ch = '-';
        }
        let prev = if idx > 0 { chars[idx - 1].1 } else { '\0' };
        let next = if idx + 1 < chars.len() {
            chars[idx + 1].1
        } else {
            '\0'
        };
        let is_counter_char = match normalized_ch {
            '+' | '-' => next.is_ascii_digit() || next == 'x' || next == 'X',
            '/' => {
                (prev.is_ascii_digit() || prev == 'x' || prev == 'X')
                    && (next.is_ascii_digit()
                        || next == '-'
                        || next == '+'
                        || next == 'x'
                        || next == 'X')
            }
            _ => false,
        };
        let is_mana_hybrid_slash = normalized_ch == '/' && in_mana_braces;

        if normalized_ch.is_ascii_alphanumeric() || is_counter_char || is_mana_hybrid_slash {
            if piece_start.is_none() {
                piece_start = Some(base_span.start + rel_idx);
            }
            piece_end = base_span.start + rel_idx + original_ch.len_utf8();
            buffer.push(normalized_ch.to_ascii_lowercase());
            continue;
        }

        if matches!(normalized_ch, '\'' | '’' | '‘') {
            if piece_start.is_some() {
                piece_end = base_span.start + rel_idx + original_ch.len_utf8();
            }
            continue;
        }

        flush(&mut buffer, out, &mut piece_start, &mut piece_end);
    }

    flush(&mut buffer, out, &mut piece_start, &mut piece_end);
}

pub(crate) fn compat_word_pieces_for_token(token: &OwnedLexToken) -> Vec<CompatWordPiece> {
    let mut pieces = Vec::new();
    match token.kind {
        TokenKind::Word | TokenKind::Number => {
            push_normalized_words(token.parser_text(), token.span(), false, &mut pieces);
        }
        TokenKind::Tilde => pieces.push(CompatWordPiece {
            text: "this".to_string(),
            span: token.span(),
        }),
        TokenKind::ManaGroup => {
            let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
            if !inner.is_empty() {
                push_normalized_words(
                    inner,
                    TextSpan {
                        line: token.span().line,
                        start: token.span().start.saturating_add(1),
                        end: token.span().end.saturating_sub(1),
                    },
                    true,
                    &mut pieces,
                );
            }
        }
        TokenKind::Half => pieces.push(CompatWordPiece {
            text: "1/2".to_string(),
            span: token.span(),
        }),
        _ => {}
    }
    pieces
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LowercaseWordView {
    lower_words: Vec<String>,
    token_start_indices: Vec<usize>,
    token_end_indices: Vec<usize>,
}

impl LowercaseWordView {
    pub(crate) fn new(tokens: TokInput<'_>) -> Self {
        let mut lower_words = Vec::new();
        let mut token_start_indices = Vec::new();
        let mut token_end_indices = Vec::new();
        let mut token_idx = 0usize;
        while token_idx < tokens.len() {
            let token = &tokens[token_idx];
            let pieces = compat_word_pieces_for_token(token);
            if pieces.is_empty() {
                token_idx += 1;
                continue;
            }
            for piece in pieces {
                lower_words.push(piece.text);
                token_start_indices.push(token_idx);
                token_end_indices.push(token_idx + 1);
            }
            token_idx += 1;
        }
        Self {
            lower_words,
            token_start_indices,
            token_end_indices,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.lower_words.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.lower_words.len()
    }

    pub(crate) fn get(&self, idx: usize) -> Option<&str> {
        self.lower_words.get(idx).map(String::as_str)
    }

    pub(crate) fn starts_with(&self, expected: &[&str]) -> bool {
        self.slice_eq(0, expected)
    }

    pub(crate) fn slice_eq(&self, start: usize, expected: &[&str]) -> bool {
        self.lower_words
            .get(start..start.saturating_add(expected.len()))
            .is_some_and(|slice| {
                slice
                    .iter()
                    .map(String::as_str)
                    .zip(expected.iter().copied())
                    .all(|(actual, expected)| actual == expected)
            })
    }

    pub(crate) fn find_phrase_start(&self, expected: &[&str]) -> Option<usize> {
        if expected.is_empty() || self.lower_words.len() < expected.len() {
            return None;
        }
        let mut idx = 0usize;
        let last_start = self.lower_words.len() - expected.len();
        while idx <= last_start {
            if self.slice_eq(idx, expected) {
                return Some(idx);
            }
            idx += 1;
        }
        None
    }

    pub(crate) fn has_phrase(&self, expected: &[&str]) -> bool {
        self.find_phrase_start(expected).is_some()
    }

    pub(crate) fn find_word(&self, expected: &str) -> Option<usize> {
        let mut idx = 0usize;
        while idx < self.lower_words.len() {
            if self.lower_words[idx] == expected {
                return Some(idx);
            }
            idx += 1;
        }

        None
    }

    pub(crate) fn to_word_refs(&self) -> Vec<&str> {
        self.lower_words.iter().map(String::as_str).collect()
    }

    pub(crate) fn token_index_for_word_index(&self, word_idx: usize) -> Option<usize> {
        self.token_start_indices.get(word_idx).copied()
    }

    pub(crate) fn token_index_after_words(&self, word_count: usize) -> Option<usize> {
        if word_count == 0 {
            return Some(0);
        }
        if word_count > self.len() {
            return None;
        }
        self.token_end_indices.get(word_count - 1).copied()
    }
}
