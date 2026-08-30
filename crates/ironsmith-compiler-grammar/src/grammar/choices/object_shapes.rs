use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::ChoiceCount;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};
use super::super::{leaf, primitives};
use super::{ChoiceObjectClauseSyntaxError, word_phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPlayerChoiceActor {
    TargetPlayer,
    TargetOpponent,
    Opponent,
    ThatPlayer,
    Voter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossessiveObjectChoiceActor {
    You,
    SubjectPlayer,
    ObjectController,
    Opponent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PossessiveObjectChoiceShape {
    pub actor: PossessiveObjectChoiceActor,
    pub object_tokens: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChoiceObjectFilterFacts {
    pub bare_card: bool,
    pub graveyard_and_hand: bool,
    pub tagged_graveyard_disjunction: bool,
    pub graveyard_arm_is_plain_card: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetPlayerChoiceShape<'a> {
    pub actor: TargetPlayerChoiceActor,
    pub count: ChoiceCount,
    pub filter_tokens: &'a [OwnedLexToken],
    /// The same clause beginning at its `choose`/`chooses` verb. This lets the
    /// shared object-choice grammar retain trailing dynamic counts and chosen-
    /// set constraints without duplicating those rules for player subjects.
    pub object_choice_tokens: &'a [OwnedLexToken],
    pub filter_facts: ChoiceObjectFilterFacts,
    pub filter_is_player_target: bool,
}

pub fn parse_target_player_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetPlayerChoiceShape<'_>>, ChoiceObjectClauseSyntaxError> {
    let mut input = LexStream::new(tokens);
    let actor = match parse_target_player_choice_head.parse_next(&mut input) {
        Ok(actor) => actor,
        Err(_) => return Ok(None),
    };
    let consumed = tokens.len().saturating_sub(input.len());
    let object_choice_tokens = tokens.get(consumed.saturating_sub(1)..).unwrap_or_default();
    let body = trim_punctuation_edges(tokens.get(consumed..).unwrap_or_default());
    if body.is_empty() {
        return Err(ChoiceObjectClauseSyntaxError::MissingObject);
    }

    let (count, filter_tokens) =
        if let Some(parsed) = leaf::parse_leaf_choice_count_prefix_tokens(body) {
            (
                parsed.count,
                trim_punctuation_edges(body.get(parsed.consumed..).unwrap_or_default()),
            )
        } else {
            (ChoiceCount::exactly(1), body)
        };
    if filter_tokens.is_empty() {
        return Err(ChoiceObjectClauseSyntaxError::MissingFilter);
    }

    let filter_words = TokenWordView::new(filter_tokens).word_refs();
    Ok(Some(TargetPlayerChoiceShape {
        actor,
        count,
        filter_tokens,
        object_choice_tokens,
        filter_facts: parse_choice_object_filter_facts_words(&filter_words),
        filter_is_player_target: parse_player_target_prefix_words(&filter_words),
    }))
}

/// Remove an embedded choice-owner phrase while retaining the complete object
/// and zone description around it, e.g. `a creature card of their choice from
/// their graveyard` -> `a creature card from their graveyard`.
pub fn parse_possessive_object_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PossessiveObjectChoiceShape> {
    for (phrase, actor) in [
        (
            &["of", "its", "controller's", "choice"][..],
            PossessiveObjectChoiceActor::ObjectController,
        ),
        (
            &["of", "its", "controllers", "choice"][..],
            PossessiveObjectChoiceActor::ObjectController,
        ),
        (
            &["of", "his", "or", "her", "choice"][..],
            PossessiveObjectChoiceActor::SubjectPlayer,
        ),
        (
            &["of", "their", "choice"][..],
            PossessiveObjectChoiceActor::SubjectPlayer,
        ),
        (
            &["of", "your", "choice"][..],
            PossessiveObjectChoiceActor::You,
        ),
        (
            &["of", "an", "opponent's", "choice"][..],
            PossessiveObjectChoiceActor::Opponent,
        ),
        (
            &["of", "an", "opponents", "choice"][..],
            PossessiveObjectChoiceActor::Opponent,
        ),
    ] {
        let Some((first, _, rest)) =
            primitives::find_prefix(tokens, || primitives::phrase(phrase).void())
        else {
            continue;
        };
        let mut object_tokens = Vec::with_capacity(first + rest.len());
        object_tokens.extend_from_slice(tokens.get(..first)?);
        object_tokens.extend_from_slice(rest);
        let object_tokens = trim_punctuation_edges(&object_tokens).to_vec();
        if !object_tokens.is_empty() {
            return Some(PossessiveObjectChoiceShape {
                actor,
                object_tokens,
            });
        }
    }
    None
}

pub fn parse_choice_object_filter_facts_words(words: &[&str]) -> ChoiceObjectFilterFacts {
    let has_graveyard = word_occurs(words, parse_graveyard_word);
    let has_hand = word_occurs(words, parse_hand_word);
    let has_or = word_occurs(words, primitives::word_slice_exact("or").void());
    ChoiceObjectFilterFacts {
        bare_card: primitives::parse_full_word_slice(words, parse_card_word).is_some(),
        graveyard_and_hand: has_graveyard && has_hand,
        tagged_graveyard_disjunction: has_graveyard && has_or,
        graveyard_arm_is_plain_card: phrase_occurs(words, &["or", "a", "card", "from"])
            || phrase_occurs(words, &["or", "the", "card", "from"])
            || phrase_occurs(words, &["or", "card", "from"]),
    }
}

fn parse_target_player_choice_head(input: &mut LexStream<'_>) -> WResult<TargetPlayerChoiceActor> {
    let actor = alt((
        (primitives::kw("target"), primitives::kw("player"))
            .value(TargetPlayerChoiceActor::TargetPlayer),
        (
            primitives::kw("target"),
            alt((primitives::kw("opponent"), primitives::kw("opponents"))),
        )
            .value(TargetPlayerChoiceActor::TargetOpponent),
        (opt(primitives::kw("an")), primitives::kw("opponent"))
            .value(TargetPlayerChoiceActor::Opponent),
        (
            primitives::kw("that"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value(TargetPlayerChoiceActor::ThatPlayer),
        (primitives::kw("the"), primitives::kw("voter")).value(TargetPlayerChoiceActor::Voter),
    ))
    .parse_next(input)?;
    alt((primitives::kw("choose"), primitives::kw("chooses"))).parse_next(input)?;
    Ok(actor)
}

#[cfg(test)]
#[path = "object_shapes_inline_tests.rs"]
mod tests;

#[path = "object_shapes/core.rs"]
mod core_programs;
use core_programs::{phrase_occurs, trim_punctuation_edges, word_occurs};
#[path = "object_shapes/zone.rs"]
mod zone_programs;
use zone_programs::{parse_graveyard_word, parse_hand_word};
#[path = "object_shapes/library.rs"]
mod library_programs;
use library_programs::parse_card_word;
#[path = "object_shapes/reference.rs"]
mod reference_programs;
use reference_programs::parse_player_target_prefix_words;
