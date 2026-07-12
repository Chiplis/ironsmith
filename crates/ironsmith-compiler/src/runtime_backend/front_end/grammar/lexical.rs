use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::{any, take};

use crate::cards::builders::TextSpan;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordPiece, trim_lexed_commas};
use super::primitives as grammar;

type WordInput<'a> = &'a [&'a str];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedWordSequence<'p> {
    sequence: &'p [&'p str],
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedTokenSequence<'p> {
    sequence: &'p [&'p str],
    start: usize,
    end: usize,
}

fn same_word_sequence(actual: &[&str], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .copied()
            .zip(expected.iter().copied())
            .all(|(actual_word, expected_word)| actual_word == expected_word)
}

fn word_sequence_parser<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<WordInput<'a>, ParsedWordSequence<'p>, ErrMode<ContextError>> + 'p {
    move |input: &mut WordInput<'a>| {
        let actual: &[&str] = take(expected.len()).parse_next(input)?;
        if !same_word_sequence(actual, expected) {
            return Err(grammar::backtrack_err(
                "word sequence",
                "expected word sequence",
            ));
        }
        Ok(ParsedWordSequence {
            sequence: expected,
            start: 0,
            end: expected.len(),
        })
    }
}

fn parse_word_sequence_at<'p>(
    words: &[&str],
    start: usize,
    expected: &'p [&'p str],
) -> Option<ParsedWordSequence<'p>> {
    let mut input = words.get(start..)?;
    let mut parsed = word_sequence_parser(expected).parse_next(&mut input).ok()?;
    parsed.start = start;
    parsed.end = start + expected.len();
    Some(parsed)
}

fn parse_complete_word_sequence<'p>(
    words: &[&str],
    expected: &'p [&'p str],
) -> Option<ParsedWordSequence<'p>> {
    let mut input = words;
    let parsed = (word_sequence_parser(expected), eof)
        .map(|(parsed, _)| parsed)
        .parse_next(&mut input)
        .ok()?;
    Some(parsed)
}

fn parse_first_word_sequence<'p>(
    words: &[&str],
    expected: &'p [&'p str],
) -> Option<ParsedWordSequence<'p>> {
    let mut input = words;
    let mut parsed = repeat_till(0.., any.void(), word_sequence_parser(expected))
        .map(|((), parsed)| parsed)
        .parse_next(&mut input)
        .ok()?;
    parsed.end = words.len().saturating_sub(input.len());
    parsed.start = parsed.end.saturating_sub(expected.len());
    Some(parsed)
}

fn parse_first_word_sequence_choice<'p>(
    words: &[&str],
    expected: &[&'p [&'p str]],
) -> Option<ParsedWordSequence<'p>> {
    let mut best: Option<ParsedWordSequence<'p>> = None;
    for sequence in expected {
        if sequence.is_empty() {
            continue;
        }
        let Some(candidate) = parse_first_word_sequence(words, sequence) else {
            continue;
        };
        let replace = best
            .as_ref()
            .is_none_or(|current| candidate.start < current.start);
        if replace {
            best = Some(candidate);
        }
    }
    best
}

fn parse_complete_word_sequence_choice<'p>(
    words: &[&str],
    expected: &[&'p [&'p str]],
) -> Option<ParsedWordSequence<'p>> {
    for sequence in expected {
        if let Some(parsed) = parse_complete_word_sequence(words, sequence) {
            return Some(parsed);
        }
    }
    None
}

fn parse_word_choice_at<'p>(
    words: &[&str],
    start: usize,
    expected: &'p [&'p str],
) -> Option<&'p str> {
    let actual = *words.get(start)?;
    for candidate in expected {
        if actual == *candidate {
            return Some(candidate);
        }
    }
    None
}

fn parse_first_word_choice<'p>(
    words: &[&str],
    expected: &'p [&'p str],
) -> Option<(usize, &'p str)> {
    let mut input = words;
    let mut index = 0usize;
    while !input.is_empty() {
        let parsed: Result<&str, ErrMode<ContextError>> = any.parse_next(&mut input);
        let word = parsed.ok()?;
        for candidate in expected {
            if word == *candidate {
                return Some((index, candidate));
            }
        }
        index += 1;
    }
    None
}

fn parse_last_word_choice<'p>(words: &[&str], expected: &'p [&'p str]) -> Option<(usize, &'p str)> {
    let mut input = words;
    let mut index = 0usize;
    let mut last = None;
    while !input.is_empty() {
        let parsed: Result<&str, ErrMode<ContextError>> = any.parse_next(&mut input);
        let word = parsed.ok()?;
        for candidate in expected {
            if word == *candidate {
                last = Some((index, *candidate));
                break;
            }
        }
        index += 1;
    }
    last
}

fn parse_word_boundary_by(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    let mut input = words;
    let mut index = 0usize;
    while !input.is_empty() {
        let parsed: Result<&str, ErrMode<ContextError>> = any.parse_next(&mut input);
        let word = parsed.ok()?;
        if predicate(word) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_last_word_boundary_by(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    let mut input = words;
    let mut index = 0usize;
    let mut last = None;
    while !input.is_empty() {
        let parsed: Result<&str, ErrMode<ContextError>> = any.parse_next(&mut input);
        let word = parsed.ok()?;
        if predicate(word) {
            last = Some(index);
        }
        index += 1;
    }
    last
}

fn parse_word_window_boundary(
    words: &[&str],
    window_len: usize,
    mut predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    if window_len == 0 || window_len > words.len() {
        return None;
    }
    let mut start = 0usize;
    while start + window_len <= words.len() {
        let mut input = &words[start..];
        let parsed: Result<&[&str], ErrMode<ContextError>> =
            take(window_len).parse_next(&mut input);
        if parsed.ok().is_some_and(&mut predicate) {
            return Some(start);
        }
        start += 1;
    }
    None
}

fn same_token_sequence(actual: &[OwnedLexToken], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter().copied())
            .all(|(token, expected_word)| token.is_word(expected_word))
}

fn token_sequence_parser<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<LexStream<'a>, ParsedTokenSequence<'p>, ErrMode<ContextError>> + 'p {
    move |input: &mut LexStream<'a>| {
        let actual: &[OwnedLexToken] = take(expected.len()).parse_next(input)?;
        if !same_token_sequence(actual, expected) {
            return Err(grammar::backtrack_err(
                "token sequence",
                "expected token sequence",
            ));
        }
        Ok(ParsedTokenSequence {
            sequence: expected,
            start: 0,
            end: expected.len(),
        })
    }
}

