use super::super::permission_shapes;
use crate::lexer::{OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy)]
pub struct TriggerClausePattern<'p> {
    exact: Option<&'p [&'p str]>,
    exact_any: &'p [&'p [&'p str]],
    prefix: Option<&'p [&'p str]>,
    prefix_any: &'p [&'p [&'p str]],
    suffix: Option<&'p [&'p str]>,
    suffix_any: &'p [&'p [&'p str]],
    contains_phrases: &'p [&'p [&'p str]],
    contains_any_phrases: &'p [&'p [&'p [&'p str]]],
    contains_words: &'p [&'p str],
    contains_any_words: &'p [&'p [&'p str]],
}

impl<'p> TriggerClausePattern<'p> {
    pub const fn new() -> Self {
        Self {
            exact: None,
            exact_any: &[],
            prefix: None,
            prefix_any: &[],
            suffix: None,
            suffix_any: &[],
            contains_phrases: &[],
            contains_any_phrases: &[],
            contains_words: &[],
            contains_any_words: &[],
        }
    }

    pub const fn exact(mut self, phrase: &'p [&'p str]) -> Self {
        self.exact = Some(phrase);
        self
    }

    pub const fn exact_any(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.exact_any = phrases;
        self
    }

    pub const fn prefix(mut self, phrase: &'p [&'p str]) -> Self {
        self.prefix = Some(phrase);
        self
    }

    pub const fn prefix_any(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.prefix_any = phrases;
        self
    }

    pub const fn suffix(mut self, phrase: &'p [&'p str]) -> Self {
        self.suffix = Some(phrase);
        self
    }

    pub const fn suffix_any(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.suffix_any = phrases;
        self
    }

    pub const fn contains_phrases(mut self, phrases: &'p [&'p [&'p str]]) -> Self {
        self.contains_phrases = phrases;
        self
    }

    pub const fn contains_any_phrases(mut self, phrases: &'p [&'p [&'p [&'p str]]]) -> Self {
        self.contains_any_phrases = phrases;
        self
    }

    pub const fn contains_words(mut self, words: &'p [&'p str]) -> Self {
        self.contains_words = words;
        self
    }

    pub const fn contains_any_words(mut self, word_sets: &'p [&'p [&'p str]]) -> Self {
        self.contains_any_words = word_sets;
        self
    }
}

pub fn parse_trigger_surface_words(words: &[&str], pattern: TriggerClausePattern<'_>) -> bool {
    if let Some(exact) = pattern.exact
        && !permission_shapes::exact_words(words, exact)
    {
        return false;
    }
    if !pattern.exact_any.is_empty()
        && !pattern
            .exact_any
            .iter()
            .any(|expected| permission_shapes::exact_words(words, expected))
    {
        return false;
    }
    if let Some(prefix) = pattern.prefix
        && !permission_shapes::prefix_words(words, prefix)
    {
        return false;
    }
    if !pattern.prefix_any.is_empty()
        && !pattern
            .prefix_any
            .iter()
            .any(|expected| permission_shapes::prefix_words(words, expected))
    {
        return false;
    }
    if let Some(suffix) = pattern.suffix
        && !permission_shapes::suffix_words(words, suffix)
    {
        return false;
    }
    if !pattern.suffix_any.is_empty()
        && !pattern
            .suffix_any
            .iter()
            .any(|expected| permission_shapes::suffix_words(words, expected))
    {
        return false;
    }
    if pattern
        .contains_phrases
        .iter()
        .any(|phrase| permission_shapes::find_words(words, phrase).is_none())
    {
        return false;
    }
    if pattern.contains_any_phrases.iter().any(|alternatives| {
        !alternatives
            .iter()
            .any(|phrase| permission_shapes::find_words(words, phrase).is_some())
    }) {
        return false;
    }
    if pattern
        .contains_words
        .iter()
        .any(|word| permission_shapes::find_words(words, &[*word]).is_none())
    {
        return false;
    }
    if pattern.contains_any_words.iter().any(|alternatives| {
        !alternatives
            .iter()
            .any(|word| permission_shapes::find_words(words, &[*word]).is_some())
    }) {
        return false;
    }
    true
}

pub fn parse_trigger_surface_tokens(
    tokens: &[OwnedLexToken],
    pattern: TriggerClausePattern<'_>,
) -> bool {
    parse_trigger_surface_words(&TokenWordView::new(tokens).word_refs(), pattern)
}

pub fn find_trigger_surface_window(
    words: &[&str],
    width: usize,
    pattern: TriggerClausePattern<'_>,
) -> Option<usize> {
    if width == 0 || width > words.len() {
        return None;
    }
    let mut start = 0usize;
    while start + width <= words.len() {
        if parse_trigger_surface_words(&words[start..start + width], pattern) {
            return Some(start);
        }
        start += 1;
    }
    None
}

macro_rules! trigger_clause_pattern {
    (exact $phrase:expr) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .exact($phrase)
    };
    (exact_any $phrases:expr) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .exact_any($phrases)
    };
    (prefix $prefix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .prefix($prefix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix_any $prefixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .prefix_any($prefixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix $prefix:expr; suffix_any $suffixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .prefix($prefix)
            .suffix_any($suffixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix_any $prefixes:expr; suffix $suffix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .prefix_any($prefixes)
            .suffix($suffix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix_any $prefixes:expr; suffix_any $suffixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .prefix_any($prefixes)
            .suffix_any($suffixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (prefix $prefix:expr; suffix $suffix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .prefix($prefix)
            .suffix($suffix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (suffix $suffix:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .suffix($suffix)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (suffix_any $suffixes:expr $(; contains_phrases $contains_phrases:expr)? $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .suffix_any($suffixes)
            $(.contains_phrases($contains_phrases))?
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (contains_phrases $contains_phrases:expr $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .contains_phrases($contains_phrases)
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (contains_any_phrases $contains_any_phrases:expr $(; contains_words $contains_words:expr)? $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .contains_any_phrases($contains_any_phrases)
            $(.contains_words($contains_words))?
            $(.contains_any_words($contains_any_words))?
    };
    (contains_words $contains_words:expr $(; contains_any_words $contains_any_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .contains_words($contains_words)
            $(.contains_any_words($contains_any_words))?
    };
    (contains_any_words $contains_any_words:expr $(; contains_words $contains_words:expr)?) => {
        $crate::grammar::trigger_clauses::TriggerClausePattern::new()
            .contains_any_words($contains_any_words)
            $(.contains_words($contains_words))?
    };
}

pub(crate) use trigger_clause_pattern;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_surface_patterns_are_winnow_backed() {
        let pattern = TriggerClausePattern::new()
            .prefix(&["you", "cast"])
            .contains_words(&["spell"]);
        assert!(parse_trigger_surface_words(
            &["you", "cast", "a", "spell"],
            pattern
        ));
        assert!(!parse_trigger_surface_words(
            &["you", "cast", "a", "land"],
            pattern
        ));
    }
}
