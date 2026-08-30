use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Until;
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedAbilityVerb {
    Gain,
    Lose,
    Has,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GainThenGetShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub ability_tokens: &'a [OwnedLexToken],
    pub pump_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GetThenAbilityShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub pump_tokens: &'a [OwnedLexToken],
    pub ability_tokens: &'a [OwnedLexToken],
    pub ability_verb: SharedAbilityVerb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachedReferenceSubject {
    EnchantedCreature,
    EquippedCreature,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttachedAndRelatedGetAbilityShape<'a> {
    pub subject: AttachedReferenceSubject,
    pub pump_tokens: &'a [OwnedLexToken],
    pub ability_tokens: &'a [OwnedLexToken],
    pub duration: Until,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttachedAndRelatedGetShape<'a> {
    pub subject: AttachedReferenceSubject,
    pub pump_tokens: &'a [OwnedLexToken],
    pub duration: Until,
}

fn gain_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("gain"), primitives::kw("gains")))
        .void()
        .parse_next(input)
}

fn get_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("get"), primitives::kw("gets")))
        .void()
        .parse_next(input)
}

fn shared_ability_verb<'a>(input: &mut LexStream<'a>) -> WResult<SharedAbilityVerb> {
    alt((
        alt((primitives::kw("gain"), primitives::kw("gains"))).value(SharedAbilityVerb::Gain),
        alt((primitives::kw("lose"), primitives::kw("loses"))).value(SharedAbilityVerb::Lose),
        alt((primitives::kw("has"), primitives::kw("have"))).value(SharedAbilityVerb::Has),
    ))
    .parse_next(input)
}

fn nonempty_trimmed(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = trim_lexed_commas(tokens);
    (!tokens.is_empty()).then_some(tokens)
}

fn semantic_subject_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = nonempty_trimmed(tokens)?;
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let Some(duration) = super::parse_leading_gain_duration_shape(&words) else {
        return Some(tokens);
    };
    let semantic_start = word_view.map_word_or_end_to_token_boundary(duration.consumed_words)?;
    nonempty_trimmed(tokens.get(semantic_start..)?)
}

/// A shared pump/ability subject cannot also contain a completed player
/// action. For example, in `you draw ... and the chosen creatures get ...
/// and gain ...`, only `the chosen creatures` is shared by `get` and `gain`;
/// the leading draw belongs to the surrounding coordinated effect chain.
fn independent_player_action_precedes_shared_subject(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).to_word_refs();
    let player_subject_words = if crate::word_primitives::first_is(&words, "you") {
        1
    } else if crate::word_primitives::parse_choice_sequence_prefix(
        &words,
        &[
            &["target", "that", "each", "chosen", "active", "defending"],
            &["player"],
        ],
    ) || crate::word_primitives::parse_sequence_prefix(&words, &["the", "player"])
        || crate::word_primitives::parse_choice_sequence_prefix(
            &words,
            &[&["its", "their"], &["controller", "owner"]],
        )
    {
        2
    } else {
        return false;
    };
    let Some(verb) = super::super::chain_splitting::find_chain_verb_words(&words) else {
        return false;
    };
    verb.word_index == player_subject_words
        && crate::word_primitives::contains_word(&words[verb.word_index + 1..], "and")
}

pub fn parse_gain_then_get_shape(tokens: &[OwnedLexToken]) -> Option<GainThenGetShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (gain_token, (), after_gain) = primitives::find_prefix(tokens, || gain_verb)?;
    let subject_tokens = nonempty_trimmed(tokens.get(..gain_token)?)?;
    let (separator_token, (), pump_tokens) =
        primitives::find_prefix(after_gain, || (primitives::kw("and"), get_verb).void())?;
    let ability_tokens = nonempty_trimmed(after_gain.get(..separator_token)?)?;
    let pump_tokens = nonempty_trimmed(pump_tokens)?;
    Some(GainThenGetShape {
        subject_tokens,
        ability_tokens,
        pump_tokens,
    })
}

#[cfg(test)]
#[path = "compound/tests.rs"]
mod tests;

#[path = "compound/reference.rs"]
mod reference_programs;
use reference_programs::parse_attached_and_related_subject;
#[path = "compound/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::{
    parse_attached_and_related_get_ability_shape, parse_attached_and_related_get_shape,
};
#[path = "compound/ability.rs"]
mod ability_programs;
pub use ability_programs::parse_get_then_ability_shape;