fn parse_first_token_sequence<'p>(
    tokens: &[OwnedLexToken],
    expected: &'p [&'p str],
) -> Option<ParsedTokenSequence<'p>> {
    if expected.is_empty() {
        return None;
    }
    let mut input = LexStream::new(tokens);
    let mut parsed = repeat_till(0.., any.void(), token_sequence_parser(expected))
        .map(|((), parsed)| parsed)
        .parse_next(&mut input)
        .ok()?;
    parsed.end = tokens.len().saturating_sub(input.len());
    parsed.start = parsed.end.saturating_sub(expected.len());
    Some(parsed)
}

fn parse_token_sequence_at<'p>(
    tokens: &[OwnedLexToken],
    start: usize,
    expected: &'p [&'p str],
) -> Option<ParsedTokenSequence<'p>> {
    let tail = tokens.get(start..)?;
    let mut input = LexStream::new(tail);
    let mut parsed = token_sequence_parser(expected)
        .parse_next(&mut input)
        .ok()?;
    parsed.start = start;
    parsed.end = start + expected.len();
    Some(parsed)
}

fn parse_token_boundary_by(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let mut index = 0usize;
    while !input.is_empty() {
        let parsed: Result<&OwnedLexToken, ErrMode<ContextError>> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if predicate(token) {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_last_token_boundary_by(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let mut index = 0usize;
    let mut last = None;
    while !input.is_empty() {
        let parsed: Result<&OwnedLexToken, ErrMode<ContextError>> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if predicate(token) {
            last = Some(index);
        }
        index += 1;
    }
    last
}

pub(crate) fn token_word_pieces_for_token(token: &OwnedLexToken) -> &[TokenWordPiece] {
    token.parser_word_pieces()
}

pub(crate) fn locate_word_sequence(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() {
        return None;
    }
    parse_first_word_sequence(words, expected).map(|parsed| parsed.start)
}

pub(crate) fn word_slice_find_phrase_start_or_zero(
    words: &[&str],
    expected: &[&str],
) -> Option<usize> {
    if expected.is_empty() {
        Some(0)
    } else {
        parse_first_word_sequence(words, expected).map(|parsed| parsed.start)
    }
}

pub(crate) fn word_slice_find_any_phrase_start<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    parse_first_word_sequence_choice(words, expected).map(|parsed| (parsed.sequence, parsed.start))
}

pub(crate) fn locate_word_sequence_choice_span(
    words: &[&str],
    expected: &[&[&str]],
) -> Option<(usize, usize)> {
    parse_first_word_sequence_choice(words, expected)
        .map(|parsed| (parsed.start, parsed.end.saturating_sub(parsed.start)))
}

pub(crate) fn word_slice_find_phrase_value<T: Clone>(
    words: &[&str],
    expected: &[(&[&str], T)],
) -> Option<(T, usize)> {
    let mut best: Option<(T, usize)> = None;
    for (sequence, value) in expected {
        if sequence.is_empty() {
            continue;
        }
        let Some(parsed) = parse_first_word_sequence(words, sequence) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, current_start)| parsed.start < *current_start)
        {
            best = Some((value.clone(), parsed.start));
        }
    }
    best
}

pub(crate) fn word_slice_find_any_phrase_start_or_zero<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    let mut best: Option<(&'p [&'p str], usize)> = None;
    for sequence in expected {
        let candidate = if sequence.is_empty() {
            Some(0)
        } else {
            parse_first_word_sequence(words, sequence).map(|parsed| parsed.start)
        };
        let Some(start) = candidate else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, current_start)| start < *current_start)
        {
            best = Some((sequence, start));
        }
    }
    best
}

pub(crate) fn word_slice_find_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    parse_word_window_boundary(words, window_len, predicate)
}

pub(crate) fn word_slice_has_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> bool {
    parse_word_window_boundary(words, window_len, predicate).is_some()
}

pub(crate) fn word_sequence_present(words: &[&str], expected: &[&str]) -> bool {
    !expected.is_empty() && parse_first_word_sequence(words, expected).is_some()
}

pub(crate) fn word_sequence_or_empty_present(words: &[&str], expected: &[&str]) -> bool {
    expected.is_empty() || parse_first_word_sequence(words, expected).is_some()
}

pub(crate) fn word_sequence_choice_present(words: &[&str], expected: &[&[&str]]) -> bool {
    parse_first_word_sequence_choice(words, expected).is_some()
}

pub(crate) fn word_sequence_choice_or_empty_present(words: &[&str], expected: &[&[&str]]) -> bool {
    for sequence in expected {
        if sequence.is_empty() || parse_first_word_sequence(words, sequence).is_some() {
            return true;
        }
    }
    false
}

pub(crate) fn complete_word_sequence_surface(words: &[&str], expected: &[&str]) -> bool {
    parse_complete_word_sequence(words, expected).is_some()
}

pub(crate) fn complete_word_sequence_choice(words: &[&str], expected: &[&[&str]]) -> bool {
    parse_complete_word_sequence_choice(words, expected).is_some()
}

pub(crate) fn complete_word_sequence_at(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words
        .get(idx..)
        .is_some_and(|tail| parse_complete_word_sequence(tail, expected).is_some())
}

pub(crate) fn complete_word_sequence_choice_at(
    words: &[&str],
    idx: usize,
    expected: &[&[&str]],
) -> bool {
    words
        .get(idx..)
        .is_some_and(|tail| parse_complete_word_sequence_choice(tail, expected).is_some())
}

pub(crate) fn word_slice_at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    words.get(idx).is_some_and(|word| *word == expected)
}

pub(crate) fn word_slice_at_is_any(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    parse_word_choice_at(words, idx, expected).is_some()
}

pub(crate) fn word_slice_first_is(words: &[&str], expected: &str) -> bool {
    words.first().is_some_and(|word| *word == expected)
}

pub(crate) fn word_slice_first_is_any(words: &[&str], expected: &[&str]) -> bool {
    parse_word_choice_at(words, 0, expected).is_some()
}

pub(crate) fn word_slice_last_is(words: &[&str], expected: &str) -> bool {
    words.last().is_some_and(|word| *word == expected)
}

pub(crate) fn word_slice_last_is_any(words: &[&str], expected: &[&str]) -> bool {
    words
        .len()
        .checked_sub(1)
        .and_then(|idx| parse_word_choice_at(words, idx, expected))
        .is_some()
}

pub(crate) fn word_slice_matching_phrase<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<&'p [&'p str]> {
    parse_complete_word_sequence_choice(words, expected).map(|parsed| parsed.sequence)
}

pub(crate) fn word_slice_matching_prefix<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<&'p [&'p str]> {
    for sequence in expected {
        if parse_word_sequence_at(words, 0, sequence).is_some() {
            return Some(sequence);
        }
    }
    None
}

