#![allow(dead_code)]

use std::ops::Range;

use super::lexer::{LexedClause, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LexCaptureRole {
    Subject,
    Action,
    Object,
    Modifier,
    Condition,
    Amount,
    Tail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LexCaptureKind<'p> {
    Rest,
    WordCount(usize),
    OneOf(&'p [&'p str]),
    OneOfPhrase(&'p [&'p [&'p str]]),
    UntilPhrase(&'p [&'p str]),
    UntilLastPhrase(&'p [&'p str]),
    UntilLastPhraseBeforeToken(&'p [&'p str], TokenKind),
    UntilAnyPhrase(&'p [&'p [&'p str]]),
    UntilLastAnyPhrase(&'p [&'p [&'p str]]),
    UntilToken(TokenKind),
    OneOrMoreWords,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LexPatternAtom<'p> {
    Word(&'p str),
    Token(TokenKind),
    AnyWord(&'p [&'p str]),
    Phrase(&'p [&'p str]),
    AnyPhrase(&'p [&'p [&'p str]]),
    Optional(&'p [LexPatternAtom<'p>]),
    AnySequence(&'p [&'p [LexPatternAtom<'p>]]),
    Capture(&'p str, LexCaptureKind<'p>),
    RoleCapture(&'p str, LexCaptureRole, LexCaptureKind<'p>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LexPattern<'p> {
    atoms: &'p [LexPatternAtom<'p>],
}

impl<'p> LexPattern<'p> {
    pub(crate) const fn new(atoms: &'p [LexPatternAtom<'p>]) -> Self {
        Self { atoms }
    }

    pub(crate) const fn word(word: &'p str) -> LexPatternAtom<'p> {
        LexPatternAtom::Word(word)
    }

    pub(crate) const fn token(kind: TokenKind) -> LexPatternAtom<'p> {
        LexPatternAtom::Token(kind)
    }

    pub(crate) const fn any_word(words: &'p [&'p str]) -> LexPatternAtom<'p> {
        LexPatternAtom::AnyWord(words)
    }

    pub(crate) const fn phrase(words: &'p [&'p str]) -> LexPatternAtom<'p> {
        LexPatternAtom::Phrase(words)
    }

    pub(crate) const fn any_phrase(phrases: &'p [&'p [&'p str]]) -> LexPatternAtom<'p> {
        LexPatternAtom::AnyPhrase(phrases)
    }

    pub(crate) const fn optional(atoms: &'p [LexPatternAtom<'p>]) -> LexPatternAtom<'p> {
        LexPatternAtom::Optional(atoms)
    }

    pub(crate) const fn any_sequence(
        sequences: &'p [&'p [LexPatternAtom<'p>]],
    ) -> LexPatternAtom<'p> {
        LexPatternAtom::AnySequence(sequences)
    }

    pub(crate) const fn capture(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        LexPatternAtom::Capture(name, kind)
    }

    pub(crate) const fn role_capture(
        name: &'p str,
        role: LexCaptureRole,
        kind: LexCaptureKind<'p>,
    ) -> LexPatternAtom<'p> {
        LexPatternAtom::RoleCapture(name, role, kind)
    }

    pub(crate) const fn subject(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        Self::role_capture(name, LexCaptureRole::Subject, kind)
    }

    pub(crate) const fn action(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        Self::role_capture(name, LexCaptureRole::Action, kind)
    }

    pub(crate) const fn object(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        Self::role_capture(name, LexCaptureRole::Object, kind)
    }

    pub(crate) const fn amount(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        Self::role_capture(name, LexCaptureRole::Amount, kind)
    }

    pub(crate) const fn modifier(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        Self::role_capture(name, LexCaptureRole::Modifier, kind)
    }

    pub(crate) const fn condition(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        Self::role_capture(name, LexCaptureRole::Condition, kind)
    }

    pub(crate) const fn tail(name: &'p str, kind: LexCaptureKind<'p>) -> LexPatternAtom<'p> {
        Self::role_capture(name, LexCaptureRole::Tail, kind)
    }

    pub(crate) fn matches_clause(self, clause: LexedClause<'_>) -> bool {
        self.match_clause(clause).is_some()
    }

    pub(crate) fn matches_prefix(self, clause: LexedClause<'_>) -> bool {
        self.match_prefix(clause).is_some()
    }

    pub(crate) fn match_clause<'a>(self, clause: LexedClause<'a>) -> Option<LexPatternMatch<'p>> {
        let words = clause.word_refs();
        let matched = self.match_words(clause, words.as_slice(), 0, true)?;
        Some(matched)
    }

    pub(crate) fn match_prefix<'a>(self, clause: LexedClause<'a>) -> Option<LexPatternMatch<'p>> {
        let words = clause.word_refs();
        self.match_words(clause, words.as_slice(), 0, false)
    }

    pub(crate) fn find_in_clause<'a>(self, clause: LexedClause<'a>) -> Option<LexPatternMatch<'p>> {
        let words = clause.word_refs();
        (0..=words.len())
            .filter_map(|start| self.match_words(clause, words.as_slice(), start, false))
            .min_by_key(|matched| matched.word_range.start)
    }

    fn match_words(
        self,
        clause: LexedClause<'_>,
        words: &[&str],
        start: usize,
        require_end: bool,
    ) -> Option<LexPatternMatch<'p>> {
        let mut captures = Vec::new();
        let cursor = match_atoms(self.atoms, clause, words, start, &mut captures)?;

        if require_end && cursor != words.len() {
            return None;
        }

        Some(LexPatternMatch {
            word_range: start..cursor,
            captures,
        })
    }
}

