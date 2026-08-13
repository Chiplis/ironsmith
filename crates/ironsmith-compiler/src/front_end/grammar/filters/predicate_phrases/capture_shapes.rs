use std::ops::Range;

use winnow::combinator::eof;
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::{any, take};

use super::super::super::super::lexer::LexedClause;
use super::super::super::primitives::{self, WordSliceInput};
use super::surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WinnowCaptureRole {
    Subject,
    Action,
    Object,
    Modifier,
    Condition,
    Amount,
    Tail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WinnowCaptureKind<'p> {
    Rest,
    WordCount(usize),
    OneOf(&'p [&'p str]),
    OneOfPhrase(&'p [&'p [&'p str]]),
    UntilPhrase(&'p [&'p str]),
    UntilLastPhrase(&'p [&'p str]),
    UntilAnyPhrase(&'p [&'p [&'p str]]),
    UntilLastAnyPhrase(&'p [&'p [&'p str]]),
    OneOrMoreWords,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WinnowAtom<'p> {
    Word(&'p str),
    AnyWord(&'p [&'p str]),
    Phrase(&'p [&'p str]),
    AnyPhrase(&'p [&'p [&'p str]]),
    Optional(&'p [WinnowAtom<'p>]),
    Capture(&'p str, WinnowCaptureKind<'p>),
    RoleCapture(&'p str, WinnowCaptureRole, WinnowCaptureKind<'p>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WinnowSequence<'p> {
    atoms: &'p [WinnowAtom<'p>],
}

impl<'p> WinnowSequence<'p> {
    pub(crate) const fn new(atoms: &'p [WinnowAtom<'p>]) -> Self {
        Self { atoms }
    }

    pub(crate) const fn word(word: &'p str) -> WinnowAtom<'p> {
        WinnowAtom::Word(word)
    }

    pub(crate) const fn any_word(words: &'p [&'p str]) -> WinnowAtom<'p> {
        WinnowAtom::AnyWord(words)
    }

    pub(crate) const fn phrase(words: &'p [&'p str]) -> WinnowAtom<'p> {
        WinnowAtom::Phrase(words)
    }

    pub(crate) const fn any_phrase(phrases: &'p [&'p [&'p str]]) -> WinnowAtom<'p> {
        WinnowAtom::AnyPhrase(phrases)
    }

    pub(crate) const fn optional(atoms: &'p [WinnowAtom<'p>]) -> WinnowAtom<'p> {
        WinnowAtom::Optional(atoms)
    }

    pub(crate) const fn capture(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        WinnowAtom::Capture(name, kind)
    }

    pub(crate) const fn role_capture(
        name: &'p str,
        role: WinnowCaptureRole,
        kind: WinnowCaptureKind<'p>,
    ) -> WinnowAtom<'p> {
        WinnowAtom::RoleCapture(name, role, kind)
    }

    pub(crate) const fn subject(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        Self::role_capture(name, WinnowCaptureRole::Subject, kind)
    }

    pub(crate) const fn action(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        Self::role_capture(name, WinnowCaptureRole::Action, kind)
    }

    pub(crate) const fn object(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        Self::role_capture(name, WinnowCaptureRole::Object, kind)
    }

    pub(crate) const fn amount(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        Self::role_capture(name, WinnowCaptureRole::Amount, kind)
    }

    pub(crate) const fn modifier(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        Self::role_capture(name, WinnowCaptureRole::Modifier, kind)
    }

    pub(crate) const fn condition(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        Self::role_capture(name, WinnowCaptureRole::Condition, kind)
    }

    pub(crate) const fn tail(name: &'p str, kind: WinnowCaptureKind<'p>) -> WinnowAtom<'p> {
        Self::role_capture(name, WinnowCaptureRole::Tail, kind)
    }

    pub(crate) fn accepts_full(self, clause: LexedClause<'_>) -> bool {
        self.parse_full(clause).is_some()
    }

    pub(crate) fn parse_full<'a>(self, clause: LexedClause<'a>) -> Option<WinnowSequenceMatch<'p>> {
        self.parse_from(clause, 0, true)
    }

    pub(crate) fn parse_prefix<'a>(
        self,
        clause: LexedClause<'a>,
    ) -> Option<WinnowSequenceMatch<'p>> {
        self.parse_from(clause, 0, false)
    }

    pub(crate) fn locate_in<'a>(self, clause: LexedClause<'a>) -> Option<WinnowSequenceMatch<'p>> {
        let words = clause.word_refs();
        for start in 0..=words.len() {
            if let Some(parsed) = self.parse_from_words(&words, start, false) {
                return Some(parsed);
            }
        }
        None
    }

    fn parse_from(
        self,
        clause: LexedClause<'_>,
        start: usize,
        require_end: bool,
    ) -> Option<WinnowSequenceMatch<'p>> {
        let words = clause.word_refs();
        self.parse_from_words(&words, start, require_end)
    }

    fn parse_from_words(
        self,
        words: &[&str],
        start: usize,
        require_end: bool,
    ) -> Option<WinnowSequenceMatch<'p>> {
        let mut input: WordSliceInput<'_> = words.get(start..)?;
        let mut captures = Vec::new();
        parse_atoms(self.atoms, words.len(), &mut input, &mut captures).ok()?;
        if require_end {
            parse_end(&mut input).ok()?;
        }
        let end = words.len().checked_sub(input.len())?;
        Some(WinnowSequenceMatch {
            word_range: start..end,
            captures,
        })
    }
}

