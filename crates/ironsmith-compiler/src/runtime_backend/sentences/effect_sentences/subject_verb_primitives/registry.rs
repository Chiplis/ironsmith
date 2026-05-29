use super::*;
use crate::parse_trace;

pub(super) const FOR_EACH_PLAYER_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["each", "player"],
    &["each", "players"],
];
pub(super) const EACH_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
    &["each", "opponent"],
    &["each", "opponents"],
];
pub(super) const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"]];
pub(super) const CHOOSE_ALL_OR_PUT_ALL_PREFIXES: &[&[&str]] =
    &[&["choose", "all"], &["put", "all"]];
pub(super) const UP_TO_PREFIXES: &[&[&str]] = &[&["up", "to"]];
pub(super) const ANY_NUMBER_OF_PREFIXES: &[&[&str]] = &[&["any", "number", "of"]];
pub(super) const CHOOSE_ALL_PREFIXES: &[&[&str]] = &[&["choose", "all"]];
pub(super) const THAT_PREFIXES: &[&[&str]] = &[&["that"]];
pub(super) const MECHANIC_MARKER_PREFIXES: &[&[&str]] = &[
    &["you", "choose", "one", "of", "them"],
    &[
        "you", "may", "put", "a", "land", "card", "from", "among", "them", "into", "your", "hand",
    ],
    &["stand", "and", "fight"],
    &["it", "doesnt", "untap", "during"],
];
pub(crate) type SubjectVerbPrimitiveParser =
    for<'a> fn(SubjectVerbPrimitiveClause<'a>) -> Result<Option<Vec<EffectAst>>, CardTextError>;

pub(super) type SubjectVerbPrimitiveNormalizedWords<'a> = TokenWordView<'a>;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SubjectVerbPrimitiveClause<'a> {
    tokens: &'a [OwnedLexToken],
}

