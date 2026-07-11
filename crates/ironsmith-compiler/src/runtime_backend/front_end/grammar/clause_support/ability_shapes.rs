use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::primitives::{self, WordSliceInput};
use crate::runtime_backend::front_end::lexer::OwnedLexToken;

pub(crate) fn parse_protection_from_colored_spells_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        primitives::phrase(&[
            "protection",
            "from",
            "spells",
            "that",
            "are",
            "one",
            "or",
            "more",
            "colors",
        ]),
        "protection-from-colored-spells",
    )
    .is_ok()
}

pub(crate) fn parse_casualty_planeswalker_copy_prefix_words(words: &[&str]) -> bool {
    parse_prefix_words(
        words,
        &[
            "casualty",
            "x",
            "the",
            "copy",
            "isnt",
            "legendary",
            "and",
            "has",
            "starting",
            "loyalty",
            "x",
        ],
    )
}

pub(crate) fn parse_read_ahead_prefix_words(words: &[&str]) -> bool {
    parse_prefix_words(words, &["read", "ahead"])
}

fn parse_prefix_words(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    parse_word_phrase(&mut input, expected).is_ok()
}

fn parse_word_phrase(
    input: &mut WordSliceInput<'_>,
    expected: &'static [&'static str],
) -> WResult<()> {
    for word in expected {
        primitives::word_slice_exact(word)
            .void()
            .parse_next(input)?;
    }
    Ok(())
}
