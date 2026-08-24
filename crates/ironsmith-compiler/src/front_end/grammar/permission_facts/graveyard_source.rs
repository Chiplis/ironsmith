//! Typed grammar facts for graveyard/source permission clauses.

use crate::types::CardType;
use crate::zone::Zone;

use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives};

const ONCE_EACH_TURN_GRAVEYARD_CAST_PREFIX: &[&str] = &[
    "once", "during", "each", "of", "your", "turns", "you", "may", "cast",
];
const GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX: &[&str] =
    &["in", "addition", "to", "paying", "its", "other", "costs"];
const GRAVEYARD_CAST_EXILE_AFTER_RESOLUTION_SUFFIX: &[&str] = &[
    "if",
    "a",
    "spell",
    "cast",
    "this",
    "way",
    "would",
    "be",
    "put",
    "into",
    "your",
    "graveyard",
    "exile",
    "it",
    "instead",
];
const DIE_ROLL_PERMISSION_TAIL: &[&str] = &[
    "this",
    "turn",
    "if",
    "you",
    "cast",
    "it",
    "this",
    "way",
    "and",
    "it",
    "would",
    "be",
    "put",
    "into",
    "your",
    "graveyard",
    "exile",
    "it",
    "instead",
];
const TOP_LIBRARY_SHARED_TYPE_PREFIX: &[&str] = &["once", "each", "turn", "you", "may", "cast"];
const SHARES_CARD_TYPE_WITH: &[&str] = &["if", "it", "shares", "a", "card", "type", "with"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKindFact {
    Card,
    Spell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnceEachTurnGraveyardCastFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub cost_tokens: Option<&'a [OwnedLexToken]>,
    pub exiles_after_resolution: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraveyardAdditionalCostFact<'a> {
    Sacrifice {
        filter_tokens: &'a [OwnedLexToken],
    },
    ExileCards {
        count: u32,
        card_types: Vec<CardType>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceGraveyardAdditionalCostFact<'a> {
    pub source_kind: SourceKindFact,
    pub cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCastPermissionFact {
    pub source_kind: SourceKindFact,
    pub zone: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceGraveyardDieRollCastFact {
    pub result: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceGraveyardDynamicSurchargeFact<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub cost_tokens: &'a [OwnedLexToken],
    pub repetition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnceEachTurnTopLibrarySharedTypeFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub source_reference_tokens: &'a [OwnedLexToken],
}

pub fn parse_once_each_turn_graveyard_cast_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OnceEachTurnGraveyardCastFact<'_>> {
    parse_semantic_all(tokens, parse_once_each_turn_graveyard_cast_lexed)
}

fn parse_once_each_turn_graveyard_cast_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<OnceEachTurnGraveyardCastFact<'a>> {
    semantic_phrase(ONCE_EACH_TURN_GRAVEYARD_CAST_PREFIX).parse_next(input)?;
    let subject_tokens = take_until_semantic_phrase(input, &["from"])?;
    semantic_kw("from").parse_next(input)?;
    semantic_phrase(&["your", "graveyard"]).parse_next(input)?;

    let cost_tokens = if opt(semantic_kw("by")).parse_next(input)?.is_some() {
        let cost_tokens = take_until_semantic_phrase(input, GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX)?;
        semantic_phrase(GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX).parse_next(input)?;
        Some(trim_lexed_commas(cost_tokens))
    } else {
        None
    };
    let exiles_after_resolution = opt(semantic_phrase(
        GRAVEYARD_CAST_EXILE_AFTER_RESOLUTION_SUFFIX,
    ))
    .parse_next(input)?
    .is_some();

    Ok(OnceEachTurnGraveyardCastFact {
        subject_tokens: trim_lexed_commas(subject_tokens),
        cost_tokens,
        exiles_after_resolution,
    })
}

pub fn parse_graveyard_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<GraveyardAdditionalCostFact<'_>> {
    parse_semantic_all(
        tokens,
        alt((
            parse_sacrificing_additional_cost_lexed,
            parse_exiling_graveyard_additional_cost_lexed,
        )),
    )
}

fn parse_sacrificing_additional_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<GraveyardAdditionalCostFact<'a>> {
    semantic_kw("sacrificing").parse_next(input)?;
    let filter_tokens = take_semantic_rest(input)?;
    Ok(GraveyardAdditionalCostFact::Sacrifice {
        filter_tokens: trim_lexed_commas(filter_tokens),
    })
}

