use crate::lexer::{OwnedLexToken, parser_token_word_refs};

use super::super::super::leaf;
use super::common;

const TOKEN_SACRIFICE_PREFIXES: &[&[&str]] = &[
    &["sacrifice", "it"],
    &["sacrifice", "them"],
    &["sacrifice", "that", "token"],
    &["sacrifice", "those", "tokens"],
];
const TOKEN_EXILE_PREFIXES: &[&[&str]] = &[&["exile", "it"], &["exile", "them"]];
const DELAYED_END_STEP_SACRIFICE_PREFIXES: &[&[&str]] = &[
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "end",
        "step",
        "sacrifice",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
        "sacrifice",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "next",
        "end",
        "step",
        "sacrifice",
    ],
];
const DELAYED_END_STEP_EXILE_PREFIXES: &[&[&str]] = &[
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "end",
        "step",
        "exile",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
        "exile",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "next",
        "end",
        "step",
        "exile",
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCopyModifierKind {
    GainHasteUntilEndOfTurn,
    HasHaste,
    EnterTappedAndAttacking,
    EnterTappedAndAttackingThatPlayer,
    SacrificeAtNextEndStep,
    SacrificeAtNextUpkeep,
    ExileAtNextEndStep,
}

fn non_article_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
        .into_iter()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect()
}

fn has_terminal_phrase(words: &[&str], phrase: &[&str]) -> bool {
    common::word_offset(words, phrase)
        .is_some_and(|start| start.saturating_add(phrase.len()) == words.len())
}

#[cfg(test)]
#[path = "token_copy_inline_tests.rs"]
mod tests;

#[path = "token_copy/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::parse_token_copy_modifier_kind;