fn match_atoms<'p>(
    atoms: &[LexPatternAtom<'p>],
    clause: LexedClause<'_>,
    words: &[&str],
    start: usize,
    captures: &mut Vec<LexPatternCapture<'p>>,
) -> Option<usize> {
    let mut cursor = start;

    for atom in atoms {
        match *atom {
            LexPatternAtom::Word(expected) => {
                if words.get(cursor).copied() != Some(expected) {
                    return None;
                }
                cursor += 1;
            }
            LexPatternAtom::Token(expected) => {
                if !token_kind_at_word_boundary(clause, cursor, expected) {
                    return None;
                }
            }
            LexPatternAtom::AnyWord(expected) => {
                let word = words.get(cursor).copied()?;
                if !expected.contains(&word) {
                    return None;
                }
                cursor += 1;
            }
            LexPatternAtom::Phrase(phrase) => {
                if !words_at(words, cursor, phrase) {
                    return None;
                }
                cursor += phrase.len();
            }
            LexPatternAtom::AnyPhrase(phrases) => {
                let phrase = phrases
                    .iter()
                    .copied()
                    .filter(|phrase| words_at(words, cursor, phrase))
                    .max_by_key(|phrase| phrase.len())?;
                cursor += phrase.len();
            }
            LexPatternAtom::Optional(optional_atoms) => {
                let mut optional_captures = captures.clone();
                if let Some(optional_cursor) = match_atoms(
                    optional_atoms,
                    clause,
                    words,
                    cursor,
                    &mut optional_captures,
                ) {
                    *captures = optional_captures;
                    cursor = optional_cursor;
                }
            }
            LexPatternAtom::AnySequence(sequences) => {
                let mut matched = None;
                for sequence in sequences {
                    let mut sequence_captures = captures.clone();
                    if let Some(sequence_cursor) =
                        match_atoms(sequence, clause, words, cursor, &mut sequence_captures)
                    {
                        let is_better = match &matched {
                            None => true,
                            Some((matched_cursor, _)) => sequence_cursor > *matched_cursor,
                        };
                        if is_better {
                            matched = Some((sequence_cursor, sequence_captures));
                        }
                    }
                }
                let (sequence_cursor, sequence_captures) = matched?;
                *captures = sequence_captures;
                cursor = sequence_cursor;
            }
            LexPatternAtom::Capture(name, kind) => {
                cursor = match_capture(name, None, kind, clause, words, cursor, captures)?;
            }
            LexPatternAtom::RoleCapture(name, role, kind) => {
                cursor = match_capture(name, Some(role), kind, clause, words, cursor, captures)?;
            }
        }
    }

    Some(cursor)
}

