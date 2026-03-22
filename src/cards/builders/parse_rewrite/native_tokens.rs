use super::lexer::OwnedLexToken;

pub(crate) type TokInput<'a> = &'a [OwnedLexToken];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LowercaseWordView {
    lower_words: Vec<String>,
    token_indices: Vec<usize>,
}

impl LowercaseWordView {
    pub(crate) fn new(tokens: TokInput<'_>) -> Self {
        let mut lower_words = Vec::new();
        let mut token_indices = Vec::new();
        for (token_idx, token) in tokens.iter().enumerate() {
            let Some(word) = token.as_word() else {
                continue;
            };
            lower_words.push(word.to_ascii_lowercase());
            token_indices.push(token_idx);
        }
        Self {
            lower_words,
            token_indices,
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
        self.token_indices.get(word_idx).copied()
    }
}
