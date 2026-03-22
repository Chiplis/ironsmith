use super::lexer::OwnedLexToken;
use super::lexer::TokenKind;

pub(crate) type TokInput<'a> = &'a [OwnedLexToken];

fn push_normalized_words(slice: &str, in_mana_braces: bool, out: &mut Vec<String>) {
    let mut buffer = String::new();
    let chars: Vec<(usize, char)> = slice.char_indices().collect();

    let flush = |buffer: &mut String, out: &mut Vec<String>| {
        if !buffer.is_empty() {
            out.push(std::mem::take(buffer));
        }
    };

    for (idx, (_, mut ch)) in chars.iter().copied().enumerate() {
        if ch == '−' {
            ch = '-';
        }
        let prev = if idx > 0 { chars[idx - 1].1 } else { '\0' };
        let next = if idx + 1 < chars.len() {
            chars[idx + 1].1
        } else {
            '\0'
        };
        let is_counter_char = match ch {
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
        let is_mana_hybrid_slash = ch == '/' && in_mana_braces;

        if ch.is_ascii_alphanumeric() || is_counter_char || is_mana_hybrid_slash {
            buffer.push(ch.to_ascii_lowercase());
            continue;
        }

        if matches!(ch, '\'' | '’' | '‘') {
            continue;
        }

        flush(&mut buffer, out);
    }

    flush(&mut buffer, out);
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
            let (token_start, token_end_exclusive, pieces) = match token.kind {
                TokenKind::Word => {
                    let mut pieces = Vec::new();
                    push_normalized_words(token.slice.as_str(), false, &mut pieces);
                    (token_idx, token_idx + 1, pieces)
                }
                TokenKind::Tilde => (token_idx, token_idx + 1, vec!["this".to_string()]),
                TokenKind::ManaGroup => {
                    let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
                    let mut pieces = Vec::new();
                    if !inner.is_empty() {
                        push_normalized_words(inner, true, &mut pieces);
                    }
                    (token_idx, token_idx + 1, pieces)
                }
                TokenKind::Dash
                    if tokens
                        .get(token_idx + 1)
                        .is_some_and(|next| next.kind == TokenKind::Word)
                        && token.span.end == tokens[token_idx + 1].span.start =>
                {
                    let next = &tokens[token_idx + 1];
                    let mut pieces = Vec::new();
                    let combined = format!("-{}", next.slice);
                    push_normalized_words(combined.as_str(), false, &mut pieces);
                    token_idx += 1;
                    (token_idx - 1, token_idx + 1, pieces)
                }
                TokenKind::Half => (token_idx, token_idx + 1, vec!["1/2".to_string()]),
                _ => {
                    token_idx += 1;
                    continue;
                }
            };
            for piece in pieces {
                lower_words.push(piece);
                token_start_indices.push(token_start);
                token_end_indices.push(token_end_exclusive);
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

    pub(crate) fn first(&self) -> Option<&str> {
        self.get(0)
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

    pub(crate) fn contains_sequence(&self, expected: &[&str]) -> bool {
        if expected.is_empty() || self.lower_words.len() < expected.len() {
            return false;
        }
        (0..=self.lower_words.len() - expected.len()).any(|idx| self.slice_eq(idx, expected))
    }

    pub(crate) fn find(&self, expected: &str) -> Option<usize> {
        self.lower_words.iter().position(|word| word == expected)
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
