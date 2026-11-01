use winnow::prelude::*;

use crate::grammar::primitives;

fn normalized_word_eq(actual: &str, expected: &str) -> bool {
    actual
        .chars()
        .filter(|ch| *ch != '\'')
        .map(|ch| ch.to_ascii_lowercase())
        .eq(expected
            .chars()
            .filter(|ch| *ch != '\'')
            .map(|ch| ch.to_ascii_lowercase()))
}

fn dynamic_phrase<'a>(
    expected: &'a [&'a str],
) -> impl Parser<primitives::WordSliceInput<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>
+ 'a {
    move |input: &mut primitives::WordSliceInput<'a>| {
        let mut rest = *input;
        for expected_word in expected {
            let Some((actual, tail)) = rest.split_first() else {
                return Err(primitives::backtrack_err("divvy phrase", "word"));
            };
            if !normalized_word_eq(actual, expected_word) {
                return Err(primitives::backtrack_err("divvy phrase", "word"));
            }
            rest = tail;
        }
        *input = rest;
        Ok(())
    }
}

// Each matcher claims exactly the words it matched; a pile shape is read
// under a speculative scope (see `parse_divvy_sequence_shape`), so a shape
// that fails after a partial match reads nothing.
pub(super) fn exact(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    let matched = (dynamic_phrase(expected), primitives::word_slice_eof)
        .parse_next(&mut input)
        .is_ok();
    if matched {
        crate::word_claims::claim_words(words);
    }
    matched
}

pub(super) fn prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    let matched = dynamic_phrase(expected).parse_next(&mut input).is_ok();
    if matched {
        crate::word_claims::claim_words(&words[..expected.len()]);
    }
    matched
}

pub(super) fn phrase_anywhere(words: &[&str], expected: &[&str]) -> bool {
    let start = (0..=words.len().saturating_sub(expected.len())).find(|&start| {
        let mut input: primitives::WordSliceInput<'_> = &words[start..];
        dynamic_phrase(expected).parse_next(&mut input).is_ok()
    });
    match start {
        Some(start) if words.len() >= expected.len() => {
            crate::word_claims::claim_words(&words[start..start + expected.len()]);
            true
        }
        _ => false,
    }
}

pub(super) fn exact_sequence(sentence_words: &[Vec<&str>], expected: &[&[&str]]) -> bool {
    sentence_words.len() == expected.len()
        && sentence_words
            .iter()
            .zip(expected)
            .all(|(actual, expected)| exact(actual, expected))
}

/// Drop a trailing "(Piles can be empty.)" reminder when it survives
/// preprocessing: it restates CR 700.3 rather than adding an instruction, so
/// the reader claims it and reads the program without it.
pub(super) fn without_piles_reminder<'a, 'b>(sentence_words: &'a [Vec<&'b str>]) -> &'a [Vec<&'b str>] {
    match sentence_words.split_last() {
        Some((last, rest))
            if !rest.is_empty() && exact(last, &["piles", "can", "be", "empty"]) =>
        {
            rest
        }
        _ => sentence_words,
    }
}

/// A closing disposition sentence that may end with ", then shuffle" or be
/// followed by its own "(Then) shuffle." sentence. Returns whether the
/// program shuffles; `None` when the sentences are anything else.
pub(super) fn disposition_then_shuffle(rest: &[Vec<&str>], expected: &[&str]) -> Option<bool> {
    let is_shuffle = |words: &[&str]| {
        !words.is_empty()
            && normalized_word_eq(words[words.len() - 1], "shuffle")
            && (words.len() == 1 || (words.len() == 2 && normalized_word_eq(words[0], "then")))
    };
    match rest {
        [put] => {
            if put.len() == expected.len() + 2
                && is_shuffle(&put[expected.len()..])
                && exact(&put[..expected.len()], expected)
            {
                crate::word_claims::claim_words(&put[expected.len()..]);
                Some(true)
            } else {
                exact(put, expected).then_some(false)
            }
        }
        [put, shuffle] => (is_shuffle(shuffle.as_slice()) && exact(put, expected)).then(|| {
            crate::word_claims::claim_words(shuffle);
            true
        }),
        _ => None,
    }
}

/// The index of the first sentence after the lead whose words are exactly
/// `expected`; the sentence is claimed.
pub(super) fn find_exact_sentence(
    sentence_words: &[Vec<&str>],
    min_index: usize,
    expected: &[&str],
) -> Option<usize> {
    let index = (min_index..sentence_words.len()).find(|&index| {
        let words = &sentence_words[index];
        words.len() == expected.len()
            && words
                .iter()
                .zip(expected)
                .all(|(actual, expected)| normalized_word_eq(actual, expected))
    })?;
    exact(&sentence_words[index], expected).then_some(index)
}
