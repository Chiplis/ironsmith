use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordFallbackKind {
    Aftermath,
    BasicLandcycling,
    Encore,
    JumpStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordPrefixShape {
    Surge,
    Freerunning,
    Sneak,
    Exploit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordSpecialFormShape {
    SpellSneak,
    PermanentSneak,
    BlitzFromGraveyard,
    ExertAttack,
}

pub(crate) fn parse_keyword_fallback_kind_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordFallbackKind> {
    primitives::parse_prefix(tokens, parse_keyword_fallback_kind_lexed).map(|(kind, _)| kind)
}

pub(crate) fn parse_keyword_prefix_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordPrefixShape> {
    primitives::parse_prefix(tokens, parse_keyword_prefix_shape_lexed).map(|(shape, _)| shape)
}

pub(crate) fn parse_keyword_special_form_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordSpecialFormShape> {
    let mut input = LexStream::new(tokens);
    if parse_blitz_from_graveyard_marker_lexed
        .parse_next(&mut input)
        .is_ok()
    {
        return Some(KeywordSpecialFormShape::BlitzFromGraveyard);
    }
    if primitives::parse_prefix(tokens, parse_exert_attack_prefix_lexed).is_some() {
        return Some(KeywordSpecialFormShape::ExertAttack);
    }
    let mut input = LexStream::new(tokens);
    if find_phrase_lexed(&mut input, &["enters", "tapped", "and", "attacking"]).is_ok() {
        return Some(KeywordSpecialFormShape::PermanentSneak);
    }
    let mut input = LexStream::new(tokens);
    if parse_spell_sneak_marker_lexed
        .parse_next(&mut input)
        .is_ok()
    {
        return Some(KeywordSpecialFormShape::SpellSneak);
    }
    None
}

fn parse_keyword_fallback_kind_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordFallbackKind> {
    alt((
        primitives::kw("aftermath").value(KeywordFallbackKind::Aftermath),
        primitives::phrase(&["basic", "landcycling"]).value(KeywordFallbackKind::BasicLandcycling),
        primitives::kw("encore").value(KeywordFallbackKind::Encore),
        alt((
            primitives::kw("jumpstart").void(),
            primitives::kw("jump-start").void(),
            primitives::phrase(&["jump", "start"]),
        ))
        .value(KeywordFallbackKind::JumpStart),
    ))
    .parse_next(input)
}

fn parse_keyword_prefix_shape_lexed<'a>(input: &mut LexStream<'a>) -> WResult<KeywordPrefixShape> {
    alt((
        primitives::kw("surge").value(KeywordPrefixShape::Surge),
        primitives::kw("freerunning").value(KeywordPrefixShape::Freerunning),
        primitives::kw("sneak").value(KeywordPrefixShape::Sneak),
        primitives::kw("exploit").value(KeywordPrefixShape::Exploit),
    ))
    .parse_next(input)
}

fn parse_blitz_from_graveyard_marker_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    find_phrase_lexed(input, &["from", "your", "graveyard"])?;
    find_phrase_lexed(input, &["using", "its", "blitz", "ability"])
}

fn parse_spell_sneak_marker_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    find_phrase_lexed(input, &["you", "may", "cast", "this", "spell", "for"])?;
    find_phrase_lexed(
        input,
        &[
            "return",
            "an",
            "unblocked",
            "attacker",
            "you",
            "control",
            "to",
            "hand",
            "during",
            "the",
            "declare",
            "blockers",
            "step",
        ],
    )
}

fn parse_exert_attack_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["you", "may", "exert"]),
        (
            primitives::phrase(&["if", "this", "creature"]),
            alt((primitives::kw("hasnt"), primitives::kw("hasn't"))),
            primitives::phrase(&["been", "exerted", "this", "turn"]),
            opt(primitives::comma()),
            primitives::phrase(&["you", "may", "exert"]),
        )
            .void(),
    ))
    .void()
    .parse_next(input)
}

fn find_phrase_lexed<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
) -> WResult<()> {
    loop {
        let mut candidate = input.clone();
        if primitives::phrase(phrase)
            .parse_next(&mut candidate)
            .is_ok()
        {
            *input = candidate;
            return Ok(());
        }
        any.void().parse_next(input)?;
    }
}