fn parse_atoms<'a, 'p>(
    atoms: &[WinnowAtom<'p>],
    full_word_count: usize,
    input: &mut WordSliceInput<'a>,
    captures: &mut Vec<WinnowSequenceCapture<'p>>,
) -> WResult<()> {
    for atom in atoms {
        match *atom {
            WinnowAtom::Word(expected) => {
                dynamic_word(expected).void().parse_next(input)?;
            }
            WinnowAtom::AnyWord(expected) => {
                any.verify(|word: &&str| one_of_words(word, expected))
                    .void()
                    .parse_next(input)?;
            }
            WinnowAtom::Phrase(expected) => {
                dynamic_sequence(expected).parse_next(input)?;
            }
            WinnowAtom::AnyPhrase(alternatives) => {
                parse_longest_sequence(alternatives, input)?;
            }
            WinnowAtom::Optional(optional_atoms) => {
                let mut probe = *input;
                let mut optional_captures = captures.clone();
                if parse_atoms(
                    optional_atoms,
                    full_word_count,
                    &mut probe,
                    &mut optional_captures,
                )
                .is_ok()
                {
                    *input = probe;
                    *captures = optional_captures;
                }
            }
            WinnowAtom::Capture(name, kind) => {
                parse_capture(name, None, kind, full_word_count, input, captures)?;
            }
            WinnowAtom::RoleCapture(name, role, kind) => {
                parse_capture(name, Some(role), kind, full_word_count, input, captures)?;
            }
        }
    }
    Ok(())
}

fn parse_capture<'a, 'p>(
    name: &'p str,
    role: Option<WinnowCaptureRole>,
    kind: WinnowCaptureKind<'p>,
    full_word_count: usize,
    input: &mut WordSliceInput<'a>,
    captures: &mut Vec<WinnowSequenceCapture<'p>>,
) -> WResult<()> {
    let start = full_word_count.saturating_sub(input.len());
    match kind {
        WinnowCaptureKind::Rest => {
            let count = input.len();
            take_words(input, count)?;
        }
        WinnowCaptureKind::OneOrMoreWords => {
            if input.is_empty() {
                return Err(primitives::backtrack_err(
                    "predicate capture",
                    "one or more words",
                ));
            }
            let count = input.len();
            take_words(input, count)?;
        }
        WinnowCaptureKind::WordCount(count) => take_words(input, count)?,
        WinnowCaptureKind::OneOf(expected) => {
            any.verify(|word: &&str| one_of_words(word, expected))
                .void()
                .parse_next(input)?;
        }
        WinnowCaptureKind::OneOfPhrase(alternatives) => {
            parse_first_sequence(alternatives, input)?;
        }
        WinnowCaptureKind::UntilPhrase(expected) => {
            let count = surface::find_words(input, expected)
                .ok_or_else(|| primitives::backtrack_err("predicate capture", "following words"))?;
            take_words(input, count)?;
        }
        WinnowCaptureKind::UntilLastPhrase(expected) => {
            let count = last_sequence_offset(input, expected)
                .ok_or_else(|| primitives::backtrack_err("predicate capture", "following words"))?;
            take_words(input, count)?;
        }
        WinnowCaptureKind::UntilAnyPhrase(alternatives) => {
            let count = first_alternative_offset(input, alternatives)
                .ok_or_else(|| primitives::backtrack_err("predicate capture", "following words"))?;
            take_words(input, count)?;
        }
        WinnowCaptureKind::UntilLastAnyPhrase(alternatives) => {
            let count = last_alternative_offset(input, alternatives)
                .ok_or_else(|| primitives::backtrack_err("predicate capture", "following words"))?;
            take_words(input, count)?;
        }
    }
    let end = full_word_count.saturating_sub(input.len());
    captures.push(WinnowSequenceCapture {
        name,
        role,
        word_range: start..end,
    });
    Ok(())
}

fn dynamic_sequence<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut WordSliceInput<'a>| {
        for word in expected {
            dynamic_word(word).void().parse_next(input)?;
        }
        Ok(())
    }
}

fn dynamic_word<'a, 'p>(
    expected: &'p str,
) -> impl Parser<WordSliceInput<'a>, &'a str, ErrMode<ContextError>> + 'p {
    move |input: &mut WordSliceInput<'a>| {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err("predicate word", "expected word"));
        };
        if *word != expected {
            return Err(primitives::backtrack_err("predicate word", "expected word"));
        }
        *input = rest;
        Ok(*word)
    }
}