pub(crate) fn word_slice_matching_value<T: Clone>(
    words: &[&str],
    expected: &[(&[&str], T)],
) -> Option<T> {
    for (sequence, value) in expected {
        if parse_complete_word_sequence(words, sequence).is_some() {
            return Some(value.clone());
        }
    }
    None
}

pub(crate) fn word_suffix_present(words: &[&str], expected: &[&str]) -> bool {
    words
        .len()
        .checked_sub(expected.len())
        .and_then(|start| parse_word_sequence_at(words, start, expected))
        .is_some()
}

pub(crate) fn word_suffix_choice_present(words: &[&str], expected: &[&[&str]]) -> bool {
    for sequence in expected {
        if word_suffix_present(words, sequence) {
            return true;
        }
    }
    false
}

pub(crate) fn word_prefix_present(words: &[&str], expected: &[&str]) -> bool {
    parse_word_sequence_at(words, 0, expected).is_some()
}

pub(crate) fn word_prefix_present_at(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    parse_word_sequence_at(words, idx, expected).is_some()
}

pub(crate) fn word_prefix_choice_present(words: &[&str], expected: &[&[&str]]) -> bool {
    for sequence in expected {
        if parse_word_sequence_at(words, 0, sequence).is_some() {
            return true;
        }
    }
    false
}

pub(crate) fn word_slice_strip_prefix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    parse_word_sequence_at(words, 0, expected)?;
    Some(&words[expected.len()..])
}

pub(crate) fn word_slice_strip_suffix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    let start = words.len().checked_sub(expected.len())?;
    parse_word_sequence_at(words, start, expected)?;
    Some(&words[..start])
}

pub(crate) fn word_slice_strip_any_prefix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    for sequence in expected {
        if parse_word_sequence_at(words, 0, sequence).is_some() {
            return Some((sequence, &words[sequence.len()..]));
        }
    }
    None
}

pub(crate) fn word_slice_strip_prefix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    for (sequence, value) in expected {
        if parse_word_sequence_at(words, 0, sequence).is_some() {
            return Some((value.clone(), &words[sequence.len()..]));
        }
    }
    None
}

pub(crate) fn word_slice_strip_first_word<'w, 'a>(
    words: &'w [&'a str],
    expected: &str,
) -> Option<&'w [&'a str]> {
    words
        .first()
        .is_some_and(|word| *word == expected)
        .then_some(&words[1..])
}

pub(crate) fn word_slice_strip_first_word_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&str, T)],
) -> Option<(T, &'w [&'a str])> {
    let first = words.first()?;
    for (word, value) in expected {
        if first == word {
            return Some((value.clone(), &words[1..]));
        }
    }
    None
}

pub(crate) fn word_slice_strip_any_suffix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    for sequence in expected {
        let Some(start) = words.len().checked_sub(sequence.len()) else {
            continue;
        };
        if parse_word_sequence_at(words, start, sequence).is_some() {
            return Some((sequence, &words[..start]));
        }
    }
    None
}

pub(crate) fn word_slice_strip_suffix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    for (sequence, value) in expected {
        let Some(start) = words.len().checked_sub(sequence.len()) else {
            continue;
        };
        if parse_word_sequence_at(words, start, sequence).is_some() {
            return Some((value.clone(), &words[..start]));
        }
    }
    None
}

pub(crate) fn word_present(words: &[&str], expected: &str) -> bool {
    parse_first_word_choice(words, &[expected]).is_some()
}

pub(crate) fn locate_word(words: &[&str], expected: &str) -> Option<usize> {
    parse_first_word_choice(words, &[expected]).map(|(idx, _)| idx)
}

pub(crate) fn locate_word_choice(words: &[&str], expected: &[&str]) -> Option<usize> {
    parse_first_word_choice(words, expected).map(|(idx, _)| idx)
}

pub(crate) fn word_slice_find_any_word_from(
    words: &[&str],
    expected: &[&str],
    start: usize,
) -> Option<usize> {
    let tail = words.get(start..)?;
    parse_first_word_choice(tail, expected).map(|(idx, _)| start + idx)
}

pub(crate) fn locate_word_by(words: &[&str], predicate: impl FnMut(&str) -> bool) -> Option<usize> {
    parse_word_boundary_by(words, predicate)
}

pub(crate) fn locate_last_word_by(
    words: &[&str],
    predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    parse_last_word_boundary_by(words, predicate)
}

pub(crate) fn word_choice_present(words: &[&str], expected: &[&str]) -> bool {
    parse_first_word_choice(words, expected).is_some()
}

pub(crate) fn word_choice_absent(words: &[&str], expected: &[&str]) -> bool {
    parse_first_word_choice(words, expected).is_none()
}

pub(crate) fn every_word_present(words: &[&str], expected: &[&str]) -> bool {
    for word in expected {
        if parse_first_word_choice(words, &[*word]).is_none() {
            return false;
        }
    }
    true
}

pub(crate) fn word_slice_all_words_are_any(words: &[&str], expected: &[&str]) -> bool {
    for word in words {
        if parse_first_word_choice(expected, &[*word]).is_none() {
            return false;
        }
    }
    true
}

pub(crate) fn find_token_word_sequence(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<usize> {
    parse_first_token_sequence(tokens, expected).map(|parsed| parsed.start)
}

pub(crate) fn find_token_word_sequence_span(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<(usize, usize)> {
    parse_first_token_sequence(tokens, expected).map(|parsed| (parsed.start, parsed.end))
}

pub(crate) fn find_any_token_word_sequence_span<'p>(
    tokens: &[OwnedLexToken],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize, usize)> {
    let mut best: Option<ParsedTokenSequence<'p>> = None;
    for sequence in expected {
        let Some(candidate) = parse_first_token_sequence(tokens, sequence) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|current| candidate.start < current.start)
        {
            best = Some(candidate);
        }
    }
    best.map(|parsed| (parsed.sequence, parsed.start, parsed.end))
}

pub(crate) fn find_token_word_sequence_value<T: Clone>(
    tokens: &[OwnedLexToken],
    expected: &[(&[&str], T)],
) -> Option<(T, usize, usize)> {
    let mut best: Option<(T, usize, usize)> = None;
    for (sequence, value) in expected {
        let Some(parsed) = parse_first_token_sequence(tokens, sequence) else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|(_, current_start, _)| parsed.start < *current_start)
        {
            best = Some((value.clone(), parsed.start, parsed.end));
        }
    }
    best
}

pub(crate) fn contains_token_word_sequence(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    parse_first_token_sequence(tokens, expected).is_some()
}

pub(crate) fn token_slice_at_is(tokens: &[OwnedLexToken], idx: usize, expected: &str) -> bool {
    tokens.get(idx).is_some_and(|token| token.is_word(expected))
}