#[allow(dead_code)]
impl<'a> SubjectVerbPrimitiveClause<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self { tokens }
    }

    fn lexed(self) -> LexedClause<'a> {
        LexedClause::new(self.tokens)
    }

    pub(crate) fn tokens(self) -> &'a [OwnedLexToken] {
        self.tokens
    }

    pub(crate) fn len(self) -> usize {
        self.lexed().len()
    }

    pub(crate) fn is_empty(self) -> bool {
        self.lexed().is_empty()
    }

    pub(crate) fn token(self, idx: usize) -> Option<&'a OwnedLexToken> {
        self.lexed().token(idx)
    }

    pub(crate) fn first_is_word(self, expected: &str) -> bool {
        self.lexed().first_is_word(expected)
    }

    pub(crate) fn first_is_any_word(self, expected: &[&str]) -> bool {
        self.lexed().first_is_any_word(expected)
    }

    pub(crate) fn before(self, idx: usize) -> Self {
        Self::new(self.lexed().before(idx).tokens())
    }

    pub(crate) fn from(self, idx: usize) -> Self {
        Self::new(self.lexed().from(idx).tokens())
    }

    pub(crate) fn between(self, start: usize, end: usize) -> Self {
        Self::new(self.lexed().between(start, end).tokens())
    }

    pub(crate) fn words(self) -> SubjectVerbPrimitiveNormalizedWords<'a> {
        self.lexed().words()
    }

    pub(crate) fn word_refs(self) -> Vec<&'a str> {
        self.lexed().word_refs()
    }

    pub(crate) fn text(self) -> String {
        self.lexed().text()
    }

    pub(crate) fn span(self) -> Option<TextSpan> {
        span_from_tokens(self.tokens)
    }

    pub(crate) fn starts_with(self, expected: &[&str]) -> bool {
        self.lexed().starts_with(expected)
    }

    pub(crate) fn starts_with_any(self, phrases: &[&[&str]]) -> bool {
        self.lexed().starts_with_any(phrases)
    }

    pub(crate) fn ends_with(self, expected: &[&str]) -> bool {
        self.lexed().ends_with(expected)
    }

    pub(crate) fn ends_with_any(self, phrases: &[&[&str]]) -> bool {
        self.lexed().ends_with_any(phrases)
    }

    pub(crate) fn strip_prefix(self, expected: &[&str]) -> Option<&'a [OwnedLexToken]> {
        self.lexed()
            .strip_prefix_clause(expected)
            .map(LexedClause::tokens)
    }

    pub(crate) fn strip_prefix_clause(self, expected: &[&str]) -> Option<Self> {
        self.strip_prefix(expected).map(Self::new)
    }

    pub(crate) fn strip_any_prefix<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], &'a [OwnedLexToken])> {
        self.lexed()
            .strip_any_prefix_clause(phrases)
            .map(|(prefix, tail)| (prefix, tail.tokens()))
    }

    pub(crate) fn strip_any_prefix_clause<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self)> {
        self.strip_any_prefix(phrases)
            .map(|(prefix, tail)| (prefix, Self::new(tail)))
    }

    pub(crate) fn strip_prefix_value_clause<T: Clone>(
        self,
        phrases: &[(&[&str], T)],
    ) -> Option<(T, Self)> {
        self.lexed()
            .strip_prefix_value_clause(phrases)
            .map(|(value, tail)| (value, Self::new(tail.tokens())))
    }

    pub(crate) fn strip_suffix(self, expected: &[&str]) -> Option<Self> {
        self.lexed()
            .strip_suffix_clause(expected)
            .map(|head| Self::new(head.tokens()))
    }

    pub(crate) fn strip_any_suffix<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self)> {
        self.lexed()
            .strip_any_suffix_clause(phrases)
            .map(|(phrase, head)| (phrase, Self::new(head.tokens())))
    }

    pub(crate) fn first_word(self) -> Option<&'a str> {
        self.lexed().first_word()
    }

    pub(crate) fn find_word(self, expected: &str) -> Option<usize> {
        self.lexed().find_word(expected)
    }

    pub(crate) fn find_word_any(self, expected: &[&str]) -> Option<usize> {
        self.lexed().find_word_any(expected)
    }

    pub(crate) fn rfind_word(self, expected: &str) -> Option<usize> {
        self.lexed().rfind_word(expected)
    }

    pub(crate) fn find_phrase_start(self, expected: &[&str]) -> Option<usize> {
        self.lexed().find_phrase_start(expected)
    }

    pub(crate) fn find_any_phrase_start<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], usize)> {
        self.lexed().find_any_phrase_start(phrases)
    }

    pub(crate) fn find_any_phrase_span<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(usize, usize)> {
        self.lexed().find_any_phrase_span(phrases)
    }

    pub(crate) fn token_index_for_word_index(self, word_idx: usize) -> Option<usize> {
        self.lexed().token_index_for_word_index(word_idx)
    }

    pub(crate) fn token_index_after_words(self, word_count: usize) -> Option<usize> {
        self.lexed().token_index_after_words(word_count)
    }

    pub(crate) fn before_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_index_for_word_index(word_idx)?;
        Some(self.before(token_idx))
    }

    pub(crate) fn from_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_index_for_word_index(word_idx)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn after_words(self, word_count: usize) -> Option<Self> {
        let token_idx = self.token_index_after_words(word_count)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn before_words(self, word_count: usize) -> Option<Self> {
        let token_idx = self.token_index_after_words(word_count)?;
        Some(self.before(token_idx))
    }

    pub(crate) fn without_token_range_trimmed_clause(
        self,
        start: usize,
        len: usize,
    ) -> SubjectVerbPrimitiveOwnedClause {
        let end = start.saturating_add(len).min(self.tokens.len());
        let mut tokens = self.tokens[..start.min(self.tokens.len())].to_vec();
        tokens.extend_from_slice(&self.tokens[end..]);
        let trimmed_tokens = LexedClause::new(&tokens).trimmed().tokens().to_vec();
        SubjectVerbPrimitiveOwnedClause::new(trimmed_tokens)
    }

    pub(crate) fn without_token_ranges_trimmed_clause(
        self,
        ranges: &[(usize, usize)],
    ) -> SubjectVerbPrimitiveOwnedClause {
        let mut ranges = ranges
            .iter()
            .filter_map(|(start, len)| {
                if *len == 0 {
                    None
                } else {
                    let start = (*start).min(self.tokens.len());
                    let end = start.saturating_add(*len).min(self.tokens.len());
                    (start < end).then_some((start, end))
                }
            })
            .collect::<Vec<_>>();
        ranges.sort_unstable_by_key(|(start, _)| *start);

        let mut tokens = Vec::new();
        let mut cursor = 0usize;
        for (start, end) in ranges {
            if start > cursor {
                tokens.extend_from_slice(&self.tokens[cursor..start]);
            }
            cursor = cursor.max(end);
        }
        if cursor < self.tokens.len() {
            tokens.extend_from_slice(&self.tokens[cursor..]);
        }
        let trimmed_tokens = LexedClause::new(&tokens).trimmed().tokens().to_vec();
        SubjectVerbPrimitiveOwnedClause::new(trimmed_tokens)
    }

    pub(crate) fn without_phrase_trimmed_clause(
        self,
        phrase: &[&str],
    ) -> Option<SubjectVerbPrimitiveOwnedClause> {
        self.lexed()
            .without_phrase_trimmed(phrase)
            .map(SubjectVerbPrimitiveOwnedClause::new)
    }

    pub(crate) fn without_any_phrase_trimmed_clause<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], SubjectVerbPrimitiveOwnedClause)> {
        self.lexed()
            .without_any_phrase_trimmed(phrases)
            .map(|(phrase, tokens)| (phrase, SubjectVerbPrimitiveOwnedClause::new(tokens)))
    }

    pub(crate) fn find_token_word(self, expected: &str) -> Option<usize> {
        self.lexed().find_token_word(expected)
    }

    pub(crate) fn find_token_word_any(self, expected: &[&str]) -> Option<usize> {
        self.lexed().find_token_word_any(expected)
    }

    pub(crate) fn find_token_word_where(
        self,
        expected: &str,
        mut predicate: impl FnMut(usize, Self) -> bool,
    ) -> Option<usize> {
        self.lexed().find_token_word_where(expected, |idx, tail| {
            predicate(idx, Self::new(tail.tokens()))
        })
    }

    pub(crate) fn find_unquoted_token_word(self, expected: &str) -> Option<usize> {
        self.lexed().find_unquoted_token_word(expected)
    }

    pub(crate) fn rfind_token_word(self, expected: &str) -> Option<usize> {
        self.lexed().rfind_token_word(expected)
    }

    pub(crate) fn split_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_word(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_word_trimmed(self, expected: &str) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_word_trimmed(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_word_any(self, expected: &[&str]) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_word_any(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_word_any_trimmed(self, expected: &[&str]) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_word_any_trimmed(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn rsplit_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        self.lexed()
            .rsplit_once_on_word(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn rsplit_once_on_word_trimmed(self, expected: &str) -> Option<(Self, Self)> {
        self.lexed()
            .rsplit_once_on_word_trimmed(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_comma(self) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_comma()
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_phrase(self, expected: &[&str]) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_phrase(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_before_phrase(self, expected: &[&str]) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_before_phrase(expected)
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_any_phrase(
        self,
        phrases: &[&'static [&'static str]],
    ) -> Option<(&'static [&'static str], Self, Self)> {
        self.lexed()
            .split_once_on_any_phrase(phrases)
            .map(|(phrase, head, tail)| {
                (phrase, Self::new(head.tokens()), Self::new(tail.tokens()))
            })
    }

    pub(crate) fn contains_word(self, expected: &str) -> bool {
        self.lexed().contains_word(expected)
    }

    pub(crate) fn contains_any_word(self, expected: &[&str]) -> bool {
        self.lexed().contains_any_word(expected)
    }

    pub(crate) fn count_word(self, expected: &str) -> usize {
        self.lexed().count_word(expected)
    }

    pub(crate) fn contains_comma(self) -> bool {
        self.lexed().contains_comma()
    }

    pub(crate) fn contains_comma_or_any_word(self, expected: &[&str]) -> bool {
        self.lexed().contains_comma_or_any_word(expected)
    }

    pub(crate) fn contains_all_words(self, expected: &[&str]) -> bool {
        self.lexed().contains_all_words(expected)
    }

    pub(crate) fn contains_phrase(self, expected: &[&str]) -> bool {
        self.words().has_phrase(expected)
    }

    pub(crate) fn contains_any_phrase(self, phrases: &[&[&str]]) -> bool {
        self.lexed().contains_any_phrase(phrases)
    }

    pub(crate) fn trim(self) -> Vec<OwnedLexToken> {
        self.lexed().trim()
    }

    pub(crate) fn trimmed(self) -> Self {
        Self::new(self.lexed().trimmed().tokens())
    }

    pub(crate) fn trimmed_tokens(self) -> &'a [OwnedLexToken] {
        self.lexed().trimmed_tokens()
    }

    pub(crate) fn trimmed_word_refs(self) -> Vec<&'a str> {
        self.lexed().trimmed_word_refs()
    }

    pub(crate) fn comma_segments(self) -> Vec<Self> {
        self.lexed()
            .comma_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub(crate) fn trimmed_comma_segments(self) -> Vec<Self> {
        self.lexed()
            .trimmed_comma_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub(crate) fn and_segments(self) -> Vec<Self> {
        self.lexed()
            .and_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub(crate) fn trimmed_and_segments(self) -> Vec<Self> {
        self.lexed()
            .trimmed_and_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub(crate) fn trimmed_and_comma_segments(self) -> Vec<Self> {
        self.lexed()
            .trimmed_and_comma_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub(crate) fn period_segments(self) -> Vec<Self> {
        self.lexed()
            .period_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub(crate) fn trimmed_period_segments(self) -> Vec<Self> {
        self.lexed()
            .trimmed_period_segments()
            .into_iter()
            .map(|segment| Self::new(segment.tokens()))
            .collect()
    }

    pub(crate) fn split_comma_then(self) -> Option<(Self, Self)> {
        self.lexed()
            .split_comma_then()
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_comma_then_trimmed(self) -> Option<(Self, Self)> {
        self.lexed()
            .split_comma_then_trimmed()
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_then(self) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_then()
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn split_once_on_then_trimmed(self) -> Option<(Self, Self)> {
        self.lexed()
            .split_once_on_then_trimmed()
            .map(|(head, tail)| (Self::new(head.tokens()), Self::new(tail.tokens())))
    }

    pub(crate) fn comma_then_idx(self) -> Option<usize> {
        self.lexed().comma_then_idx()
    }

    pub(crate) fn without_leading_connectors_clause(self) -> Self {
        Self::new(self.lexed().without_leading_connectors_clause().tokens())
    }

    pub(crate) fn without_trailing_words_clause(self, words: &[&str]) -> Self {
        Self::new(self.lexed().without_trailing_words_clause(words).tokens())
    }

    pub(crate) fn parse_with_lexed(
        self,
        parser: fn(&[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        parser(self.tokens)
    }

    pub(crate) fn parse_one_with_lexed(
        self,
        parser: fn(&[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError>,
    ) -> Result<Option<Vec<EffectAst>>, CardTextError> {
        Ok(parser(self.tokens)?.map(|effect| vec![effect]))
    }

    pub(crate) fn parse_value_with_lexed<T>(
        self,
        parser: fn(&[OwnedLexToken]) -> Result<Option<T>, CardTextError>,
    ) -> Result<Option<T>, CardTextError> {
        parser(self.tokens)
    }
}

impl<'a> std::ops::Deref for SubjectVerbPrimitiveClause<'a> {
    type Target = [OwnedLexToken];

    fn deref(&self) -> &Self::Target {
        self.tokens
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct SubjectVerbPrimitiveOwnedClause {
    tokens: Vec<OwnedLexToken>,
}

#[allow(dead_code)]
impl SubjectVerbPrimitiveOwnedClause {
    pub(crate) fn new(tokens: Vec<OwnedLexToken>) -> Self {
        Self { tokens }
    }

    pub(crate) fn from_clause(clause: SubjectVerbPrimitiveClause<'_>) -> Self {
        Self::new(clause.tokens().to_vec())
    }

    pub(crate) fn from_comma_trimmed_clause(clause: SubjectVerbPrimitiveClause<'_>) -> Self {
        Self::new(clause.trim())
    }

    pub(crate) fn synthetic_words(words: &[&str]) -> Self {
        Self::new(
            words
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect(),
        )
    }

    pub(crate) fn as_clause(&self) -> SubjectVerbPrimitiveClause<'_> {
        SubjectVerbPrimitiveClause::new(&self.tokens)
    }

    pub(crate) fn tokens(&self) -> &[OwnedLexToken] {
        &self.tokens
    }

    pub(crate) fn len(&self) -> usize {
        self.tokens.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub(crate) fn first_word(&self) -> Option<&str> {
        self.as_clause().first_word()
    }

    pub(crate) fn word_refs(&self) -> Vec<&str> {
        self.as_clause().word_refs()
    }

    pub(crate) fn contains_word(&self, expected: &str) -> bool {
        self.as_clause().contains_word(expected)
    }

    pub(crate) fn find_token_word(&self, expected: &str) -> Option<usize> {
        self.as_clause().find_token_word(expected)
    }

    pub(crate) fn from_tokens(&self, idx: usize) -> &[OwnedLexToken] {
        &self.tokens[idx.min(self.tokens.len())..]
    }

    pub(crate) fn append_comma_then(&mut self, clause: SubjectVerbPrimitiveClause<'_>) {
        self.tokens
            .push(OwnedLexToken::comma(TextSpan::synthetic()));
        self.tokens.extend_from_slice(clause.tokens());
    }

    pub(crate) fn append_clause(&mut self, clause: SubjectVerbPrimitiveClause<'_>) {
        self.tokens.extend_from_slice(clause.tokens());
    }

    pub(crate) fn extend_from_slice(&mut self, tokens: &[OwnedLexToken]) {
        self.tokens.extend_from_slice(tokens);
    }

    pub(crate) fn insert_leading_word(&mut self, word: &str) {
        self.tokens.insert(
            0,
            OwnedLexToken::word(word.to_string(), TextSpan::synthetic()),
        );
    }

    pub(crate) fn remove_leading_word(&mut self, expected: &str) -> bool {
        if self
            .tokens
            .first()
            .is_some_and(|token| token.is_word(expected))
        {
            self.tokens.remove(0);
            true
        } else {
            false
        }
    }

    pub(crate) fn replace_leading_word(&mut self, word: &str) -> bool {
        if let Some(token) = self.tokens.first_mut()
            && token.as_word().is_some()
        {
            token.replace_word(word);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectVerbPrimitiveStage {
    PreDiagnostic,
    PostDiagnostic,
}

pub(crate) struct SubjectVerbPrimitive {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) stage: SubjectVerbPrimitiveStage,
    pub(crate) head_hints: &'static [LexRuleHeadHint],
    pub(crate) shape_mask: u32,
    pub(crate) parser: SubjectVerbPrimitiveParser,
}

pub(super) fn strip_quoted_possessive_suffix(word: &str) -> &str {
    str_strip_suffix(word, "'s")
        .or_else(|| str_strip_suffix(word, "’s"))
        .or_else(|| str_strip_suffix(word, "s'"))
        .or_else(|| str_strip_suffix(word, "s’"))
        .unwrap_or(word)
}

pub(super) fn parse_pluralized_subtype_word(word: &str) -> Option<Subtype> {
    parse_subtype_word(word).or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
}

fn summarize_effects(effects: &[EffectAst]) -> String {
    effects
        .iter()
        .map(|effect| {
            let debug = format!("{effect:?}");
            debug
                .split(|ch: char| ch == ' ' || ch == '{' || ch == '(')
                .next()
                .unwrap_or("Effect")
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn primitive_subject_verb_route(id: &str) -> String {
    let verb = if id.contains("choose") {
        "Choose"
    } else if id.contains("search") {
        "Search"
    } else if id.contains("reveal") {
        "Reveal"
    } else if id.contains("exile") {
        "Exile"
    } else if id.contains("destroy") {
        "Destroy"
    } else if id.contains("return") {
        "Return"
    } else if id.contains("sacrifice") {
        "Sacrifice"
    } else if id.contains("counter") || id.contains("sticker") {
        "Put"
    } else if id.contains("draw") {
        "Draw"
    } else if id.contains("damage") {
        "Deal"
    } else if id.contains("gain") {
        "Gain"
    } else if id.contains("shuffle") {
        "Shuffle"
    } else if id.contains("copy") {
        "Copy"
    } else if id.contains("transform") {
        "Transform"
    } else if id.contains("cant") {
        "Cant"
    } else if id.contains("become") || id.contains("type") {
        "Become"
    } else if id.contains("distribute") {
        "Distribute"
    } else if id.contains("fight") {
        "Fight"
    } else if id.contains("unless-pays") {
        "Pay"
    } else {
        "Do"
    };
    let subject = if id.starts_with("each-player")
        || id.starts_with("for-each-player")
        || id.starts_with("each-opponent")
    {
        "iterated"
    } else if id.starts_with("you") {
        "explicit"
    } else if id.starts_with("target") {
        "explicit"
    } else {
        "implicit"
    };
    format!("subject-verb verb={verb} subject={subject} recognizer={id}")
}

fn run_sentence_primitive(
    primitive: &SubjectVerbPrimitive,
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = SubjectVerbPrimitiveClause::new(tokens);
    match (primitive.parser)(clause) {
        Ok(Some(effects)) => {
            let stage = format!(
                "parse_effect_sentence:subject-verb-primitive-hit:{}",
                primitive.id
            );
            parser_trace(&stage, tokens);
            parse_trace::event(format!(
                "effect subject/verb primitive: {} -> {}",
                primitive.id,
                summarize_effects(&effects)
            ));
            parse_trace::event(format!(
                "effect-route: {}",
                primitive_subject_verb_route(primitive.id)
            ));
            if effects.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "primitive '{}' produced empty effects (clause: '{}')",
                    primitive.id,
                    clause.text()
                )));
            }
            Ok(Some(effects))
        }
        Ok(None) => Ok(None),
        Err(err) => {
            if parser_trace_enabled() {
                eprintln!(
                    "[parser-flow] stage=parse_effect_sentence:subject-verb-primitive-error primitive={} clause='{}' error={err:?}",
                    primitive.id,
                    clause.text()
                );
            }
            parse_trace::event(format!(
                "effect subject/verb primitive: {} errored: {err:?}",
                primitive.id
            ));
            Err(err)
        }
    }
}

fn normalize_parser_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    for token in &mut normalized {
        match token.kind {
            crate::runtime_backend::lexer::TokenKind::Word
            | crate::runtime_backend::lexer::TokenKind::Number
            | crate::runtime_backend::lexer::TokenKind::Tilde => {
                let replacement = token.parser_text().to_string();
                let _ = token.replace_word(replacement);
            }
            _ => {}
        }
    }
    normalized
}

fn run_sentence_primitive_lexed(
    primitive: &SubjectVerbPrimitive,
    tokens: &[OwnedLexToken],
    lowered: &OnceCell<Vec<OwnedLexToken>>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let lowered_tokens = lowered.get_or_init(|| normalize_parser_tokens(tokens));
    run_sentence_primitive(primitive, lowered_tokens)
}

pub(crate) fn run_subject_verb_primitives_lexed(
    tokens: &[OwnedLexToken],
    primitives: &'static [SubjectVerbPrimitive],
    index: &LexRuleHintIndex,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head, second) = lexed_head_words(tokens).unwrap_or(("", None));
    let mut tried = vec![false; primitives.len()];
    let lowered = OnceCell::new();
    let mut candidate_indices = index.candidate_indices(head, second);
    candidate_indices.sort_by_key(|idx| (primitives[*idx].priority, primitives[*idx].shape_mask));
    for idx in candidate_indices {
        tried[idx] = true;
        if let Some(effects) = run_sentence_primitive_lexed(&primitives[idx], tokens, &lowered)? {
            return Ok(Some(effects));
        }
    }

    let mut fallback_indices = primitives
        .iter()
        .enumerate()
        .filter_map(|(idx, _)| (!tried[idx]).then_some(idx))
        .collect::<Vec<_>>();
    fallback_indices.sort_by_key(|idx| (primitives[*idx].priority, primitives[*idx].shape_mask));

    for idx in fallback_indices {
        let primitive = &primitives[idx];
        if let Some(effects) = run_sentence_primitive_lexed(primitive, tokens, &lowered)? {
            return Ok(Some(effects));
        }
    }

    Ok(None)
}

pub(super) fn parse_preconditional_subject_verb_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    debug_assert!(
        PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
            .iter()
            .all(|primitive| primitive.stage == SubjectVerbPrimitiveStage::PreDiagnostic)
    );
    run_subject_verb_primitives_lexed(
        view.tokens,
        PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )
}

pub(super) fn parse_postconditional_subject_verb_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    debug_assert!(
        POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
            .iter()
            .all(|primitive| primitive.stage == SubjectVerbPrimitiveStage::PostDiagnostic)
    );
    run_subject_verb_primitives_lexed(
        view.tokens,
        POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
        &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
    )
}

pub(crate) const SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>;
    1] = [LexRuleDef {
    id: "preconditional-subject-verb-primitives",
    priority: 135,
    heads: &[],
    shape_mask: 0,
    run: parse_preconditional_subject_verb_primitives_rule_lexed,
}];

pub(crate) const SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>;
    1] = [LexRuleDef {
    id: "postconditional-subject-verb-primitives",
    priority: 160,
    heads: &[],
    shape_mask: 0,
    run: parse_postconditional_subject_verb_primitives_rule_lexed,
}];

pub(crate) const SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SUBJECT_VERB_PRIMITIVE_PRE_DIAGNOSTIC_RULES_LEXED);

pub(crate) const SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SUBJECT_VERB_PRIMITIVE_POST_DIAGNOSTIC_RULES_LEXED);

pub(crate) fn parse_sentence_return_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_return_with_counters_on_it(SubjectVerbPrimitiveClause::new(tokens))
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_put_onto_battlefield_with_counters_on_it(SubjectVerbPrimitiveClause::new(tokens))
}

pub(crate) fn parse_sentence_exile_source_with_counters_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_exile_source_with_counters(SubjectVerbPrimitiveClause::new(tokens))
}

pub(crate) fn parse_you_and_target_player_each_draw_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if clause.strip_prefix(&["you", "and"]).is_none() {
        return Ok(None);
    }

    let (target_player, mut idx) =
        match (clause_words.get(2).copied(), clause_words.get(3).copied()) {
            (Some("target"), Some("opponent" | "opponents")) => (PlayerAst::TargetOpponent, 4usize),
            (Some("target"), Some("player" | "players")) => (PlayerAst::Target, 4usize),
            (Some("that"), Some("player" | "players")) => (PlayerAst::That, 4usize),
            _ => return Ok(None),
        };

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let remainder_words = &clause_words[idx..];
    if remainder_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_text
        )));
    }
    if let Some((count, used_words)) = parse_half_rounded_down_draw_count_words(remainder_words) {
        let trailing_words = &remainder_words[used_words..];
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing shared draw clause (clause: '{}')",
                clause_text
            )));
        }
        return Ok(Some(vec![
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::You,
                SubjectVerbActionAst::Draw {
                    count: count.clone(),
                },
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                target_player,
                SubjectVerbActionAst::Draw { count },
            ),
        ]));
    }
    let synthetic_clause = SubjectVerbPrimitiveOwnedClause::synthetic_words(&remainder_words);
    let (count, used) = parse_value(synthetic_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_text
        ))
    })?;
    if synthetic_clause
        .tokens()
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in shared draw sentence (clause: '{}')",
            clause_text
        )));
    }

    let trailing_words = synthetic_clause.as_clause().from(used + 1).word_refs();
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw clause (clause: '{}')",
            clause_text
        )));
    }

    Ok(Some(vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: count.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            target_player,
            SubjectVerbActionAst::Draw { count },
        ),
    ]))
}