fn match_capture<'p>(
    name: &'p str,
    role: Option<LexCaptureRole>,
    kind: LexCaptureKind<'p>,
    clause: LexedClause<'_>,
    words: &[&str],
    cursor: usize,
    captures: &mut Vec<LexPatternCapture<'p>>,
) -> Option<usize> {
    match kind {
        LexCaptureKind::Rest => {
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..words.len(),
            });
            Some(words.len())
        }
        LexCaptureKind::OneOrMoreWords => {
            if cursor >= words.len() {
                return None;
            }
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..words.len(),
            });
            Some(words.len())
        }
        LexCaptureKind::WordCount(count) => {
            let end = cursor.checked_add(count)?;
            if end > words.len() {
                return None;
            }
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..end,
            });
            Some(end)
        }
        LexCaptureKind::OneOf(expected) => {
            let word = words.get(cursor).copied()?;
            if !expected.contains(&word) {
                return None;
            }
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..cursor + 1,
            });
            Some(cursor + 1)
        }
        LexCaptureKind::OneOfPhrase(expected) => {
            let phrase = expected
                .iter()
                .copied()
                .find(|phrase| words_at(words, cursor, phrase))?;
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..cursor + phrase.len(),
            });
            Some(cursor + phrase.len())
        }
        LexCaptureKind::UntilPhrase(phrase) => {
            let end = find_phrase(words, cursor, phrase)?;
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..end,
            });
            Some(end)
        }
        LexCaptureKind::UntilLastPhrase(phrase) => {
            let end = rfind_phrase(words, cursor, phrase)?;
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..end,
            });
            Some(end)
        }
        LexCaptureKind::UntilLastPhraseBeforeToken(phrase, delimiter) => {
            let delimiter_word = word_index_before_token_kind(clause, cursor, delimiter)?;
            let end = rfind_phrase_in_word_range(words, cursor, delimiter_word, phrase)?;
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..end,
            });
            Some(end)
        }
        LexCaptureKind::UntilAnyPhrase(phrases) => {
            let (_, end) = find_any_phrase(words, cursor, phrases)?;
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..end,
            });
            Some(end)
        }
        LexCaptureKind::UntilLastAnyPhrase(phrases) => {
            let (_, end) = rfind_any_phrase(words, cursor, phrases)?;
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..end,
            });
            Some(end)
        }
        LexCaptureKind::UntilToken(delimiter) => {
            let end = word_index_before_token_kind(clause, cursor, delimiter)?;
            captures.push(LexPatternCapture {
                name,
                role,
                word_range: cursor..end,
            });
            Some(end)
        }
    }
}

fn token_kind_at_word_boundary(
    clause: LexedClause<'_>,
    word_cursor: usize,
    expected: TokenKind,
) -> bool {
    clause
        .token_index_after_words(word_cursor)
        .and_then(|token_idx| clause.tokens().get(token_idx))
        .is_some_and(|token| token.kind == expected)
}

fn word_index_before_token_kind(
    clause: LexedClause<'_>,
    start_word: usize,
    delimiter: TokenKind,
) -> Option<usize> {
    let start_token = clause.token_index_for_word_or_end(start_word)?;
    let delimiter_token = clause
        .tokens()
        .iter()
        .enumerate()
        .skip(start_token)
        .find_map(|(idx, token)| (token.kind == delimiter).then_some(idx))?;
    Some(
        clause
            .words()
            .token_start_indices()
            .iter()
            .take_while(|token_idx| **token_idx < delimiter_token)
            .count(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexPatternMatch<'p> {
    pub(crate) word_range: Range<usize>,
    captures: Vec<LexPatternCapture<'p>>,
}

impl<'p> LexPatternMatch<'p> {
    pub(crate) fn captures(&self) -> &[LexPatternCapture<'p>] {
        &self.captures
    }

    pub(crate) fn capture(&self, name: &str) -> Option<&LexPatternCapture<'p>> {
        self.captures.iter().find(|capture| capture.name == name)
    }

    pub(crate) fn capture_by_role(&self, role: LexCaptureRole) -> Option<&LexPatternCapture<'p>> {
        self.captures
            .iter()
            .find(|capture| capture.role == Some(role))
    }

    pub(crate) fn capture_word_range(&self, name: &str) -> Option<Range<usize>> {
        self.capture(name).map(|capture| capture.word_range.clone())
    }

    pub(crate) fn capture_clause<'a>(
        &self,
        name: &str,
        clause: LexedClause<'a>,
    ) -> Option<LexedClause<'a>> {
        self.capture(name)
            .and_then(|capture| capture.clause(clause))
    }

    pub(crate) fn capture_clause_by_role<'a>(
        &self,
        role: LexCaptureRole,
        clause: LexedClause<'a>,
    ) -> Option<LexedClause<'a>> {
        self.capture_by_role(role)
            .and_then(|capture| capture.clause(clause))
    }

    pub(crate) fn matched_clause<'a>(&self, clause: LexedClause<'a>) -> Option<LexedClause<'a>> {
        clause.between_word_range(self.word_range.start, self.word_range.end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexPatternCapture<'p> {
    pub(crate) name: &'p str,
    pub(crate) role: Option<LexCaptureRole>,
    pub(crate) word_range: Range<usize>,
}