pub(crate) fn token_slice_at_is_any(
    tokens: &[OwnedLexToken],
    idx: usize,
    expected: &[&str],
) -> bool {
    tokens
        .get(idx)
        .is_some_and(|token| token.is_any_word(expected))
}

pub(crate) fn token_slice_at_kind(
    tokens: &[OwnedLexToken],
    idx: usize,
    expected: TokenKind,
) -> bool {
    tokens.get(idx).is_some_and(|token| token.kind == expected)
}

pub(crate) fn token_slice_first_is(tokens: &[OwnedLexToken], expected: &str) -> bool {
    token_slice_at_is(tokens, 0, expected)
}

pub(crate) fn token_slice_first_is_any(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    token_slice_at_is_any(tokens, 0, expected)
}

pub(crate) fn token_slice_first_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    token_slice_at_kind(tokens, 0, expected)
}

pub(crate) fn token_slice_last_is(tokens: &[OwnedLexToken], expected: &str) -> bool {
    tokens.last().is_some_and(|token| token.is_word(expected))
}

pub(crate) fn token_slice_last_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    tokens.last().is_some_and(|token| token.kind == expected)
}

pub(crate) fn token_prefix_present(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    parse_token_sequence_at(tokens, 0, expected).is_some()
}

pub(crate) fn token_prefix_present_at(
    tokens: &[OwnedLexToken],
    idx: usize,
    expected: &[&str],
) -> bool {
    parse_token_sequence_at(tokens, idx, expected).is_some()
}

pub(crate) fn token_prefix_choice_present(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    for sequence in expected {
        if parse_token_sequence_at(tokens, 0, sequence).is_some() {
            return true;
        }
    }
    false
}

pub(crate) fn complete_token_word_sequence(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    parse_complete_word_sequence(&words, expected).is_some()
}

pub(crate) fn complete_token_word_sequence_choice(
    tokens: &[OwnedLexToken],
    expected: &[&[&str]],
) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    parse_complete_word_sequence_choice(&words, expected).is_some()
}

pub(crate) fn token_word_suffix_present(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    words
        .len()
        .checked_sub(expected.len())
        .and_then(|start| parse_word_sequence_at(&words, start, expected))
        .is_some()
}

pub(crate) fn token_slice_ends_with_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    for sequence in expected {
        let Some(start) = words.len().checked_sub(sequence.len()) else {
            continue;
        };
        if parse_word_sequence_at(&words, start, sequence).is_some() {
            return true;
        }
    }
    false
}

pub(crate) fn token_slice_strip_word_prefix<'a>(
    tokens: &'a [OwnedLexToken],
    expected: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    if expected.is_empty() {
        return Some(tokens);
    }
    let view = TokenWordView::new(tokens);
    parse_word_sequence_at(&view.word_refs(), 0, expected)?;
    let token_end = view.token_index_after_words(expected.len())?;
    Some(&tokens[token_end..])
}

pub(crate) fn token_slice_strip_any_word_prefix<'a, 'p>(
    tokens: &'a [OwnedLexToken],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [OwnedLexToken])> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    for sequence in expected {
        if parse_word_sequence_at(&words, 0, sequence).is_none() {
            continue;
        }
        let token_end = view.token_index_after_words(sequence.len())?;
        return Some((sequence, &tokens[token_end..]));
    }
    None
}

pub(crate) fn locate_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    parse_token_boundary_by(tokens, |token| token.is_word(expected))
}

pub(crate) fn locate_token_word_choice(
    tokens: &[OwnedLexToken],
    expected: &[&str],
) -> Option<usize> {
    parse_token_boundary_by(tokens, |token| token.is_any_word(expected))
}

pub(crate) fn locate_last_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    parse_last_token_boundary_by(tokens, |token| token.is_word(expected))
}

pub(crate) fn contains_token_word(tokens: &[OwnedLexToken], expected: &str) -> bool {
    locate_token_word(tokens, expected).is_some()
}

pub(crate) fn contains_token_any_word(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    locate_token_word_choice(tokens, expected).is_some()
}

pub(crate) fn locate_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> Option<usize> {
    parse_token_boundary_by(tokens, |token| token.kind == expected)
}

pub(crate) fn contains_token_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    locate_token_kind(tokens, expected).is_some()
}

