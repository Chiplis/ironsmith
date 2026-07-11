use super::*;
use crate::parse_trace;
use crate::runtime_backend::front_end::grammar::effects::subject_verb_registry_shapes as registry_shapes;
pub(super) const MECHANIC_MARKER_PREFIXES: &[&[&str]] = &[
    &["you", "choose", "one", "of", "them"],
    &[
        "you", "may", "put", "a", "land", "card", "from", "among", "them", "into", "your", "hand",
    ],
    &["stand", "and", "fight"],
    &["venture", "into", "the", "dungeon"],
    &["it", "doesnt", "untap", "during"],
];
pub(crate) type SubjectVerbPrimitiveParser =
    for<'a> fn(SubjectVerbPrimitiveClause<'a>) -> Result<Option<Vec<EffectAst>>, CardTextError>;
pub(super) type SubjectVerbPrimitiveNormalizedWords<'a> = TokenWordView<'a>;

const REGISTRY_CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const PRIMITIVE_ROUTE_VERBS: &[(&[&str], &str)] = &[
    (&["choose"], "Choose"),
    (&["search"], "Search"),
    (&["reveal"], "Reveal"),
    (&["exile"], "Exile"),
    (&["destroy"], "Destroy"),
    (&["return"], "Return"),
    (&["sacrifice"], "Sacrifice"),
    (&["counter", "sticker"], "Put"),
    (&["draw"], "Draw"),
    (&["damage"], "Deal"),
    (&["gain"], "Gain"),
    (&["shuffle"], "Shuffle"),
    (&["copy"], "Copy"),
    (&["transform"], "Transform"),
    (&["cant"], "Cant"),
    (&["become", "type"], "Become"),
    (&["distribute"], "Distribute"),
    (&["fight"], "Fight"),
    (&["unless-pays"], "Pay"),
];
const PRIMITIVE_ITERATED_SUBJECT_PREFIXES: &[&str] =
    &["each-player", "for-each-player", "each-opponent"];
const PRIMITIVE_EXPLICIT_SUBJECT_PREFIXES: &[&str] = &["you", "target"];
const THAT_PLAYER_SUBJECT_WORDS: &[&str] = &["that", "player"];
const YOU_SUBJECT_WORDS: &[&str] = &["you"];
const THEIR_HAND_OWNER_WORD: &str = "their";
const YOUR_HAND_OWNER_WORD: &str = "your";

fn registry_token_matches_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn registry_word_is_card_or_cards(word: &str) -> bool {
    REGISTRY_CARD_OR_CARDS_WORDS
        .iter()
        .any(|expected| word == *expected)
}

fn registry_token_is_card_or_cards(token: &OwnedLexToken) -> bool {
    token.as_word().is_some_and(registry_word_is_card_or_cards)
}

fn registry_token_is_life(token: &OwnedLexToken) -> bool {
    registry_token_matches_word(token, "life")
}

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

    pub(crate) fn first_word(self) -> Option<&'a str> {
        self.lexed().first_word()
    }

    pub(crate) fn rfind_word(self, expected: &str) -> Option<usize> {
        self.lexed().rfind_word(expected)
    }

    pub(crate) fn token_index_after_words(self, word_count: usize) -> Option<usize> {
        self.lexed().token_index_after_words(word_count)
    }

    pub(crate) fn before_word(self, word_idx: usize) -> Option<Self> {
        registry_shapes::split_registry_clause_at_word(self.tokens, word_idx)
            .map(|split| Self::new(split.before))
    }

    pub(crate) fn from_word(self, word_idx: usize) -> Option<Self> {
        registry_shapes::split_registry_clause_at_word(self.tokens, word_idx)
            .map(|split| Self::new(split.after))
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

    pub(crate) fn count_word(self, expected: &str) -> usize {
        self.lexed().count_word(expected)
    }

    pub(crate) fn contains_comma(self) -> bool {
        self.lexed().contains_comma()
    }

    pub(crate) fn contains_comma_or_any_word(self, expected: &[&str]) -> bool {
        self.lexed().contains_comma_or_any_word(expected)
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
            .is_some_and(|token| registry_token_matches_word(token, expected))
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

impl SubjectVerbPrimitive {
    pub(crate) const fn new(
        id: &'static str,
        priority: u16,
        stage: SubjectVerbPrimitiveStage,
        head_hints: &'static [LexRuleHeadHint],
        parser: SubjectVerbPrimitiveParser,
    ) -> Self {
        Self {
            id,
            priority,
            stage,
            head_hints,
            shape_mask: 0,
            parser,
        }
    }
}

pub(super) fn parse_pluralized_subtype_word(word: &str) -> Option<Subtype> {
    parse_subtype_flexible(word)
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
    let verb = primitive_route_verb(id);
    let subject = if primitive_route_starts_with_any(id, PRIMITIVE_ITERATED_SUBJECT_PREFIXES) {
        "iterated"
    } else if primitive_route_starts_with_any(id, PRIMITIVE_EXPLICIT_SUBJECT_PREFIXES) {
        "explicit"
    } else {
        "implicit"
    };
    format!("subject-verb verb={verb} subject={subject} recognizer={id}")
}

fn primitive_route_verb(id: &str) -> &'static str {
    PRIMITIVE_ROUTE_VERBS
        .iter()
        .find_map(|(needles, label)| primitive_route_contains_any(id, needles).then_some(*label))
        .unwrap_or("Do")
}

fn primitive_route_contains_any(id: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| id.contains(needle))
}

