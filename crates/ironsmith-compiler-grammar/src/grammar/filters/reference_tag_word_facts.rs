use winnow::combinator::{peek, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WordSpan {
    pub(super) start: usize,
    pub(super) end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PhraseFact<'p> {
    pub(super) phrase: &'p [&'p str],
    pub(super) span: WordSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WordFact<'i> {
    pub(super) word: &'i str,
    pub(super) index: usize,
}

fn expected_word<'i, 'p>(
    expected: &'p str,
) -> impl Parser<primitives::WordSliceInput<'i>, &'i str, ErrMode<ContextError>> + 'p {
    move |input: &mut primitives::WordSliceInput<'i>| {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err("word fact", "expected word"));
        };
        if *word != expected {
            return Err(primitives::backtrack_err("word fact", "expected word"));
        }
        *input = rest;
        Ok(*word)
    }
}

fn expected_phrase<'i, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<primitives::WordSliceInput<'i>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut primitives::WordSliceInput<'i>| {
        if expected.is_empty() {
            return Err(primitives::backtrack_err("phrase fact", "nonempty phrase"));
        }
        for word in expected {
            expected_word(word).void().parse_next(input)?;
        }
        Ok(())
    }
}

fn expected_phrase_choice<'i, 'p>(
    expected: &'p [&'p [&'p str]],
) -> impl Parser<primitives::WordSliceInput<'i>, &'p [&'p str], ErrMode<ContextError>> + 'p {
    move |input: &mut primitives::WordSliceInput<'i>| {
        for phrase in expected {
            let checkpoint = *input;
            if !phrase.is_empty() && expected_phrase(phrase).parse_next(input).is_ok() {
                return Ok(*phrase);
            }
            *input = checkpoint;
        }
        Err(primitives::backtrack_err(
            "phrase fact",
            "one expected phrase",
        ))
    }
}

fn expected_word_choice<'i, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<primitives::WordSliceInput<'i>, &'i str, ErrMode<ContextError>> + 'p {
    move |input: &mut primitives::WordSliceInput<'i>| {
        for candidate in expected {
            let checkpoint = *input;
            if let Ok(word) = expected_word(candidate).parse_next(input) {
                return Ok(word);
            }
            *input = checkpoint;
        }
        Err(primitives::backtrack_err("word fact", "one expected word"))
    }
}

pub(super) fn parse_phrase_at_head<'p>(
    words: &[&str],
    expected: &'p [&'p str],
) -> Option<PhraseFact<'p>> {
    let mut input: primitives::WordSliceInput<'_> = words;
    expected_phrase(expected).parse_next(&mut input).ok()?;
    Some(PhraseFact {
        phrase: expected,
        span: WordSpan {
            start: 0,
            end: words.len().checked_sub(input.len())?,
        },
    })
}

pub(super) fn parse_phrase_choice_at_head<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<PhraseFact<'p>> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let phrase = expected_phrase_choice(expected)
        .parse_next(&mut input)
        .ok()?;
    Some(PhraseFact {
        phrase,
        span: WordSpan {
            start: 0,
            end: words.len().checked_sub(input.len())?,
        },
    })
}

pub(super) fn parse_phrase_anywhere<'p>(
    words: &[&str],
    expected: &'p [&'p str],
) -> Option<PhraseFact<'p>> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped = repeat_till(0.., any.void(), peek(expected_phrase(expected)).void())
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    expected_phrase(expected).parse_next(&mut input).ok()?;
    Some(PhraseFact {
        phrase: expected,
        span: WordSpan {
            start: skipped.len(),
            end: words.len().checked_sub(input.len())?,
        },
    })
}

pub(super) fn parse_phrase_choice_anywhere<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<PhraseFact<'p>> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped = repeat_till(
        0..,
        any.void(),
        peek(expected_phrase_choice(expected)).void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    let phrase = expected_phrase_choice(expected)
        .parse_next(&mut input)
        .ok()?;
    Some(PhraseFact {
        phrase,
        span: WordSpan {
            start: skipped.len(),
            end: words.len().checked_sub(input.len())?,
        },
    })
}

pub(super) fn parse_phrase_whole<'p>(
    words: &[&str],
    expected: &'p [&'p str],
) -> Option<PhraseFact<'p>> {
    let fact = parse_phrase_at_head(words, expected)?;
    (fact.span.end == words.len()).then_some(fact)
}

pub(super) fn parse_phrase_choice_whole<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<PhraseFact<'p>> {
    let fact = parse_phrase_choice_at_head(words, expected)?;
    (fact.span.end == words.len()).then_some(fact)
}

pub(super) fn parse_word_choice<'i>(word: &'i str, expected: &[&str]) -> Option<WordFact<'i>> {
    let words = [word];
    let mut input: primitives::WordSliceInput<'_> = &words;
    expected_word_choice(expected)
        .void()
        .parse_next(&mut input)
        .ok()?;
    input.is_empty().then_some(WordFact { word, index: 0 })
}

pub(super) fn parse_word_choice_anywhere<'i>(
    words: &'i [&'i str],
    expected: &[&str],
) -> Option<WordFact<'i>> {
    let mut input: primitives::WordSliceInput<'i> = words;
    let skipped = repeat_till(0.., any.void(), peek(expected_word_choice(expected)).void())
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    let word = expected_word_choice(expected).parse_next(&mut input).ok()?;
    Some(WordFact {
        word,
        index: skipped.len(),
    })
}

pub(super) fn parse_last_word_choice_before<'i>(
    words: &'i [&'i str],
    expected: &[&str],
    before: usize,
) -> Option<WordFact<'i>> {
    let mut base = 0usize;
    let mut remaining = &words[..before.min(words.len())];
    let mut last = None;
    while let Some(fact) = parse_word_choice_anywhere(remaining, expected) {
        let index = base + fact.index;
        last = Some(WordFact {
            word: fact.word,
            index,
        });
        let consumed = fact.index + 1;
        base += consumed;
        remaining = &remaining[consumed..];
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_facts_preserve_matched_choice_and_span() {
        let words = ["card", "with", "mana", "value", "three"];
        let choices: &[&[&str]] = &[&["power"], &["mana", "value"]];
        let fact = parse_phrase_choice_anywhere(&words, choices).unwrap();

        assert_eq!(fact.phrase, ["mana", "value"]);
        assert_eq!(fact.span, WordSpan { start: 2, end: 4 });
    }

    #[test]
    fn word_facts_find_first_and_last_occurrences() {
        let words = ["with", "power", "with", "toughness"];
        assert_eq!(
            parse_word_choice_anywhere(&words, &["with"]).unwrap().index,
            0
        );
        assert_eq!(
            parse_last_word_choice_before(&words, &["with"], words.len())
                .unwrap()
                .index,
            2
        );
    }
}