pub(crate) fn token_slice_all_are_kind(tokens: &[OwnedLexToken], expected: TokenKind) -> bool {
    let mut input = LexStream::new(tokens);
    while !input.is_empty() {
        let parsed: Result<&OwnedLexToken, ErrMode<ContextError>> = any.parse_next(&mut input);
        if parsed.ok().is_none_or(|token| token.kind != expected) {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenWordView<'a> {
    words: Vec<&'a str>,
    token_start_indices: Vec<usize>,
    token_end_indices: Vec<usize>,
    token_len: usize,
}

impl<'a> TokenWordView<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        let mut words = Vec::new();
        let mut token_start_indices = Vec::new();
        let mut token_end_indices = Vec::new();
        let mut token_idx = 0usize;
        while token_idx < tokens.len() {
            let token = &tokens[token_idx];
            let pieces = token_word_pieces_for_token(token);
            if pieces.is_empty() {
                token_idx += 1;
                continue;
            }
            for piece in pieces {
                words.push(piece.text.as_str());
                token_start_indices.push(token_idx);
                token_end_indices.push(token_idx + 1);
            }
            token_idx += 1;
        }
        Self {
            words,
            token_start_indices,
            token_end_indices,
            token_len: tokens.len(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.words.len()
    }

    pub(crate) fn get(&self, idx: usize) -> Option<&'a str> {
        self.words.get(idx).copied()
    }

    pub(crate) fn starts_with(&self, expected: &[&str]) -> bool {
        word_prefix_present(&self.words, expected)
    }

    pub(crate) fn starts_with_at(&self, idx: usize, expected: &[&str]) -> bool {
        word_prefix_present_at(&self.words, idx, expected)
    }

    pub(crate) fn starts_with_any(&self, expected: &[&[&str]]) -> bool {
        word_prefix_choice_present(&self.words, expected)
    }

    pub(crate) fn equals_at(&self, idx: usize, expected: &[&str]) -> bool {
        complete_word_sequence_at(&self.words, idx, expected)
    }

    pub(crate) fn equals_any_at(&self, idx: usize, expected: &[&[&str]]) -> bool {
        complete_word_sequence_choice_at(&self.words, idx, expected)
    }

    pub(crate) fn ends_with(&self, expected: &[&str]) -> bool {
        word_suffix_present(&self.words, expected)
    }

    pub(crate) fn ends_with_any(&self, expected: &[&[&str]]) -> bool {
        word_suffix_choice_present(&self.words, expected)
    }

    pub(crate) fn slice_eq(&self, start: usize, expected: &[&str]) -> bool {
        self.words
            .get(start..start.saturating_add(expected.len()))
            .is_some_and(|slice| {
                slice
                    .iter()
                    .copied()
                    .zip(expected.iter().copied())
                    .all(|(actual, expected)| actual == expected)
            })
    }

    pub(crate) fn find_phrase_start(&self, expected: &[&str]) -> Option<usize> {
        locate_word_sequence(&self.words, expected)
    }

    pub(crate) fn find_any_phrase_start<'p>(
        &self,
        expected: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], usize)> {
        parse_first_word_sequence_choice(&self.words, expected)
            .map(|parsed| (parsed.sequence, parsed.start))
    }

    pub(crate) fn find_phrase_value<T: Clone>(
        &self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, usize)> {
        word_slice_find_phrase_value(&self.words, expected)
    }

    pub(crate) fn find_window_by(
        &self,
        window_len: usize,
        predicate: impl FnMut(&[&str]) -> bool,
    ) -> Option<usize> {
        word_slice_find_window_by(&self.words, window_len, predicate)
    }

    pub(crate) fn contains_window_by(
        &self,
        window_len: usize,
        predicate: impl FnMut(&[&str]) -> bool,
    ) -> bool {
        word_slice_has_window_by(&self.words, window_len, predicate)
    }

    pub(crate) fn has_phrase(&self, expected: &[&str]) -> bool {
        word_sequence_present(&self.words, expected)
    }

    pub(crate) fn has_any_phrase(&self, expected: &[&[&str]]) -> bool {
        word_sequence_choice_present(&self.words, expected)
    }

    pub(crate) fn find_word(&self, expected: &str) -> Option<usize> {
        locate_word(&self.words, expected)
    }

    pub(crate) fn find_any_word(&self, expected: &[&str]) -> Option<usize> {
        locate_word_choice(&self.words, expected)
    }

    pub(crate) fn find_any_word_from(&self, expected: &[&str], start: usize) -> Option<usize> {
        word_slice_find_any_word_from(&self.words, expected, start)
    }

    pub(crate) fn rfind_word(&self, expected: &str) -> Option<usize> {
        locate_last_word_by(&self.words, |word| word == expected)
    }

    pub(crate) fn contains_word(&self, expected: &str) -> bool {
        word_present(&self.words, expected)
    }

    pub(crate) fn contains_any_word(&self, expected: &[&str]) -> bool {
        word_choice_present(&self.words, expected)
    }

    pub(crate) fn contains_no_words(&self, expected: &[&str]) -> bool {
        word_choice_absent(&self.words, expected)
    }

    pub(crate) fn contains_all_words(&self, expected: &[&str]) -> bool {
        every_word_present(&self.words, expected)
    }

    pub(crate) fn at_is(&self, idx: usize, expected: &str) -> bool {
        word_slice_at_is(&self.words, idx, expected)
    }

    pub(crate) fn at_is_any(&self, idx: usize, expected: &[&str]) -> bool {
        word_slice_at_is_any(&self.words, idx, expected)
    }

    pub(crate) fn first_is(&self, expected: &str) -> bool {
        word_slice_first_is(&self.words, expected)
    }

    pub(crate) fn first_is_any(&self, expected: &[&str]) -> bool {
        word_slice_first_is_any(&self.words, expected)
    }

    pub(crate) fn last_is(&self, expected: &str) -> bool {
        word_slice_last_is(&self.words, expected)
    }

    pub(crate) fn last_is_any(&self, expected: &[&str]) -> bool {
        word_slice_last_is_any(&self.words, expected)
    }

    pub(crate) fn matching_value<T: Clone>(&self, expected: &[(&[&str], T)]) -> Option<T> {
        word_slice_matching_value(&self.words, expected)
    }

    pub(crate) fn strip_prefix_value<'s, T: Clone>(
        &'s self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, &'s [&'a str])> {
        word_slice_strip_prefix_value(&self.words, expected)
    }

    pub(crate) fn strip_first_word_value<'s, T: Clone>(
        &'s self,
        expected: &[(&str, T)],
    ) -> Option<(T, &'s [&'a str])> {
        word_slice_strip_first_word_value(&self.words, expected)
    }

    pub(crate) fn strip_suffix_value<'s, T: Clone>(
        &'s self,
        expected: &[(&[&str], T)],
    ) -> Option<(T, &'s [&'a str])> {
        word_slice_strip_suffix_value(&self.words, expected)
    }

    pub(crate) fn first(&self) -> Option<&str> {
        self.get(0)
    }

    pub(crate) fn word_refs(&self) -> Vec<&'a str> {
        self.words.clone()
    }

    pub(crate) fn join(&self, separator: &str) -> String {
        self.words.join(separator)
    }

    pub(crate) fn owned_words(&self) -> Vec<String> {
        self.words.iter().map(|word| (*word).to_string()).collect()
    }

    pub(crate) fn to_word_refs(&self) -> Vec<&'a str> {
        self.word_refs()
    }

    pub(crate) fn token_boundary_for_word(&self, word_idx: usize) -> Option<usize> {
        self.token_start_indices.get(word_idx).copied()
    }

    pub(crate) fn token_boundary_for_word_or_end(&self, word_idx: usize) -> Option<usize> {
        if word_idx == self.len() {
            Some(self.token_len)
        } else {
            self.token_boundary_for_word(word_idx)
        }
    }

    pub(crate) fn token_start_indices(&self) -> &[usize] {
        &self.token_start_indices
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

    pub(crate) fn token_index_after_words_or_end(&self, word_count: usize) -> Option<usize> {
        if word_count == 0 {
            return Some(0);
        }
        if word_count > self.len() {
            return None;
        }
        if word_count == self.len() {
            return Some(self.token_len);
        }
        self.token_index_after_words(word_count)
    }

    pub(crate) fn token_span_for_words(
        &self,
        start_word: usize,
        end_word: usize,
    ) -> Option<std::ops::Range<usize>> {
        if start_word > end_word || end_word > self.len() {
            return None;
        }
        let start = if start_word == end_word {
            self.token_index_after_words(start_word)?
        } else {
            self.token_boundary_for_word(start_word)?
        };
        let end = self.token_index_after_words(end_word)?;
        Some(start..end)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LexedClause<'a> {
    tokens: &'a [OwnedLexToken],
}

#[allow(dead_code)]
impl<'a> LexedClause<'a> {
    pub(crate) fn new(tokens: &'a [OwnedLexToken]) -> Self {
        Self { tokens }
    }

    pub(crate) fn tokens(self) -> &'a [OwnedLexToken] {
        self.tokens
    }

    pub(crate) fn len(self) -> usize {
        self.tokens.len()
    }

    pub(crate) fn is_empty(self) -> bool {
        self.tokens.is_empty()
    }

    pub(crate) fn token(self, idx: usize) -> Option<&'a OwnedLexToken> {
        self.tokens.get(idx)
    }

    pub(crate) fn before(self, idx: usize) -> Self {
        Self::new(&self.tokens[..idx.min(self.tokens.len())])
    }

    pub(crate) fn from(self, idx: usize) -> Self {
        Self::new(&self.tokens[idx.min(self.tokens.len())..])
    }

    pub(crate) fn between(self, start: usize, end: usize) -> Self {
        let start = start.min(self.tokens.len());
        let end = end.min(self.tokens.len()).max(start);
        Self::new(&self.tokens[start..end])
    }

    pub(crate) fn words(self) -> TokenWordView<'a> {
        TokenWordView::new(self.tokens)
    }

    pub(crate) fn word_refs(self) -> Vec<&'a str> {
        self.words().word_refs()
    }

    pub(crate) fn word_refs_where(self, mut predicate: impl FnMut(&str) -> bool) -> Vec<&'a str> {
        self.word_refs()
            .into_iter()
            .filter(|word| predicate(word))
            .collect()
    }

    pub(crate) fn word_len(self) -> usize {
        self.words().len()
    }

    pub(crate) fn text(self) -> String {
        self.words().join(" ")
    }

    pub(crate) fn span(self) -> Option<TextSpan> {
        let first = self.tokens.first()?;
        let last = self.tokens.last()?;
        Some(TextSpan {
            line: first.span.line,
            start: first.span.start,
            end: last.span.end,
        })
    }

    pub(crate) fn first_is_word(self, expected: &str) -> bool {
        self.word_refs()
            .first()
            .is_some_and(|word| *word == expected)
    }

    pub(crate) fn first_is_any_word(self, expected: &[&str]) -> bool {
        parse_word_choice_at(&self.word_refs(), 0, expected).is_some()
    }

    pub(crate) fn first_word(self) -> Option<&'a str> {
        self.tokens
            .iter()
            .flat_map(|token| token.parser_word_pieces().iter())
            .map(|piece| piece.text.as_str())
            .next()
    }

    pub(crate) fn matches_words(self, expected: &[&str]) -> bool {
        parse_complete_word_sequence(&self.word_refs(), expected).is_some()
    }

    pub(crate) fn matches_any_words(self, phrases: &[&[&str]]) -> bool {
        parse_complete_word_sequence_choice(&self.word_refs(), phrases).is_some()
    }

    pub(crate) fn matching_word_value<T: Clone>(self, phrases: &[(&[&str], T)]) -> Option<T> {
        let words = self.word_refs();
        word_slice_matching_value(words.as_slice(), phrases)
    }

    pub(crate) fn find_word_value<T: Clone>(self, phrases: &[(&[&str], T)]) -> Option<(T, usize)> {
        let words = self.word_refs();
        word_slice_find_phrase_value(words.as_slice(), phrases)
    }

    pub(crate) fn starts_with(self, expected: &[&str]) -> bool {
        parse_word_sequence_at(&self.word_refs(), 0, expected).is_some()
    }

    pub(crate) fn starts_with_any(self, phrases: &[&[&str]]) -> bool {
        let words = self.word_refs();
        for sequence in phrases {
            if parse_word_sequence_at(&words, 0, sequence).is_some() {
                return true;
            }
        }
        false
    }

    pub(crate) fn ends_with(self, expected: &[&str]) -> bool {
        let words = self.word_refs();
        word_suffix_present(&words, expected)
    }

    pub(crate) fn ends_with_any(self, phrases: &[&[&str]]) -> bool {
        let words = self.word_refs();
        word_suffix_choice_present(&words, phrases)
    }

    pub(crate) fn strip_prefix_clause(self, expected: &[&str]) -> Option<Self> {
        let words = self.word_refs();
        let matched = parse_word_sequence_at(&words, 0, expected)?;
        let token_idx = self.words().token_index_after_words(matched.end)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn strip_any_prefix_clause<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self)> {
        let words = self.word_refs();
        let mut returned_sequence = None;
        let mut consumed_words = 0usize;
        for sequence in phrases {
            let Some(matched) = parse_word_sequence_at(&words, 0, sequence) else {
                continue;
            };
            returned_sequence.get_or_insert(*sequence);
            consumed_words = consumed_words.max(matched.end);
        }
        let returned_sequence = returned_sequence?;
        let token_idx = self.words().token_index_after_words(consumed_words)?;
        Some((returned_sequence, self.from(token_idx)))
    }

    pub(crate) fn strip_prefix_value_clause<T: Clone>(
        self,
        phrases: &[(&[&str], T)],
    ) -> Option<(T, Self)> {
        let words = self.word_refs();
        word_slice_strip_prefix_value(&words, phrases).and_then(|(value, tail_words)| {
            let consumed = words.len().saturating_sub(tail_words.len());
            let token_idx = self.words().token_index_after_words(consumed)?;
            Some((value, self.from(token_idx)))
        })
    }

    pub(crate) fn strip_suffix_clause(self, expected: &[&str]) -> Option<Self> {
        let words = self.words();
        let word_count = words.len().checked_sub(expected.len())?;
        parse_word_sequence_at(&words.word_refs(), word_count, expected)?;
        let token_idx = words.token_index_after_words(word_count)?;
        Some(self.before(token_idx))
    }

    pub(crate) fn strip_any_suffix_clause<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self)> {
        let words = self.word_refs();
        for sequence in phrases {
            let Some(start) = words.len().checked_sub(sequence.len()) else {
                continue;
            };
            if parse_word_sequence_at(&words, start, sequence).is_none() {
                continue;
            }
            let token_idx = self.words().token_index_after_words(start)?;
            return Some((sequence, self.before(token_idx)));
        }
        None
    }

    pub(crate) fn strip_suffix_value_clause<T: Clone>(
        self,
        phrases: &[(&[&str], T)],
    ) -> Option<(T, Self)> {
        let words = self.word_refs();
        word_slice_strip_suffix_value(&words, phrases).and_then(|(value, head_words)| {
            let token_idx = self.words().token_index_after_words(head_words.len())?;
            Some((value, self.before(token_idx)))
        })
    }

    pub(crate) fn token_boundary_for_word(self, word_idx: usize) -> Option<usize> {
        self.words().token_boundary_for_word(word_idx)
    }

    pub(crate) fn token_boundary_for_word_or_end(self, word_idx: usize) -> Option<usize> {
        self.words().token_boundary_for_word_or_end(word_idx)
    }

    pub(crate) fn token_index_after_words(self, word_count: usize) -> Option<usize> {
        self.words().token_index_after_words(word_count)
    }

    pub(crate) fn before_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_boundary_for_word(word_idx)?;
        Some(self.before(token_idx))
    }

    pub(crate) fn from_word(self, word_idx: usize) -> Option<Self> {
        let token_idx = self.token_boundary_for_word(word_idx)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn after_words(self, word_count: usize) -> Option<Self> {
        let token_idx = self.token_index_after_words(word_count)?;
        Some(self.from(token_idx))
    }

    pub(crate) fn between_words_trimmed(self, start: usize, end: usize) -> Self {
        self.between_word_range(start, end)
            .unwrap_or_else(|| self.between(self.tokens.len(), self.tokens.len()))
            .trimmed()
    }

    pub(crate) fn between_word_range(self, start: usize, end: usize) -> Option<Self> {
        let range = self.words().token_span_for_words(start, end)?;
        Some(self.between(range.start, range.end))
    }

    pub(crate) fn without_word_range_trimmed(
        self,
        word_start: usize,
        word_len: usize,
    ) -> Vec<OwnedLexToken> {
        let token_start = self
            .token_boundary_for_word(word_start)
            .unwrap_or(self.tokens.len());
        let token_end = self
            .token_boundary_for_word(word_start + word_len)
            .unwrap_or(self.tokens.len());
        let mut tokens = self.tokens[..token_start].to_vec();
        tokens.extend_from_slice(&self.tokens[token_end..]);
        LexedClause::new(&tokens).trim()
    }

    pub(crate) fn without_phrase_trimmed(self, phrase: &[&str]) -> Option<Vec<OwnedLexToken>> {
        let words = self.word_refs();
        let matched = parse_first_word_sequence(&words, phrase)?;
        Some(self.without_word_range_trimmed(matched.start, phrase.len()))
    }

    pub(crate) fn without_any_phrase_trimmed<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Vec<OwnedLexToken>)> {
        let words = self.word_refs();
        let matched = parse_first_word_sequence_choice(&words, phrases)?;
        Some((
            matched.sequence,
            self.without_word_range_trimmed(matched.start, matched.sequence.len()),
        ))
    }

    pub(crate) fn find_word(self, expected: &str) -> Option<usize> {
        parse_first_word_choice(&self.word_refs(), &[expected]).map(|(idx, _)| idx)
    }

    pub(crate) fn find_word_any(self, expected: &[&str]) -> Option<usize> {
        parse_first_word_choice(&self.word_refs(), expected).map(|(idx, _)| idx)
    }

    pub(crate) fn rfind_word(self, expected: &str) -> Option<usize> {
        self.words().rfind_word(expected)
    }

    pub(crate) fn find_phrase_start(self, expected: &[&str]) -> Option<usize> {
        parse_first_word_sequence(&self.word_refs(), expected).map(|matched| matched.start)
    }

    pub(crate) fn find_any_phrase_start<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(&'p [&'p str], usize)> {
        let words = self.word_refs();
        parse_first_word_sequence_choice(&words, phrases)
            .map(|matched| (matched.sequence, matched.start))
    }

    pub(crate) fn find_any_phrase_span<'p>(
        self,
        phrases: &'p [&'p [&'p str]],
    ) -> Option<(usize, usize)> {
        let words = self.word_refs();
        locate_word_sequence_choice_span(&words, phrases)
    }

    pub(crate) fn contains_word(self, expected: &str) -> bool {
        parse_first_word_choice(&self.word_refs(), &[expected]).is_some()
    }

    pub(crate) fn contains_any_word(self, expected: &[&str]) -> bool {
        parse_first_word_choice(&self.word_refs(), expected).is_some()
    }

    pub(crate) fn contains_no_words(self, expected: &[&str]) -> bool {
        parse_first_word_choice(&self.word_refs(), expected).is_none()
    }

    pub(crate) fn count_word(self, expected: &str) -> usize {
        self.word_refs()
            .into_iter()
            .filter(|word| *word == expected)
            .count()
    }

    pub(crate) fn contains_comma(self) -> bool {
        contains_token_kind(self.tokens, TokenKind::Comma)
    }

    pub(crate) fn contains_comma_or_any_word(self, expected: &[&str]) -> bool {
        self.contains_comma() || parse_first_word_choice(&self.word_refs(), expected).is_some()
    }

    pub(crate) fn contains_all_words(self, expected: &[&str]) -> bool {
        every_word_present(&self.word_refs(), expected)
    }

    pub(crate) fn find_token_word(self, expected: &str) -> Option<usize> {
        locate_token_word(self.tokens, expected)
    }

    pub(crate) fn find_token_word_any(self, expected: &[&str]) -> Option<usize> {
        locate_token_word_choice(self.tokens, expected)
    }

    pub(crate) fn find_token_word_where(
        self,
        expected: &str,
        mut predicate: impl FnMut(usize, Self) -> bool,
    ) -> Option<usize> {
        self.tokens.iter().enumerate().find_map(|(idx, token)| {
            if token.is_word(expected) && predicate(idx, self.from(idx + 1)) {
                Some(idx)
            } else {
                None
            }
        })
    }

    pub(crate) fn find_unquoted_token_word(self, expected: &str) -> Option<usize> {
        let mut inside_quotes = false;
        for (idx, token) in self.tokens.iter().enumerate() {
            if token.is_quote() {
                inside_quotes = !inside_quotes;
                continue;
            }
            if !inside_quotes && token.is_word(expected) {
                return Some(idx);
            }
        }
        None
    }

    pub(crate) fn rfind_token_word(self, expected: &str) -> Option<usize> {
        locate_last_token_word(self.tokens, expected)
    }

    pub(crate) fn split_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        let idx = locate_token_word(self.tokens, expected)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn split_once_on_word_trimmed(self, expected: &str) -> Option<(Self, Self)> {
        self.split_once_on_word(expected)
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn split_once_on_word_any(self, expected: &[&str]) -> Option<(Self, Self)> {
        let idx = self.find_token_word_any(expected)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn split_once_on_word_any_trimmed(self, expected: &[&str]) -> Option<(Self, Self)> {
        self.split_once_on_word_any(expected)
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn rsplit_once_on_word(self, expected: &str) -> Option<(Self, Self)> {
        let idx = locate_last_token_word(self.tokens, expected)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn rsplit_once_on_word_trimmed(self, expected: &str) -> Option<(Self, Self)> {
        self.rsplit_once_on_word(expected)
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn split_once_on_comma(self) -> Option<(Self, Self)> {
        let idx = locate_token_kind(self.tokens, TokenKind::Comma)?;
        Some((self.before(idx), self.from(idx + 1)))
    }

    pub(crate) fn split_once_before_word(self, expected: &str) -> Option<(Self, Self)> {
        let idx = locate_token_word(self.tokens, expected)?;
        Some((self.before(idx), self.from(idx)))
    }

    pub(crate) fn split_once_before_phrase(self, expected: &[&str]) -> Option<(Self, Self)> {
        let words = self.word_refs();
        let word_idx = parse_first_word_sequence(&words, expected)?.start;
        let token_idx = self.token_boundary_for_word(word_idx)?;
        Some((self.before(token_idx), self.from(token_idx)))
    }

    pub(crate) fn split_once_on_phrase(self, expected: &[&str]) -> Option<(Self, Self)> {
        let words = self.word_refs();
        let word_idx = parse_first_word_sequence(&words, expected)?.start;
        let start_token_idx = self.token_boundary_for_word(word_idx)?;
        let end_token_idx = self.token_index_after_words(word_idx + expected.len())?;
        Some((self.before(start_token_idx), self.from(end_token_idx)))
    }

    pub(crate) fn split_once_before_any_phrase<'p>(
        self,
        phrases: &[&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self, Self)> {
        let words = self.word_refs();
        let matched = parse_first_word_sequence_choice(&words, phrases)?;
        let token_idx = self.token_boundary_for_word(matched.start)?;
        Some((
            matched.sequence,
            self.before(token_idx),
            self.from(token_idx),
        ))
    }

    pub(crate) fn split_once_on_any_phrase<'p>(
        self,
        phrases: &[&'p [&'p str]],
    ) -> Option<(&'p [&'p str], Self, Self)> {
        let words = self.word_refs();
        let matched = parse_first_word_sequence_choice(&words, phrases)?;
        let start_token_idx = self.token_boundary_for_word(matched.start)?;
        let end_token_idx = self.token_index_after_words(matched.end)?;
        Some((
            matched.sequence,
            self.before(start_token_idx),
            self.from(end_token_idx),
        ))
    }

    pub(crate) fn take_until_token_matching<F>(self, mut predicate: F) -> Self
    where
        F: FnMut(&OwnedLexToken) -> bool,
    {
        let idx = parse_token_boundary_by(self.tokens, |token| predicate(token))
            .unwrap_or(self.tokens.len());
        self.before(idx)
    }

    pub(crate) fn trimmed(self) -> Self {
        Self::new(trim_lexed_commas(self.tokens))
    }

    pub(crate) fn trim(self) -> Vec<OwnedLexToken> {
        self.trimmed().tokens().to_vec()
    }

    pub(crate) fn trimmed_tokens(self) -> &'a [OwnedLexToken] {
        self.trimmed().tokens()
    }

    pub(crate) fn trimmed_word_refs(self) -> Vec<&'a str> {
        self.trimmed().word_refs()
    }

    pub(crate) fn comma_segments(self) -> Vec<Self> {
        grammar::split_lexed_slices_on_comma(self.tokens)
            .into_iter()
            .map(Self::new)
            .collect()
    }

    pub(crate) fn trimmed_comma_segments(self) -> Vec<Self> {
        self.comma_segments()
            .into_iter()
            .map(Self::trimmed)
            .collect()
    }

    pub(crate) fn and_segments(self) -> Vec<Self> {
        grammar::split_lexed_slices_on_and(self.tokens)
            .into_iter()
            .map(Self::new)
            .collect()
    }

    pub(crate) fn trimmed_and_segments(self) -> Vec<Self> {
        self.and_segments().into_iter().map(Self::trimmed).collect()
    }

    pub(crate) fn trimmed_and_comma_segments(self) -> Vec<Self> {
        self.and_segments()
            .into_iter()
            .flat_map(Self::trimmed_comma_segments)
            .filter(|segment| !segment.is_empty())
            .collect()
    }

    pub(crate) fn period_segments(self) -> Vec<Self> {
        grammar::split_lexed_slices_on_period(self.tokens)
            .into_iter()
            .map(Self::new)
            .collect()
    }

    pub(crate) fn trimmed_period_segments(self) -> Vec<Self> {
        self.period_segments()
            .into_iter()
            .map(Self::trimmed)
            .collect()
    }

    pub(crate) fn split_comma_then(self) -> Option<(Self, Self)> {
        grammar::split_lexed_once_on_separator(self.tokens, || {
            use winnow::Parser as _;
            (grammar::comma(), grammar::kw("then")).void()
        })
        .map(|(head, tail)| (Self::new(head), Self::new(tail)))
    }

    pub(crate) fn split_comma_then_trimmed(self) -> Option<(Self, Self)> {
        self.split_comma_then()
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn split_once_on_then(self) -> Option<(Self, Self)> {
        self.split_comma_then()
            .or_else(|| self.split_once_on_word("then"))
    }

    pub(crate) fn split_once_on_then_trimmed(self) -> Option<(Self, Self)> {
        self.split_once_on_then()
            .map(|(head, tail)| (head.trimmed(), tail.trimmed()))
    }

    pub(crate) fn comma_then_idx(self) -> Option<usize> {
        self.split_comma_then().map(|(head, _)| head.len())
    }

    pub(crate) fn without_leading_connectors_clause(self) -> Self {
        let trimmed = self.trimmed();
        let mut start = 0usize;
        while start < trimmed.tokens.len()
            && trimmed.tokens[start]
                .as_word()
                .is_some_and(|word| matches!(word, "then" | "and"))
        {
            start += 1;
        }
        trimmed.from(start)
    }

    pub(crate) fn without_trailing_words_clause(self, words: &[&str]) -> Self {
        let trimmed = self.trimmed();
        let mut end = trimmed.tokens.len();
        while end > 0
            && trimmed.tokens[end - 1]
                .as_word()
                .is_some_and(|word| crate::word_primitives::contains_word(words, word))
        {
            end -= 1;
        }
        trimmed.before(end)
    }

    /// If this clause's trailing tokens exactly spell `phrase` (word for word),
    /// return the clause with that phrase removed; otherwise return it unchanged.
    /// Unlike `without_trailing_words_clause`, this requires the exact ordered
    /// phrase and does not trim, so callers can detect a match by token count.
    pub(crate) fn without_trailing_phrase(self, phrase: &[&str]) -> Self {
        let len = self.tokens.len();
        if phrase.is_empty() || len < phrase.len() {
            return self;
        }
        let tail = &self.tokens[len - phrase.len()..];
        let matches = tail
            .iter()
            .zip(phrase)
            .all(|(token, expected)| token.as_word().is_some_and(|word| word == *expected));
        if matches {
            self.before(len - phrase.len())
        } else {
            self
        }
    }
}