fn primitive_route_starts_with_any(id: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| id.starts_with(prefix))
}

fn run_sentence_primitive(
    primitive: &SubjectVerbPrimitive,
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = SubjectVerbPrimitiveClause::new(tokens);
    let parsed = (primitive.parser)(clause);
    match parsed {
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
    let Some(shape) = registry_shapes::parse_joint_draw_shape(clause.tokens()) else {
        return Ok(None);
    };
    let amount_clause = SubjectVerbPrimitiveClause::new(shape.amount_tokens);
    let clause_text = clause.text();
    let remainder_words = amount_clause.word_refs();
    let count = if let Some((count, used_words)) =
        parse_half_rounded_down_draw_count_words(&remainder_words)
    {
        if !remainder_words[used_words..].is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing shared draw clause (clause: '{}')",
                clause_text
            )));
        }
        count
    } else {
        let (count, used) = parse_value(amount_clause.tokens()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing draw count in shared draw sentence (clause: '{}')",
                clause_text
            ))
        })?;
        if amount_clause
            .tokens()
            .get(used)
            .and_then(OwnedLexToken::as_word)
            .is_none_or(|word| !registry_word_is_card_or_cards(word))
        {
            return Err(CardTextError::ParseError(format!(
                "missing card keyword in shared draw sentence (clause: '{}')",
                clause_text
            )));
        }
        if !amount_clause.from(used + 1).word_refs().is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing shared draw clause (clause: '{}')",
                clause_text
            )));
        }
        count
    };
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
            shape.other_player,
            SubjectVerbActionAst::Draw { count },
        ),
    ]))
}

pub(crate) fn parse_sentence_you_and_target_player_each_draw(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_target_player_each_draw_sentence(clause)
}

/// "You and that player each gain that much life." / "You and target opponent
/// each lose 2 life." — the joint-subject analog of the shared draw sentence.
pub(crate) fn parse_you_and_player_each_gain_or_lose_life_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_joint_life_shape(clause.tokens()) else {
        return Ok(None);
    };
    let amount_clause = SubjectVerbPrimitiveClause::new(shape.amount_tokens);
    let Some((amount, used)) = parse_value(amount_clause.tokens()) else {
        return Ok(None);
    };
    if amount_clause
        .tokens()
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "life")
        || !amount_clause.from(used + 1).word_refs().is_empty()
    {
        return Ok(None);
    }
    let action = |amount: Value| {
        if shape.gains {
            SubjectVerbActionAst::GainLife { amount }
        } else {
            SubjectVerbActionAst::LoseLife { amount }
        }
    };
    Ok(Some(vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            action(amount.clone()),
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            shape.other_player,
            action(amount),
        ),
    ]))
}

pub(crate) fn parse_sentence_you_and_player_each_gain_or_lose_life(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_player_each_gain_or_lose_life_sentence(clause)
}

/// "You and that player each create three 1/1 white Spirit creature tokens
/// with flying." — joint-subject token creation: parse the verb phrase once
/// and emit one copy per subject.
pub(crate) fn parse_you_and_player_each_create_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_joint_create_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Ok(parsed) =
        crate::runtime_backend::sentences::effect_sentences::parse_effect_sentence_lexed(
            shape.effect_tokens,
        )
    else {
        return Ok(None);
    };
    let [EffectAst::SubjectVerb(template)] = parsed.as_slice() else {
        return Ok(None);
    };
    fn with_subject_player(
        template: &SubjectVerbEffectAst,
        player: PlayerAst,
    ) -> SubjectVerbEffectAst {
        let mut copy = template.clone();
        copy.subject.player = player.clone();
        if let SubjectVerbActionAst::CreateTokenWithMods {
            player: action_player,
            ..
        } = &mut copy.action
        {
            *action_player = player;
        }
        copy
    }
    Ok(Some(vec![
        EffectAst::SubjectVerb(with_subject_player(template, PlayerAst::You)),
        EffectAst::SubjectVerb(with_subject_player(template, shape.other_player)),
    ]))
}