fn parse_exiling_graveyard_additional_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<GraveyardAdditionalCostFact<'a>> {
    semantic_kw("exiling").parse_next(input)?;
    let count = semantic_number_token.parse_next(input)?;
    let (atoms, ()) = repeat_till::<_, _, Vec<Option<CardType>>, _, _, _, _>(
        1..,
        parse_card_type_atom,
        peek(alt((semantic_kw("card"), semantic_kw("cards")))),
    )
    .parse_next(input)?;
    alt((semantic_kw("card"), semantic_kw("cards"))).parse_next(input)?;
    semantic_phrase(&["from", "your", "graveyard"]).parse_next(input)?;

    let mut card_types = Vec::new();
    for card_type in atoms.into_iter().flatten() {
        if card_types.iter().all(|existing| *existing != card_type) {
            card_types.push(card_type);
        }
    }
    if card_types.is_empty() {
        return Err(primitives::backtrack_err(
            "graveyard exile additional cost",
            "at least one card type",
        ));
    }
    Ok(GraveyardAdditionalCostFact::ExileCards { count, card_types })
}

fn parse_card_type_atom(input: &mut LexStream<'_>) -> WResult<Option<CardType>> {
    alt((
        alt((semantic_kw("artifact"), semantic_kw("artifacts"))).value(Some(CardType::Artifact)),
        alt((semantic_kw("creature"), semantic_kw("creatures"))).value(Some(CardType::Creature)),
        alt((semantic_kw("enchantment"), semantic_kw("enchantments")))
            .value(Some(CardType::Enchantment)),
        alt((semantic_kw("instant"), semantic_kw("instants"))).value(Some(CardType::Instant)),
        alt((semantic_kw("land"), semantic_kw("lands"))).value(Some(CardType::Land)),
        alt((semantic_kw("planeswalker"), semantic_kw("planeswalkers")))
            .value(Some(CardType::Planeswalker)),
        alt((semantic_kw("sorcery"), semantic_kw("sorceries"))).value(Some(CardType::Sorcery)),
        alt((semantic_kw("and"), semantic_kw("or"), semantic_kw("and/or"))).value(None),
    ))
    .parse_next(input)
}

pub fn parse_source_graveyard_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceGraveyardAdditionalCostFact<'_>> {
    parse_semantic_all(tokens, parse_source_graveyard_additional_cost_lexed)
}

fn parse_source_graveyard_additional_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SourceGraveyardAdditionalCostFact<'a>> {
    semantic_kw("this").parse_next(input)?;
    let source_kind = parse_source_kind.parse_next(input)?;
    semantic_phrase(&["from", "your", "graveyard", "by"]).parse_next(input)?;
    let cost_tokens = take_until_semantic_phrase(input, GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX)?;
    semantic_phrase(GRAVEYARD_CAST_ADDITIONAL_COST_SUFFIX).parse_next(input)?;
    Ok(SourceGraveyardAdditionalCostFact {
        source_kind,
        cost_tokens: trim_lexed_commas(cost_tokens),
    })
}

pub fn parse_source_cast_permission_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceCastPermissionFact> {
    parse_semantic_all(tokens, parse_source_cast_permission_lexed)
}

fn parse_source_cast_permission_lexed(
    input: &mut LexStream<'_>,
) -> WResult<SourceCastPermissionFact> {
    semantic_kw("this").parse_next(input)?;
    let source_kind = parse_source_kind.parse_next(input)?;
    semantic_kw("from").parse_next(input)?;
    let zone = alt((
        semantic_phrase(&["your", "graveyard"]).value(Zone::Graveyard),
        semantic_kw("exile").value(Zone::Exile),
    ))
    .parse_next(input)?;
    Ok(SourceCastPermissionFact { source_kind, zone })
}

fn parse_source_kind(input: &mut LexStream<'_>) -> WResult<SourceKindFact> {
    alt((
        semantic_kw("card").value(SourceKindFact::Card),
        semantic_kw("spell").value(SourceKindFact::Spell),
    ))
    .parse_next(input)
}

pub fn parse_source_graveyard_die_roll_cast_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceGraveyardDieRollCastFact> {
    parse_semantic_all(tokens, parse_source_graveyard_die_roll_cast_lexed)
}