pub(crate) fn parse_sentence_you_and_target_player_each_draw(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_target_player_each_draw_sentence(clause)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EventValueSpec;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn shared_draw_sentence_accepts_that_player() {
        let tokens = lex_line("You and that player each draw that many cards.", 0)
            .expect("xyris-style shared draw clause should lex");

        let parsed = parse_you_and_target_player_each_draw_sentence(
            SubjectVerbPrimitiveClause::new(&tokens),
        )
        .expect("xyris-style shared draw clause should not error")
        .expect("xyris-style shared draw clause should parse");

        assert!(matches!(
            parsed.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::You,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw {
                        count: Value::EventValue(EventValueSpec::Amount),
                    },
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject: SubjectVerbSubjectAst {
                        player: PlayerAst::That,
                        ..
                    },
                    action: SubjectVerbActionAst::Draw {
                        count: Value::EventValue(EventValueSpec::Amount),
                    },
                }),
            ]
        ));
    }
}

pub(crate) fn parse_sentence_choose_player_to_effect(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let stripped_clause = clause.without_leading_connectors_clause();
    if stripped_clause.is_empty() {
        return Ok(None);
    }

    let Some((choose_clause, tail_clause)) = stripped_clause.split_once_on_word("to") else {
        return Ok(None);
    };

    let choose_clause = choose_clause.trimmed();
    let tail_clause = tail_clause.trimmed();
    if choose_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }
    let Some((chooser, filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(choose_clause.tokens())?
    else {
        return Ok(None);
    };

    let mut tail_effects = parse_effect_chain(tail_clause.tokens())?;
    for effect in &mut tail_effects {
        bind_implicit_player_context(effect, PlayerAst::That);
    }

    let mut effects = vec![EffectAst::subject_verb_choose_player(
        chooser,
        filter,
        TagKey::from(IT_TAG),
        random,
        exclude_previous_choices,
    )];
    effects.extend(tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_half_the_creatures_they_control_to_their_owners_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let stripped_clause = clause.without_leading_connectors_clause();
    if stripped_clause.len() < 10 || !stripped_clause.first_is_word("return") {
        return Ok(None);
    }

    let words = stripped_clause.word_refs();
    let Some(the_idx) = words.iter().position(|word| *word == "the") else {
        return Ok(None);
    };
    let Some(they_idx) = words.iter().position(|word| *word == "they") else {
        return Ok(None);
    };
    let Some(control_idx) = words.iter().position(|word| *word == "control") else {
        return Ok(None);
    };
    let Some(to_idx) = words.iter().position(|word| *word == "to") else {
        return Ok(None);
    };
    let Some(owner_idx) = words
        .iter()
        .position(|word| matches!(*word, "owner's" | "owners'" | "owners" | "owner"))
    else {
        return Ok(None);
    };
    if the_idx + 1 >= they_idx
        || they_idx + 1 != control_idx
        || control_idx >= to_idx
        || to_idx >= owner_idx
        || !words
            .get(owner_idx + 1)
            .is_some_and(|word| matches!(*word, "hand" | "hands"))
        || !word_slice_ends_with(&words, &["rounded", "up"])
    {
        return Ok(None);
    }

    let Some(filter_clause) = stripped_clause
        .from_word(the_idx + 1)
        .and_then(|tail| tail.before_word(they_idx - the_idx - 1))
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if filter_clause.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if filter.controller.is_none() {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
    }
    let count_value = Value::HalfRoundedDown(Box::new(Value::Add(
        Box::new(Value::Count(filter.clone())),
        Box::new(Value::Fixed(1)),
    )));
    let chosen_tag = TagKey::from("chosen");
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::dynamic_x(),
            count_value: Some(count_value),
            player: PlayerAst::That,
            tag: chosen_tag.clone(),
        },
        EffectAst::subject_verb_return_all_to_hand(ObjectFilter::tagged(chosen_tag)),
    ]))
}

