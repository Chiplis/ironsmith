use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::target::PlayerFilter;

use super::super::primitives::{self, WordSliceInput};

const END_STEP_PHRASES: &[&[&str]] = &[
    &["beginning", "of", "your", "next", "end", "step"],
    &["beginning", "of", "the", "next", "end", "step"],
    &["beginning", "of", "next", "end", "step"],
    &["beginning", "of", "the", "end", "step"],
    &["beginning", "of", "end", "step"],
];

const DELAY_REFERENCE_WORDS: &[&str] =
    &["token", "tokens", "permanent", "permanents", "it", "them"];

#[derive(Debug, Clone, PartialEq)]
pub struct NextEndStepDelayFacts {
    pub sacrifice_reference: bool,
    pub exile_reference: bool,
    pub player: PlayerFilter,
}

pub fn parse_next_end_step_delay_words(words: &[&str]) -> Option<NextEndStepDelayFacts> {
    let matched_end_step = END_STEP_PHRASES
        .iter()
        .copied()
        .any(|phrase| phrase_present(words, phrase));
    if !matched_end_step {
        return None;
    }

    let has_reference = DELAY_REFERENCE_WORDS
        .iter()
        .copied()
        .any(|word| word_present(words, word));
    Some(NextEndStepDelayFacts {
        sacrifice_reference: has_reference && word_present(words, "sacrifice"),
        exile_reference: has_reference && word_present(words, "exile"),
        player: if phrase_present(words, END_STEP_PHRASES[0]) {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        },
    })
}

fn phrase_present(words: &[&str], phrase: &[&str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    while !input.is_empty() {
        let mut probe = input;
        if parse_dynamic_phrase(&mut probe, phrase).is_ok() {
            return true;
        }
        if parse_any_word.parse_next(&mut input).is_err() {
            break;
        }
    }
    false
}

fn parse_dynamic_phrase(input: &mut WordSliceInput<'_>, phrase: &[&str]) -> WResult<()> {
    for expected in phrase {
        let word = parse_any_word.parse_next(input)?;
        if word != *expected {
            return Err(primitives::backtrack_err(
                "end-step phrase",
                "end-step phrase",
            ));
        }
    }
    Ok(())
}

fn parse_any_word<'a>(input: &mut WordSliceInput<'a>) -> WResult<&'a str> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err("word", "word"));
    };
    *input = rest;
    Ok(*word)
}

fn word_present(words: &[&str], expected: &str) -> bool {
    for word in words {
        if *word == expected {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_facts_preserve_action_reference_and_player() {
        assert_eq!(
            parse_next_end_step_delay_words(&[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "end",
                "step",
                "sacrifice",
                "it",
            ]),
            Some(NextEndStepDelayFacts {
                sacrifice_reference: true,
                exile_reference: false,
                player: PlayerFilter::You,
            })
        );
        assert!(parse_next_end_step_delay_words(&["sacrifice", "it"]).is_none());
    }
}