fn parse_source_graveyard_die_roll_cast_lexed(
    input: &mut LexStream<'_>,
) -> WResult<SourceGraveyardDieRollCastFact> {
    semantic_phrase(&[
        "this",
        "card",
        "from",
        "your",
        "graveyard",
        "as",
        "long",
        "as",
        "youve",
        "rolled",
        "a",
    ])
    .parse_next(input)?;
    let result = semantic_number_token.parse_next(input)?;
    semantic_phrase(DIE_ROLL_PERMISSION_TAIL).parse_next(input)?;
    Ok(SourceGraveyardDieRollCastFact { result })
}

pub fn parse_source_graveyard_dynamic_surcharge_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceGraveyardDynamicSurchargeFact<'_>> {
    parse_semantic_all(tokens, parse_source_graveyard_dynamic_surcharge_lexed)
}

fn parse_source_graveyard_dynamic_surcharge_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SourceGraveyardDynamicSurchargeFact<'a>> {
    semantic_phrase(&["you", "may", "cast"]).parse_next(input)?;
    let source_tokens = (
        semantic_kw("this"),
        alt((
            semantic_kw("card"),
            semantic_kw("spell"),
            semantic_kw("permanent"),
            semantic_kw("creature"),
            semantic_kw("artifact"),
            semantic_kw("enchantment"),
        )),
    )
        .void()
        .take()
        .parse_next(input)?;
    semantic_phrase(&["from", "your", "graveyard", "if", "you", "pay"]).parse_next(input)?;
    let cost_tokens = take_until_semantic_phrase(input, &["more", "to", "cast", "it"])?;
    semantic_phrase(&["more", "to", "cast", "it"]).parse_next(input)?;
    let repetition_tokens = (semantic_phrase(&["for", "each"]), take_semantic_rest)
        .void()
        .take()
        .parse_next(input)?;
    Ok(SourceGraveyardDynamicSurchargeFact {
        source_tokens: trim_lexed_commas(source_tokens),
        cost_tokens: trim_lexed_commas(cost_tokens),
        repetition_tokens: trim_lexed_commas(repetition_tokens),
    })
}

pub fn parse_once_each_turn_top_library_shared_type_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OnceEachTurnTopLibrarySharedTypeFact<'_>> {
    parse_semantic_all(tokens, parse_once_each_turn_top_library_shared_type_lexed)
}

fn parse_once_each_turn_top_library_shared_type_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<OnceEachTurnTopLibrarySharedTypeFact<'a>> {
    semantic_phrase(TOP_LIBRARY_SHARED_TYPE_PREFIX).parse_next(input)?;
    let subject_tokens = take_until_semantic_phrase(input, &["from"])?;
    if parse_semantic_all(
        subject_tokens,
        alt((semantic_phrase(&["a", "spell"]), semantic_kw("spells"))),
    )
    .is_none()
    {
        return Err(primitives::backtrack_err(
            "top-library shared-type subject",
            "a spell or spells",
        ));
    }
    semantic_phrase(&["from", "the", "top", "of", "your", "library"]).parse_next(input)?;
    semantic_phrase(SHARES_CARD_TYPE_WITH).parse_next(input)?;
    let source_reference_tokens = (
        semantic_phrase(&["a", "card", "exiled", "with", "this"]),
        semantic_single_word_token,
    )
        .void()
        .take()
        .parse_next(input)?;
    Ok(OnceEachTurnTopLibrarySharedTypeFact {
        subject_tokens: trim_lexed_commas(subject_tokens),
        source_reference_tokens: trim_lexed_commas(source_reference_tokens),
    })
}

fn parse_semantic_all<'a, O, P>(tokens: &'a [OwnedLexToken], mut parser: P) -> Option<O>
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    let output = parser.parse_next(&mut input).ok()?;
    semantic_finish.parse_next(&mut input).ok()?;
    Some(output)
}

fn take_until_semantic_phrase<'a>(
    input: &mut LexStream<'a>,
    phrase: &'static [&'static str],
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(semantic_phrase(phrase)))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn take_semantic_rest<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(semantic_finish))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn semantic_number_token(input: &mut LexStream<'_>) -> WResult<u32> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    leaf::parse_leaf_number_token_lexed.parse_next(input)
}

fn semantic_single_word_token(input: &mut LexStream<'_>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    any.verify(|token: &&OwnedLexToken| token.parser_word_pieces().len() == 1)
        .void()
        .parse_next(input)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., semantic_noise),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn semantic_noise(input: &mut LexStream<'_>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| token.parser_word_pieces().is_empty())
        .void()
        .parse_next(input)
}

fn semantic_finish(input: &mut LexStream<'_>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

#[cfg(test)]
#[path = "graveyard_source_inline_tests.rs"]
mod tests;