pub(crate) fn parse_sentence_damage_to_that_player_half_damage_of_those_spells(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let stripped_clause = clause.without_leading_connectors_clause();
    if stripped_clause.is_empty() {
        return Ok(None);
    }

    let Some((_before_deal, after_deal)) =
        stripped_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };
    let tail_words = after_deal.word_refs();
    if tail_words.len() != 20 {
        return Ok(None);
    }
    if !crate::runtime_backend::lexer::word_slice_eq(
        &tail_words[..14],
        &[
            "damage", "to", "that", "player", "equal", "to", "half", "the", "damage", "dealt",
            "by", "one", "of", "those",
        ],
    ) || !crate::runtime_backend::lexer::word_slice_eq(
        &tail_words[15..],
        &["spells", "this", "turn", "rounded", "down"],
    ) {
        return Ok(None);
    }

    let card_type = parse_card_type(tail_words[14]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported spell type in historical half-damage sentence (clause: '{}')",
            clause.text()
        ))
    })?;
    Ok(Some(vec![
        EffectAst::subject_verb_choose_spell_cast_history(
            PlayerAst::You,
            PlayerAst::That,
            ObjectFilter::default().with_type(card_type),
            TagKey::from(IT_TAG),
        ),
        EffectAst::subject_verb_damage(
            Value::HalfRoundedDown(Box::new(Value::DamageDealtThisTurnByTaggedSpellCast(
                TagKey::from(IT_TAG),
            ))),
            TargetAst::Player(PlayerFilter::target_player(), None),
        ),
    ]))
}

