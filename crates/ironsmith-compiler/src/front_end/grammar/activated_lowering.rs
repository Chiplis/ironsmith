use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, split_lexed_sentences,
};
use super::primitives;
use crate::ir::ActivatedPresentationKind;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedManaEffectKind {
    AddMana,
    ColorsAmong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedRestrictionSentenceKind {
    ManaSource,
    SpendThisManaOnly,
    WhenSpendThisManaToCast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedXDefinitionIntro {
    WhereXIs,
    XIs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivatedXDefinitionShape<'a> {
    pub intro: ActivatedXDefinitionIntro,
    pub value_tokens: &'a [OwnedLexToken],
    pub exiled_card_mana_value: bool,
}

fn surface_has_sequence(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn starts_with_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(phrase)).is_some()
}

fn parse_colors_among_mana_surface(tokens: &[OwnedLexToken]) -> bool {
    surface_has_sequence(tokens, &["for", "each", "color", "among"])
        && surface_has_sequence(tokens, &["add", "one", "mana", "of", "that", "color"])
}

pub fn parse_activated_mana_effect_kind(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedManaEffectKind> {
    if parse_colors_among_mana_surface(tokens) {
        return Some(ActivatedManaEffectKind::ColorsAmong);
    }
    alt((
        primitives::kw("add").void(),
        primitives::phrase(&["you", "add"]),
        primitives::phrase(&["that", "player", "add"]),
        primitives::phrase(&["target", "player", "add"]),
    ))
    .parse_peek(LexStream::new(tokens))
    .ok()
    .map(|_| ActivatedManaEffectKind::AddMana)
}

pub fn contains_where_x_definition(tokens: &[OwnedLexToken]) -> bool {
    surface_has_sequence(tokens, &["where", "x", "is"])
}

pub fn contains_add_x_mana(tokens: &[OwnedLexToken]) -> bool {
    surface_has_sequence(tokens, &["add", "x", "mana"])
}

pub fn any_player_may_activate_on_stack(tokens: &[OwnedLexToken]) -> bool {
    surface_has_sequence(
        tokens,
        &["any", "player", "may", "activate", "this", "ability"],
    ) && surface_has_sequence(tokens, &["on", "the", "stack"])
}

pub fn parse_activated_presentation_kind_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedPresentationKind> {
    let (delimiter, _, _) = primitives::find_prefix(tokens, || {
        alt((
            primitives::token_kind(TokenKind::Dash),
            primitives::token_kind(TokenKind::EmDash),
        ))
    })?;
    let label = tokens.get(..delimiter)?;
    let label_words = TokenWordView::new(label);
    let head = label_words.first()?;
    match head {
        "throw" => Some(
            if label
                .iter()
                .filter(|token| token.kind == TokenKind::Period)
                .count()
                >= 3
            {
                ActivatedPresentationKind::ThrowEllipsis
            } else {
                ActivatedPresentationKind::Throw
            },
        ),
        "boast" => Some(ActivatedPresentationKind::Boast),
        "exhaust" => Some(ActivatedPresentationKind::Exhaust),
        "renew" => Some(ActivatedPresentationKind::Renew),
        "channel" => Some(ActivatedPresentationKind::Channel),
        "cohort" => Some(ActivatedPresentationKind::Cohort),
        "teleport" => Some(ActivatedPresentationKind::Teleport),
        "transmute" => Some(ActivatedPresentationKind::Transmute),
        _ => None,
    }
}

pub fn parse_activated_functional_zones_tokens(
    cost_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Vec<Zone> {
    let effect_sentences = split_lexed_sentences(effect_tokens);
    super::functional_zones::parse_activated_functional_zones_tokens(cost_tokens, &effect_sentences)
}

fn parse_mana_source_restriction_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["spend", "only", "mana"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["to", "activate", "this", "ability"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["to", "activate", "this", "ability"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

pub fn classify_activated_restriction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedRestrictionSentenceKind> {
    if primitives::parse_all(
        tokens,
        parse_mana_source_restriction_lexed,
        "activation-mana-source-restriction",
    )
    .is_ok()
    {
        return Some(ActivatedRestrictionSentenceKind::ManaSource);
    }
    if starts_with_phrase(tokens, &["spend", "this", "mana", "only"]) {
        return Some(ActivatedRestrictionSentenceKind::SpendThisManaOnly);
    }
    starts_with_phrase(
        tokens,
        &["when", "you", "spend", "this", "mana", "to", "cast"],
    )
    .then_some(ActivatedRestrictionSentenceKind::WhenSpendThisManaToCast)
}

fn x_definition_intro<'a>(input: &mut LexStream<'a>) -> WResult<ActivatedXDefinitionIntro> {
    alt((
        primitives::phrase(&["where", "x", "is"]).value(ActivatedXDefinitionIntro::WhereXIs),
        primitives::phrase(&["x", "is"]).value(ActivatedXDefinitionIntro::XIs),
    ))
    .parse_next(input)
}

fn parse_activated_x_definition_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(ActivatedXDefinitionIntro, &'a [OwnedLexToken])> {
    let intro = x_definition_intro.parse_next(input)?;
    let value_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok((intro, value_tokens))
}

fn exiled_card_mana_value_tail(tokens: &[OwnedLexToken]) -> bool {
    let mut parser = alt((
        primitives::phrase(&["the", "mana", "value", "of", "that", "card"]),
        primitives::phrase(&["that", "card", "mana", "value"]),
        primitives::phrase(&["that", "cards", "mana", "value"]),
    ));
    let Ok((rest, ())) = parser.parse_peek(LexStream::new(tokens)) else {
        return false;
    };
    let consumed = tokens.len().saturating_sub(rest.len());
    primitives::parse_all(
        &tokens[consumed..],
        (opt(primitives::period()), eof).void(),
        "x-definition-tail",
    )
    .is_ok()
}

pub fn parse_activated_x_definition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedXDefinitionShape<'_>> {
    let (intro, value_tokens) = primitives::parse_all(
        tokens,
        parse_activated_x_definition_lexed,
        "activated-x-definition",
    )
    .ok()?;
    Some(ActivatedXDefinitionShape {
        intro,
        value_tokens,
        exiled_card_mana_value: exiled_card_mana_value_tail(value_tokens),
    })
}

pub fn find_activated_x_definition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedXDefinitionShape<'_>> {
    let (offset, _, _) =
        primitives::find_prefix(tokens, || primitives::phrase(&["where", "x", "is"]))?;
    parse_activated_x_definition_tokens(&tokens[offset..])
}

fn parse_level_number_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    primitives::kw("level").parse_next(input)?;
    let token = any.parse_next(input)?;
    let level = token
        .parser_text()
        .parse::<u32>()
        .map_err(|_| primitives::backtrack_err("level number", "decimal level"))?;
    Ok(level)
}

pub fn parse_level_number_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    primitives::parse_prefix(tokens, parse_level_number_lexed).map(|(level, _)| level)
}

#[cfg(test)]
#[path = "activated_lowering/tests.rs"]
mod tests;