impl LexPatternCapture<'_> {
    pub(crate) fn clause<'a>(&self, clause: LexedClause<'a>) -> Option<LexedClause<'a>> {
        clause.between_word_range(self.word_range.start, self.word_range.end)
    }
}

fn words_at(words: &[&str], start: usize, phrase: &[&str]) -> bool {
    words
        .get(start..start.saturating_add(phrase.len()))
        .is_some_and(|window| window == phrase)
}

fn find_phrase(words: &[&str], start: usize, phrase: &[&str]) -> Option<usize> {
    (start..=words.len()).find(|idx| words_at(words, *idx, phrase))
}

fn rfind_phrase(words: &[&str], start: usize, phrase: &[&str]) -> Option<usize> {
    (start..=words.len())
        .rev()
        .find(|idx| words_at(words, *idx, phrase))
}

fn rfind_phrase_in_word_range(
    words: &[&str],
    start: usize,
    end: usize,
    phrase: &[&str],
) -> Option<usize> {
    if start > end || end > words.len() {
        return None;
    }
    (start..=end)
        .rev()
        .find(|idx| idx.saturating_add(phrase.len()) <= end && words_at(words, *idx, phrase))
}

fn find_any_phrase<'p>(
    words: &[&str],
    start: usize,
    phrases: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    (start..=words.len())
        .flat_map(|idx| {
            phrases
                .iter()
                .copied()
                .filter(move |phrase| words_at(words, idx, phrase))
                .map(move |phrase| (phrase, idx))
        })
        .min_by_key(|(_, idx)| *idx)
}