pub(crate) fn parse_draw_for_each_card_exiled_from_hand_this_way_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    #[derive(Clone, Copy)]
    enum DrawExiledHandPattern {
        ThatPlayerShuffleThenDraw,
        ThatPlayerDraw,
        YouDraw,
        ImplicitDraw,
        ThatPlayerDraws,
        ImplicitDraws,
    }

    let clause = clause.without_leading_connectors_clause();
    let clause_words = clause.word_refs();
    let Some(pattern) = word_slice_matching_value(
        &clause_words,
        &[
            (
                &[
                    "that", "player", "shuffles", "then", "draws", "a", "card", "for", "each",
                    "card", "exiled", "from", "their", "hand", "this", "way",
                ],
                DrawExiledHandPattern::ThatPlayerShuffleThenDraw,
            ),
            (
                &[
                    "that", "player", "draws", "a", "card", "for", "each", "card", "exiled",
                    "from", "their", "hand", "this", "way",
                ],
                DrawExiledHandPattern::ThatPlayerDraw,
            ),
            (
                &[
                    "you", "draw", "a", "card", "for", "each", "card", "exiled", "from", "your",
                    "hand", "this", "way",
                ],
                DrawExiledHandPattern::YouDraw,
            ),
            (
                &[
                    "draw", "a", "card", "for", "each", "card", "exiled", "from", "your", "hand",
                    "this", "way",
                ],
                DrawExiledHandPattern::ImplicitDraw,
            ),
            (
                &[
                    "draws", "a", "card", "for", "each", "card", "exiled", "from", "their", "hand",
                    "this", "way",
                ],
                DrawExiledHandPattern::ThatPlayerDraws,
            ),
            (
                &[
                    "draws", "a", "card", "for", "each", "card", "exiled", "from", "your", "hand",
                    "this", "way",
                ],
                DrawExiledHandPattern::ImplicitDraws,
            ),
        ],
    ) else {
        return Ok(None);
    };
    let (player, mut effects) = match pattern {
        DrawExiledHandPattern::ThatPlayerShuffleThenDraw => (
            PlayerAst::That,
            vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            )],
        ),
        DrawExiledHandPattern::ThatPlayerDraw | DrawExiledHandPattern::ThatPlayerDraws => {
            (PlayerAst::That, Vec::new())
        }
        DrawExiledHandPattern::YouDraw => (PlayerAst::You, Vec::new()),
        DrawExiledHandPattern::ImplicitDraw | DrawExiledHandPattern::ImplicitDraws => {
            (PlayerAst::Implicit, Vec::new())
        }
    };

    let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
    if matches!(player, PlayerAst::That) {
        filter.owner = Some(PlayerFilter::IteratedPlayer);
    }

    effects.push(EffectAst::subject_verb_draw_for_each_tagged_matching(
        player,
        TagKey::from(IT_TAG),
        filter,
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_draw_for_each_card_exiled_from_hand_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_for_each_card_exiled_from_hand_this_way_sentence(clause)
}

pub(crate) fn parse_sentence_you_and_attacking_player_each_draw_and_lose(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.len() < 11 || clause.strip_prefix(&["you", "and"]).is_none() {
        return Ok(None);
    }

    let mut idx = 2usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"attacking") || clause_words.get(idx + 1) != Some(&"player") {
        return Ok(None);
    }
    idx += 2;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let draw_clause = SubjectVerbPrimitiveOwnedClause::synthetic_words(&clause_words[idx..]);
    let draw_words = draw_clause.word_refs();
    let (draw_count, after_draw_words) = if let Some((draw_count, used_words)) =
        parse_half_rounded_down_draw_count_words(&draw_words)
    {
        (draw_count, draw_words[used_words..].to_vec())
    } else {
        let (draw_count, draw_used) = parse_value(draw_clause.tokens()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing shared draw count (clause: '{}')",
                clause_text
            ))
        })?;
        if draw_clause
            .tokens()
            .get(draw_used)
            .and_then(OwnedLexToken::as_word)
            .is_none_or(|word| word != "card" && word != "cards")
        {
            return Err(CardTextError::ParseError(format!(
                "missing card keyword in shared draw/lose sentence (clause: '{}')",
                clause_text
            )));
        }

        (
            draw_count,
            draw_clause.as_clause().from(draw_used + 1).word_refs(),
        )
    };
    if after_draw_words.first() != Some(&"and")
        || !matches!(after_draw_words.get(1).copied(), Some("lose" | "loses"))
    {
        return Ok(None);
    }

    let lose_clause = SubjectVerbPrimitiveOwnedClause::synthetic_words(&after_draw_words[2..]);
    let (lose_amount, lose_used) = parse_value(lose_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared life-loss amount (clause: '{}')",
            clause_text
        ))
    })?;
    if lose_clause
        .tokens()
        .get(lose_used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "life")
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in shared draw/lose sentence (clause: '{}')",
            clause_text
        )));
    }

    let trailing_words = lose_clause.as_clause().from(lose_used + 1).word_refs();
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw/lose clause (clause: '{}')",
            clause_text
        )));
    }

    Ok(Some(vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: draw_count.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Attacking,
            SubjectVerbActionAst::Draw { count: draw_count },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife {
                amount: lose_amount.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Attacking,
            SubjectVerbActionAst::LoseLife {
                amount: lose_amount,
            },
        ),
    ]))
}

