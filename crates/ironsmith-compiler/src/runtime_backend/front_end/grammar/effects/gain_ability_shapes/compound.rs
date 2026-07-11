use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SharedAbilityVerb {
    Gain,
    Lose,
    Has,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GainThenGetShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) ability_tokens: &'a [OwnedLexToken],
    pub(crate) pump_tokens: &'a [OwnedLexToken],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GetThenAbilityShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) pump_tokens: &'a [OwnedLexToken],
    pub(crate) ability_tokens: &'a [OwnedLexToken],
    pub(crate) ability_verb: SharedAbilityVerb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttachedReferenceSubject {
    EnchantedCreature,
    EquippedCreature,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AttachedAndRelatedGetAbilityShape<'a> {
    pub(crate) subject: AttachedReferenceSubject,
    pub(crate) pump_tokens: &'a [OwnedLexToken],
    pub(crate) ability_tokens: &'a [OwnedLexToken],
    pub(crate) duration: Until,
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
    let semantic_start = word_view.token_boundary_for_word_or_end(duration.consumed_words)?;
    nonempty_trimmed(tokens.get(semantic_start..)?)
}

pub(crate) fn parse_gain_then_get_shape(tokens: &[OwnedLexToken]) -> Option<GainThenGetShape<'_>> {
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

pub(crate) fn parse_get_then_ability_shape(
    tokens: &[OwnedLexToken],
) -> Option<GetThenAbilityShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (get_token, (), after_get) = primitives::find_prefix(tokens, || get_verb)?;
    let subject_tokens = semantic_subject_tokens(tokens.get(..get_token)?)?;
    let (separator_token, ability_verb, ability_tokens) =
        primitives::find_prefix(after_get, || {
            (primitives::kw("and"), shared_ability_verb).map(|(_, verb)| verb)
        })?;
    let pump_tokens = nonempty_trimmed(after_get.get(..separator_token)?)?;
    let ability_tokens = nonempty_trimmed(ability_tokens)?;
    Some(GetThenAbilityShape {
        subject_tokens,
        pump_tokens,
        ability_tokens,
        ability_verb,
    })
}

pub(crate) fn parse_attached_and_related_get_ability_shape(
    tokens: &[OwnedLexToken],
) -> Option<AttachedAndRelatedGetAbilityShape<'_>> {
    let shape = parse_get_then_ability_shape(tokens)?;
    if !matches!(
        shape.ability_verb,
        SharedAbilityVerb::Gain | SharedAbilityVerb::Has
    ) {
        return None;
    }
    let subject = primitives::parse_all(
        shape.subject_tokens,
        parse_attached_and_related_subject,
        "attached object and related creatures subject",
    )
    .ok()?;
    let (ability_tokens, ()) =
        primitives::split_lexed_once_before_suffix(shape.ability_tokens, 1, || {
            (
                primitives::phrase(&["until", "end", "of", "turn"]),
                primitives::sentence_end(),
            )
                .void()
        })?;
    let ability_tokens = nonempty_trimmed(ability_tokens)?;
    Some(AttachedAndRelatedGetAbilityShape {
        subject,
        pump_tokens: shape.pump_tokens,
        ability_tokens,
        duration: Until::EndOfTurn,
    })
}

fn parse_attached_and_related_subject(
    input: &mut LexStream<'_>,
) -> WResult<AttachedReferenceSubject> {
    let subject = alt((
        primitives::phrase(&["enchanted", "creature"])
            .value(AttachedReferenceSubject::EnchantedCreature),
        primitives::phrase(&["equipped", "creature"])
            .value(AttachedReferenceSubject::EquippedCreature),
    ))
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(alt((primitives::kw("each"), primitives::kw("all")))).parse_next(input)?;
    primitives::phrase(&["other", "creatures"]).parse_next(input)?;
    opt(primitives::kw("that")).parse_next(input)?;
    alt((primitives::kw("share"), primitives::kw("shares"))).parse_next(input)?;
    primitives::phrase(&["a", "creature", "type", "with", "it"]).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(subject)
}

#[cfg(test)]
#[path = "compound/tests.rs"]
mod tests;