fn rfind_any_phrase<'p>(
    words: &[&str],
    start: usize,
    phrases: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    (start..=words.len())
        .rev()
        .flat_map(|idx| {
            phrases
                .iter()
                .copied()
                .filter(move |phrase| words_at(words, idx, phrase))
                .map(move |phrase| (phrase, idx))
        })
        .max_by_key(|(_, idx)| *idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::TextSpan;
    use crate::runtime_backend::lexer::OwnedLexToken;

    fn tokens(words: &[&str]) -> Vec<OwnedLexToken> {
        crate::runtime_backend::lexer::synthetic_word_tokens(words)
    }

    #[test]
    fn pattern_matches_phrase_alternation_and_captures_tail() {
        let tokens = tokens(&[
            "each", "opponent", "loses", "life", "equal", "to", "cards", "in", "hand",
        ]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::any_phrase(&[&["each", "opponent"], &["each", "player"]]),
            LexPattern::word("loses"),
            LexPattern::capture("amount", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        let amount = matched
            .capture_clause("amount", clause)
            .expect("amount capture");
        assert_eq!(
            amount.word_refs(),
            ["life", "equal", "to", "cards", "in", "hand"]
        );
    }

    #[test]
    fn pattern_can_capture_until_following_phrase() {
        let tokens = tokens(&["target", "creature", "you", "control", "gets", "+1", "+1"]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::word("target"),
            LexPattern::capture("object", LexCaptureKind::UntilPhrase(&["gets"])),
            LexPattern::word("gets"),
            LexPattern::capture("modifier", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        assert_eq!(
            matched
                .capture_clause("object", clause)
                .expect("object")
                .word_refs(),
            ["creature", "you", "control"]
        );
        assert_eq!(
            matched
                .capture_clause("modifier", clause)
                .expect("modifier")
                .word_refs(),
            ["+1", "+1"]
        );
    }

    #[test]
    fn pattern_can_capture_one_of_several_phrases() {
        let tokens = tokens(&["until", "end", "of", "turn", "you", "may", "cast"]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::modifier(
                "duration",
                LexCaptureKind::OneOfPhrase(&[&["until", "end", "of", "turn"], &["this", "turn"]]),
            ),
            LexPattern::capture("permission", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Modifier, clause)
                .expect("duration")
                .word_refs(),
            ["until", "end", "of", "turn"]
        );
        assert_eq!(
            matched
                .capture_clause("permission", clause)
                .expect("permission")
                .word_refs(),
            ["you", "may", "cast"]
        );
    }

    #[test]
    fn pattern_supports_optional_groups() {
        let tokens = tokens(&[
            "sacrifice",
            "it",
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "end",
            "step",
        ]);
        let clause = LexedClause::new(&tokens);
        let optional_the = [LexPattern::word("the")];
        let atoms = [
            LexPattern::phrase(&["sacrifice", "it", "at", "the", "beginning", "of"]),
            LexPattern::optional(&optional_the),
            LexPattern::phrase(&["next", "end", "step"]),
        ];
        let pattern = LexPattern::new(&atoms);

        assert!(pattern.matches_clause(clause));
    }

    #[test]
    fn pattern_supports_alternate_sequences() {
        let tokens = tokens(&["if", "that", "card", "remains", "exiled"]);
        let clause = LexedClause::new(&tokens);
        let plural = [
            LexPattern::phrase(&["if", "those", "cards"]),
            LexPattern::phrase(&["remain", "exiled"]),
        ];
        let singular = [
            LexPattern::phrase(&["if", "that", "card"]),
            LexPattern::phrase(&["remains", "exiled"]),
        ];
        let alternatives: &[&[LexPatternAtom<'_>]] = &[&plural, &singular];
        let atoms = [LexPattern::any_sequence(alternatives)];
        let pattern = LexPattern::new(&atoms);

        assert!(pattern.matches_clause(clause));
    }

    #[test]
    fn pattern_alternate_sequences_prefer_longest_match() {
        let tokens = tokens(&["if", "any", "of", "those", "cards", "remain", "exiled"]);
        let clause = LexedClause::new(&tokens);
        let short = [LexPattern::phrase(&["if"])];
        let long = [LexPattern::phrase(&["if", "any", "of", "those", "cards"])];
        let alternatives: &[&[LexPatternAtom<'_>]] = &[&short, &long];
        let atoms = [
            LexPattern::any_sequence(alternatives),
            LexPattern::capture("tail", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        assert_eq!(
            matched
                .capture_clause("tail", clause)
                .expect("tail")
                .word_refs(),
            ["remain", "exiled"]
        );
    }

    #[test]
    fn pattern_captures_named_roles() {
        let tokens = tokens(&["sacrifice", "that", "token", "at", "end", "of", "combat"]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::word("sacrifice"),
            LexPattern::object(
                "object",
                LexCaptureKind::UntilPhrase(&["at", "end", "of", "combat"]),
            ),
            LexPattern::phrase(&["at", "end", "of", "combat"]),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        let object = matched
            .capture_clause_by_role(LexCaptureRole::Object, clause)
            .expect("object role");
        assert_eq!(object.word_refs(), ["that", "token"]);
    }

    #[test]
    fn pattern_role_helpers_capture_subject_action_object() {
        let tokens = tokens(&["you", "control", "three", "artifacts"]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
            LexPattern::action("action", LexCaptureKind::OneOf(&["control", "controls"])),
            LexPattern::amount("amount", LexCaptureKind::WordCount(1)),
            LexPattern::object("object", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Subject, clause)
                .expect("subject")
                .word_refs(),
            ["you"]
        );
        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Action, clause)
                .expect("action")
                .word_refs(),
            ["control"]
        );
        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Object, clause)
                .expect("object")
                .word_refs(),
            ["artifacts"]
        );
    }

    #[test]
    fn pattern_can_capture_until_last_phrase() {
        let tokens = tokens(&[
            "return",
            "target",
            "card",
            "from",
            "your",
            "graveyard",
            "to",
            "the",
            "battlefield",
            "with",
            "a",
            "+1",
            "+1",
            "counter",
            "on",
            "it",
        ]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::word("return"),
            LexPattern::capture("object", LexCaptureKind::UntilLastPhrase(&["to"])),
            LexPattern::word("to"),
            LexPattern::capture("destination", LexCaptureKind::UntilPhrase(&["with"])),
            LexPattern::word("with"),
            LexPattern::capture("counter", LexCaptureKind::UntilLastPhrase(&["on"])),
            LexPattern::word("on"),
            LexPattern::capture("tail", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        assert_eq!(
            matched
                .capture_clause("object", clause)
                .expect("object")
                .word_refs(),
            ["target", "card", "from", "your", "graveyard"]
        );
        assert_eq!(
            matched
                .capture_clause("counter", clause)
                .expect("counter")
                .word_refs(),
            ["a", "+1", "+1", "counter"]
        );
    }

    #[test]
    fn pattern_can_capture_fixed_word_count() {
        let tokens = tokens(&["those", "instant", "spells"]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::word("those"),
            LexPattern::capture("card_type", LexCaptureKind::WordCount(1)),
            LexPattern::word("spells"),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        assert_eq!(
            matched
                .capture_clause("card_type", clause)
                .expect("card type")
                .word_refs(),
            ["instant"]
        );
    }

    #[test]
    fn pattern_can_capture_until_token_delimiter() {
        let tokens = crate::runtime_backend::lex_line(
            "This turn, whenever you draw a card, draw a card.",
            0,
        )
        .expect("delimiter text should lex");
        let clause = LexedClause::new(&tokens).trimmed();
        let atoms = [
            LexPattern::modifier(
                "duration",
                LexCaptureKind::OneOfPhrase(&[&["this", "turn"]]),
            ),
            LexPattern::action("intro", LexCaptureKind::OneOf(&["when", "whenever"])),
            LexPattern::condition("trigger", LexCaptureKind::UntilToken(TokenKind::Comma)),
            LexPattern::tail("effect", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");

        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Condition, clause)
                .expect("trigger")
                .word_refs(),
            ["you", "draw", "a", "card"]
        );
        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Tail, clause)
                .expect("effect")
                .word_refs(),
            ["draw", "a", "card"]
        );
    }

    #[test]
    fn pattern_can_capture_until_phrase_before_token_delimiter() {
        let tokens = crate::runtime_backend::lex_line(
            "Whenever you draw a card this turn, target creature can't block this turn.",
            0,
        )
        .expect("suffix delimiter text should lex");
        let clause = LexedClause::new(&tokens).trimmed();
        let atoms = [
            LexPattern::action("intro", LexCaptureKind::OneOf(&["when", "whenever"])),
            LexPattern::condition(
                "trigger",
                LexCaptureKind::UntilLastPhraseBeforeToken(&["this", "turn"], TokenKind::Comma),
            ),
            LexPattern::modifier(
                "duration",
                LexCaptureKind::OneOfPhrase(&[&["this", "turn"]]),
            ),
            LexPattern::token(TokenKind::Comma),
            LexPattern::tail("effect", LexCaptureKind::Rest),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");

        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Condition, clause)
                .expect("trigger")
                .word_refs(),
            ["you", "draw", "a", "card"]
        );
        assert_eq!(
            matched
                .capture_clause_by_role(LexCaptureRole::Tail, clause)
                .expect("effect")
                .word_refs(),
            ["target", "creature", "cant", "block", "this", "turn"]
        );
    }

    #[test]
    fn pattern_can_capture_one_of_set() {
        let tokens = tokens(&["from", "their", "hand"]);
        let clause = LexedClause::new(&tokens);
        let atoms = [
            LexPattern::word("from"),
            LexPattern::capture("owner", LexCaptureKind::OneOf(&["your", "their"])),
            LexPattern::word("hand"),
        ];
        let pattern = LexPattern::new(&atoms);

        let matched = pattern.match_clause(clause).expect("match");
        assert_eq!(
            matched
                .capture_clause("owner", clause)
                .expect("owner")
                .word_refs(),
            ["their"]
        );
    }
}