pub(crate) fn parse_sentence_sacrifice_it_next_end_step(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "sacrifice <object> at the beginning of [the] next end step"
    const NEXT_END_STEP_TIMING: &[&[&str]] = &[
        &["at", "the", "beginning", "of", "the", "next", "end", "step"],
        &["at", "the", "beginning", "of", "next", "end", "step"],
    ];
    let Some(object_clause) = clause.strip_prefix_clause(&["sacrifice"]) else {
        return Ok(None);
    };
    let Some((_timing, object_clause, _tail)) =
        object_clause.split_once_on_any_phrase(NEXT_END_STEP_TIMING)
    else {
        return Ok(None);
    };

    let object_clause = object_clause.trimmed();
    if object_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in delayed next-end-step clause (clause: '{}')",
            clause.text()
        )));
    }

    let object_words = object_clause.word_refs();
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["the", "creature"]
            | ["that", "creature"]
            | ["the", "permanent"]
            | ["that", "permanent"]
            | ["the", "token"]
            | ["that", "token"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(object_clause.tokens(), false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: PlayerFilter::Any,
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            filter,
            1,
            None,
        )],
    }]))
}

pub(crate) fn parse_sentence_if_tagged_cards_remain_exiled(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    const REMAIN_EXILED_PREFIXES: &[&[&str]] = &[
        &["if", "any", "of", "those", "cards", "remain", "exiled"],
        &["if", "those", "cards", "remain", "exiled"],
        &["if", "that", "card", "remains", "exiled"],
        &["if", "it", "remains", "exiled"],
    ];

    if !clause.starts_with_any(REMAIN_EXILED_PREFIXES) {
        return Ok(None);
    }

    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        clause.tokens(),
        parse_effect_chain_lexed,
    )
    .map(Some)
}
