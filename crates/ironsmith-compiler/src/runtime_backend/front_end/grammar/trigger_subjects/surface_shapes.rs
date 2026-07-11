use super::super::permission_shapes;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TriggerSurface<'p> {
    exact: Option<&'p [&'p str]>,
    exact_any: &'p [&'p [&'p str]],
    prefix: Option<&'p [&'p str]>,
    prefix_any: &'p [&'p [&'p str]],
    suffix: Option<&'p [&'p str]>,
    suffix_any: &'p [&'p [&'p str]],
    required_sequences: &'p [&'p [&'p str]],
    required_sequence_sets: &'p [&'p [&'p [&'p str]]],
    required_words: &'p [&'p str],
    required_word_sets: &'p [&'p [&'p str]],
}

impl<'p> TriggerSurface<'p> {
    pub(crate) const fn new() -> Self {
        Self {
            exact: None,
            exact_any: &[],
            prefix: None,
            prefix_any: &[],
            suffix: None,
            suffix_any: &[],
            required_sequences: &[],
            required_sequence_sets: &[],
            required_words: &[],
            required_word_sets: &[],
        }
    }

    pub(crate) const fn exact(mut self, expected: &'p [&'p str]) -> Self {
        self.exact = Some(expected);
        self
    }

    pub(crate) const fn exact_any(mut self, expected: &'p [&'p [&'p str]]) -> Self {
        self.exact_any = expected;
        self
    }

    pub(crate) const fn prefix(mut self, expected: &'p [&'p str]) -> Self {
        self.prefix = Some(expected);
        self
    }

    pub(crate) const fn prefix_any(mut self, expected: &'p [&'p [&'p str]]) -> Self {
        self.prefix_any = expected;
        self
    }

    pub(crate) const fn suffix(mut self, expected: &'p [&'p str]) -> Self {
        self.suffix = Some(expected);
        self
    }

    pub(crate) const fn suffix_any(mut self, expected: &'p [&'p [&'p str]]) -> Self {
        self.suffix_any = expected;
        self
    }

    pub(crate) const fn contains_phrases(mut self, expected: &'p [&'p [&'p str]]) -> Self {
        self.required_sequences = expected;
        self
    }

    pub(crate) const fn contains_any_phrases(
        mut self,
        expected: &'p [&'p [&'p [&'p str]]],
    ) -> Self {
        self.required_sequence_sets = expected;
        self
    }

    pub(crate) const fn contains_words(mut self, expected: &'p [&'p str]) -> Self {
        self.required_words = expected;
        self
    }

    pub(crate) const fn contains_any_words(mut self, expected: &'p [&'p [&'p str]]) -> Self {
        self.required_word_sets = expected;
        self
    }

    pub(crate) fn accepts_words(self, words: &[&str]) -> bool {
        if let Some(expected) = self.exact
            && !permission_shapes::exact_words(words, expected)
        {
            return false;
        }
        if !self.exact_any.is_empty()
            && !self
                .exact_any
                .iter()
                .any(|expected| permission_shapes::exact_words(words, expected))
        {
            return false;
        }
        if let Some(expected) = self.prefix
            && !permission_shapes::prefix_words(words, expected)
        {
            return false;
        }
        if !self.prefix_any.is_empty()
            && !self
                .prefix_any
                .iter()
                .any(|expected| permission_shapes::prefix_words(words, expected))
        {
            return false;
        }
        if let Some(expected) = self.suffix
            && !permission_shapes::suffix_words(words, expected)
        {
            return false;
        }
        if !self.suffix_any.is_empty()
            && !self
                .suffix_any
                .iter()
                .any(|expected| permission_shapes::suffix_words(words, expected))
        {
            return false;
        }
        if self
            .required_sequences
            .iter()
            .any(|expected| permission_shapes::find_words(words, expected).is_none())
        {
            return false;
        }
        if self.required_sequence_sets.iter().any(|alternatives| {
            !alternatives
                .iter()
                .any(|expected| permission_shapes::find_words(words, expected).is_some())
        }) {
            return false;
        }
        if self
            .required_words
            .iter()
            .any(|word| permission_shapes::find_words(words, &[*word]).is_none())
        {
            return false;
        }
        if self.required_word_sets.iter().any(|alternatives| {
            !alternatives
                .iter()
                .any(|word| permission_shapes::find_words(words, &[*word]).is_some())
        }) {
            return false;
        }
        true
    }

    pub(crate) fn locate_window(self, words: &[&str], width: usize) -> Option<usize> {
        if width == 0 || width > words.len() {
            return None;
        }
        for start in 0..=words.len() - width {
            if self.accepts_words(&words[start..start + width]) {
                return Some(start);
            }
        }
        None
    }

    pub(crate) fn find_word(self, words: &[&str]) -> Option<usize> {
        for (index, word) in words.iter().enumerate() {
            if self.accepts_words(&[*word]) {
                return Some(index);
            }
        }
        None
    }
}

macro_rules! trigger_surface {
    (exact $expected:expr) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new().exact($expected)
    };
    (exact_any $expected:expr) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .exact_any($expected)
    };
    (prefix $expected:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .prefix($expected)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (prefix_any $expected:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .prefix_any($expected)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (prefix $expected:expr; suffix $suffix:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .prefix($expected)
            .suffix($suffix)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (prefix $expected:expr; suffix_any $suffixes:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .prefix($expected)
            .suffix_any($suffixes)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (prefix_any $expected:expr; suffix $suffix:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .prefix_any($expected)
            .suffix($suffix)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (prefix_any $expected:expr; suffix_any $suffixes:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .prefix_any($expected)
            .suffix_any($suffixes)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (suffix $expected:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .suffix($expected)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (suffix_any $expected:expr $(; contains_phrases $sequences:expr)? $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .suffix_any($expected)
            $(.contains_phrases($sequences))?
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (contains_phrases $expected:expr $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .contains_phrases($expected)
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (contains_any_phrases $expected:expr $(; contains_words $words:expr)? $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .contains_any_phrases($expected)
            $(.contains_words($words))?
            $(.contains_any_words($word_sets))?
    };
    (contains_words $expected:expr $(; contains_any_words $word_sets:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .contains_words($expected)
            $(.contains_any_words($word_sets))?
    };
    (contains_any_words $expected:expr $(; contains_words $words:expr)?) => {
        $crate::runtime_backend::grammar::trigger_subjects::TriggerSurface::new()
            .contains_any_words($expected)
            $(.contains_words($words))?
    };
}

pub(crate) use trigger_surface;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_composed_trigger_surfaces() {
        let shape = trigger_surface!(prefix & ["a", "spell"]; contains_words & ["turn"]);
        assert!(shape.accepts_words(&["a", "spell", "during", "your", "turn"]));
        assert!(!shape.accepts_words(&["a", "spell"]));
    }
}