pub(crate) fn parse_sentence_you_and_player_each_create(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_player_each_create_sentence(clause)
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
    let Some(shape) = registry_shapes::parse_choose_player_to_effect_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Some((chooser, filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(shape.choose_tokens)?
    else {
        return Ok(None);
    };
    let mut tail_effects = parse_effect_chain(shape.effect_tokens)?;
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
    let Some(shape) = registry_shapes::parse_return_half_controlled_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter(shape.filter_tokens, false)?;
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
    let Some(shape) = registry_shapes::parse_historical_half_damage_shape(clause.tokens()) else {
        return Ok(None);
    };
    let card_type = parse_card_type(shape.card_type_word).ok_or_else(|| {
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
    let Some(shape) = registry_shapes::parse_draw_for_exiled_hand_shape(clause.tokens()) else {
        return Ok(None);
    };
    let subject_words = LexedClause::new(shape.subject_tokens).word_refs();
    let hand_owner = match shape.hand_owner {
        registry_shapes::ExiledHandOwner::Your => Some(YOUR_HAND_OWNER_WORD),
        registry_shapes::ExiledHandOwner::Their => Some(THEIR_HAND_OWNER_WORD),
    };
    let Some((player, mut effects)) = draw_exiled_hand_this_way_actor(
        &subject_words,
        hand_owner,
        shape.shuffles_first,
        shape.starts_with_draws,
    ) else {
        return Ok(None);
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

fn draw_exiled_hand_this_way_actor(
    subject_words: &[&str],
    hand_owner: Option<&str>,
    shuffles_first: bool,
    starts_with_draws: bool,
) -> Option<(PlayerAst, Vec<EffectAst>)> {
    if subject_words == THAT_PLAYER_SUBJECT_WORDS && hand_owner == Some(THEIR_HAND_OWNER_WORD) {
        let effects = if shuffles_first {
            vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            )]
        } else {
            Vec::new()
        };
        return Some((PlayerAst::That, effects));
    }
    if !shuffles_first
        && subject_words == YOU_SUBJECT_WORDS
        && hand_owner == Some(YOUR_HAND_OWNER_WORD)
    {
        return Some((PlayerAst::You, Vec::new()));
    }
    if !shuffles_first
        && subject_words.is_empty()
        && hand_owner == Some(THEIR_HAND_OWNER_WORD)
        && starts_with_draws
    {
        return Some((PlayerAst::That, Vec::new()));
    }
    if !shuffles_first && subject_words.is_empty() && hand_owner == Some(YOUR_HAND_OWNER_WORD) {
        return Some((PlayerAst::Implicit, Vec::new()));
    }
    None
}

pub(crate) fn parse_sentence_draw_for_each_card_exiled_from_hand_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_for_each_card_exiled_from_hand_this_way_sentence(clause)
}

pub(crate) fn parse_sentence_you_and_attacking_player_each_draw_and_lose(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_attacking_player_draw_lose_shape(clause.tokens())
    else {
        return Ok(None);
    };
    let draw_clause = SubjectVerbPrimitiveClause::new(shape.draw_tokens);
    let draw_words = draw_clause.word_refs();
    let draw_count =
        if let Some((count, used_words)) = parse_half_rounded_down_draw_count_words(&draw_words) {
            if !draw_words[used_words..].is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing shared draw clause (clause: '{}')",
                    clause.text()
                )));
            }
            count
        } else {
            let (count, used) = parse_value(draw_clause.tokens()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing shared draw count (clause: '{}')",
                    clause.text()
                ))
            })?;
            if draw_clause
                .tokens()
                .get(used)
                .is_none_or(|token| !registry_token_is_card_or_cards(token))
                || !draw_clause.from(used + 1).word_refs().is_empty()
            {
                return Err(CardTextError::ParseError(format!(
                    "missing card keyword in shared draw/lose sentence (clause: '{}')",
                    clause.text()
                )));
            }
            count
        };
    let lose_clause = SubjectVerbPrimitiveClause::new(shape.lose_tokens);
    let (lose_amount, lose_used) = parse_value(lose_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared life-loss amount (clause: '{}')",
            clause.text()
        ))
    })?;
    if lose_clause
        .tokens()
        .get(lose_used)
        .is_none_or(|token| !registry_token_is_life(token))
        || !lose_clause.from(lose_used + 1).word_refs().is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in shared draw/lose sentence (clause: '{}')",
            clause.text()
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
    let Some(shape) = registry_shapes::parse_registry_next_end_step_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.action != registry_shapes::RegistryDelayedAction::Sacrifice {
        return Ok(None);
    }
    let filter = if registry_shapes::is_tagged_delayed_object(shape.object_tokens) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(shape.object_tokens, false)?
    };
    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: if shape.your_end_step {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        },
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            filter,
            1,
            None,
        )],
    }]))
}

pub(crate) fn parse_sentence_exile_it_next_end_step(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = registry_shapes::parse_registry_next_end_step_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.action != registry_shapes::RegistryDelayedAction::Exile {
        return Ok(None);
    }
    let object_clause = SubjectVerbPrimitiveClause::new(shape.object_tokens);
    let target = if registry_shapes::is_tagged_delayed_object(shape.object_tokens) {
        TargetAst::Tagged(TagKey::from(IT_TAG), object_clause.span())
    } else {
        TargetAst::Object(
            parse_object_filter(shape.object_tokens, false)?,
            None,
            object_clause.span(),
        )
    };
    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: if shape.your_end_step {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        },
        effects: vec![EffectAst::subject_verb_exile(target, false)],
    }]))
}

pub(crate) fn parse_sentence_if_tagged_cards_remain_exiled(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if registry_shapes::parse_remain_exiled_tail(clause.tokens()).is_none() {
        return Ok(None);
    }
    parse_conditional_sentence_with_grammar_entrypoint_lexed(
        clause.tokens(),
        parse_effect_chain_lexed,
    )
    .map(Some)
}