fn parse_end(input: &mut WordSliceInput<'_>) -> WResult<()> {
    eof.void().parse_next(input)
}

fn parse_longest_sequence<'a>(
    alternatives: &[&[&str]],
    input: &mut WordSliceInput<'a>,
) -> WResult<()> {
    let mut best = None;
    for expected in alternatives {
        let mut probe = *input;
        if dynamic_sequence(expected).parse_next(&mut probe).is_ok()
            && best
                .as_ref()
                .is_none_or(|current: &&[&str]| probe.len() < current.len())
        {
            best = Some(probe);
        }
    }
    let Some(parsed) = best else {
        return Err(primitives::backtrack_err(
            "predicate words",
            "one of the expected sequences",
        ));
    };
    *input = parsed;
    Ok(())
}

fn parse_first_sequence<'a>(
    alternatives: &[&[&str]],
    input: &mut WordSliceInput<'a>,
) -> WResult<()> {
    for expected in alternatives {
        let mut probe = *input;
        if dynamic_sequence(expected).parse_next(&mut probe).is_ok() {
            *input = probe;
            return Ok(());
        }
    }
    Err(primitives::backtrack_err(
        "predicate words",
        "one of the expected sequences",
    ))
}

fn take_words(input: &mut WordSliceInput<'_>, count: usize) -> WResult<()> {
    let _: &[&str] = take(count).parse_next(input)?;
    Ok(())
}

fn first_alternative_offset(words: &[&str], alternatives: &[&[&str]]) -> Option<usize> {
    let mut best = None;
    for expected in alternatives {
        if let Some(offset) = surface::find_words(words, expected)
            && best.is_none_or(|current| offset < current)
        {
            best = Some(offset);
        }
    }
    best
}

fn last_alternative_offset(words: &[&str], alternatives: &[&[&str]]) -> Option<usize> {
    let mut best = None;
    for expected in alternatives {
        if let Some(offset) = last_sequence_offset(words, expected)
            && best.is_none_or(|current| offset > current)
        {
            best = Some(offset);
        }
    }
    best
}

fn last_sequence_offset(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() || expected.len() > words.len() {
        return None;
    }
    let reversed_words: Vec<&str> = words.iter().rev().copied().collect();
    let reversed_expected: Vec<&str> = expected.iter().rev().copied().collect();
    let reverse_prefix = surface::find_words(&reversed_words, &reversed_expected)?;
    words
        .len()
        .checked_sub(reverse_prefix.checked_add(expected.len())?)
}

fn one_of_words(word: &str, expected: &[&str]) -> bool {
    for candidate in expected {
        if *candidate == word {
            return true;
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WinnowSequenceMatch<'p> {
    pub(crate) word_range: Range<usize>,
    captures: Vec<WinnowSequenceCapture<'p>>,
}

impl<'p> WinnowSequenceMatch<'p> {
    pub(crate) fn capture(&self, name: &str) -> Option<&WinnowSequenceCapture<'p>> {
        for capture in &self.captures {
            if capture.name == name {
                return Some(capture);
            }
        }
        None
    }

    pub(crate) fn capture_by_role(
        &self,
        role: WinnowCaptureRole,
    ) -> Option<&WinnowSequenceCapture<'p>> {
        for capture in &self.captures {
            if capture.role == Some(role) {
                return Some(capture);
            }
        }
        None
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
        role: WinnowCaptureRole,
        clause: LexedClause<'a>,
    ) -> Option<LexedClause<'a>> {
        self.capture_by_role(role)
            .and_then(|capture| capture.clause(clause))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WinnowSequenceCapture<'p> {
    pub(crate) name: &'p str,
    pub(crate) role: Option<WinnowCaptureRole>,
    pub(crate) word_range: Range<usize>,
}

impl WinnowSequenceCapture<'_> {
    pub(crate) fn clause<'a>(&self, clause: LexedClause<'a>) -> Option<LexedClause<'a>> {
        clause.between_word_range(self.word_range.start, self.word_range.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn winnow_sequence_returns_typed_capture_ranges() {
        let tokens = lex_line("target creature you control gets plus one", 0).expect("lex fixture");
        let clause = LexedClause::new(&tokens);
        let atoms = [
            WinnowSequence::word("target"),
            WinnowSequence::capture("object", WinnowCaptureKind::UntilPhrase(&["gets"])),
            WinnowSequence::word("gets"),
            WinnowSequence::capture("tail", WinnowCaptureKind::Rest),
        ];
        let parsed = WinnowSequence::new(&atoms)
            .parse_full(clause)
            .expect("typed sequence");
        assert_eq!(
            parsed.capture_clause("object", clause).unwrap().word_refs(),
            ["creature", "you", "control"]
        );
    }
}
