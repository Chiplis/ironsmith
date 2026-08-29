use winnow::combinator::alt;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;
use crate::effect::{Until, Value};
use crate::grammar::{leaf, primitives};
use crate::lexer::{OwnedLexToken, render_token_slice};
use crate::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone)]
pub struct BasePowerClauseShape<'a> {
    pub power: Value,
    pub target_tokens: &'a [OwnedLexToken],
    pub duration: Until,
}

#[derive(Debug, Clone)]
pub struct BasePowerToughnessClauseShape<'a> {
    pub power: Value,
    pub toughness: Value,
    pub target_tokens: &'a [OwnedLexToken],
    pub duration: Until,
    pub where_x_tokens: Option<&'a [OwnedLexToken]>,
}

fn has_or_have<'a>(input: &mut crate::lexer::LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("has"), primitives::kw("have")))
        .void()
        .parse_next(input)
}

fn duration_from_leaf(duration: leaf::LeafDurationPhrase) -> Until {
    match duration {
        leaf::LeafDurationPhrase::ThisTurn | leaf::LeafDurationPhrase::UntilEndOfTurn => {
            Until::EndOfTurn
        }
        leaf::LeafDurationPhrase::UntilEndOfCombat => Until::EndOfCombat,
        leaf::LeafDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
        leaf::LeafDurationPhrase::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
        leaf::LeafDurationPhrase::UntilYourNextUpkeep => Until::YourNextUpkeep,
        leaf::LeafDurationPhrase::ControllersNextUntapStep => Until::ControllersNextUntapStep,
        leaf::LeafDurationPhrase::Forever => Until::Forever,
    }
}

fn duration_prefix(tokens: &[OwnedLexToken]) -> Option<(Until, &[OwnedLexToken])> {
    let parsed = leaf::parse_leaf_restriction_duration_prefix_tokens(tokens)?;
    Some((
        duration_from_leaf(parsed.duration),
        trim_edge_punctuation_tokens(parsed.rest),
    ))
}

fn complete_duration(tokens: &[OwnedLexToken]) -> Option<Until> {
    let (duration, rest) = duration_prefix(tokens)?;
    rest.is_empty().then_some(duration)
}

fn contains_temporal_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["this", "turn"]),
            primitives::phrase(&["next", "turn"]),
            primitives::phrase(&["until", "end", "of", "turn"]),
            primitives::phrase(&["until", "the", "end", "of", "turn"]),
        ))
        .void()
    })
    .is_some()
}

fn permits_unqualified_duration(subject: &[OwnedLexToken], all: &[OwnedLexToken]) -> bool {
    primitives::contains_word(subject, "target")
        || duration_prefix(subject).is_some()
        || contains_temporal_marker(all)
}

fn target_and_leading_duration(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<Until>) {
    duration_prefix(tokens)
        .map(|(duration, rest)| (rest, Some(duration)))
        .unwrap_or_else(|| (trim_edge_punctuation_tokens(tokens), None))
}

fn has_shared_gain_tail(tokens: &[OwnedLexToken]) -> bool {
    let tokens = duration_prefix(tokens)
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    primitives::parse_prefix(
        trim_edge_punctuation_tokens(tokens),
        (
            primitives::kw("and"),
            alt((
                primitives::kw("gain"),
                primitives::kw("gains"),
                primitives::kw("lose"),
                primitives::kw("loses"),
                primitives::kw("has"),
                primitives::kw("have"),
                primitives::kw("get"),
                primitives::kw("gets"),
            )),
        )
            .void(),
    )
    .is_some()
}

fn split_subject_and_rest(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let (has_index, _, rest) = primitives::find_prefix(tokens, || has_or_have)?;
    let subject = trim_edge_punctuation_tokens(tokens.get(..has_index)?);
    (!subject.is_empty()).then_some((subject, trim_edge_punctuation_tokens(rest)))
}

#[cfg(test)]
#[path = "power_inline_tests.rs"]
mod tests;

#[path = "power/core_programs.rs"]
mod core_programs;
pub use core_programs::{parse_base_power_clause_shape, parse_base_power_toughness_clause_shape};
