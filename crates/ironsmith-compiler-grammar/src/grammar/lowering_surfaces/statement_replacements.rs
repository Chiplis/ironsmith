use winnow::combinator::{alt, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::model::facts::StatementReplacementSurfaceKind;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

pub fn parse_statement_replacement_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementReplacementSurfaceKind> {
    crate::grammar::primitives::probe_all(
        tokens,
        alt((
            parse_bargained_return_replacement,
            parse_kicked_count_override_replacement,
            parse_kicked_multi_zone_to_battlefield,
            parse_clash_win_top_replacement,
            parse_morbid_search_to_battlefield,
        )),
        "statement replacement surface",
    )
}

fn parse_bargained_return_replacement(
    input: &mut LexStream<'_>,
) -> WResult<StatementReplacementSurfaceKind> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["if", "this", "spell", "was", "bargained"]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&[
            "one", "of", "those", "cards", "with", "mana", "value", "4", "or", "less",
        ]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&[
            "onto",
            "the",
            "battlefield",
            "instead",
            "of",
            "putting",
            "it",
            "into",
            "your",
            "hand",
        ]),
    )
    .parse_next(input)?;
    consume_tail(input)?;
    Ok(StatementReplacementSurfaceKind::BargainedReturnToBattlefield)
}

fn parse_kicked_count_override_replacement(
    input: &mut LexStream<'_>,
) -> WResult<StatementReplacementSurfaceKind> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&[
            "put", "two", "of", "those", "cards", "into", "your", "hand", "instead",
        ]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["put", "one", "of", "those", "cards", "into", "your", "hand"]),
    )
    .parse_next(input)?;
    consume_tail(input)?;
    Ok(StatementReplacementSurfaceKind::KickedCountOverride)
}

fn parse_kicked_multi_zone_to_battlefield(
    input: &mut LexStream<'_>,
) -> WResult<StatementReplacementSurfaceKind> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["if", "this", "spell", "was", "kicked"]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&[
            "put",
            "those",
            "cards",
            "onto",
            "the",
            "battlefield",
            "instead",
            "of",
            "putting",
            "them",
            "into",
            "your",
            "hand",
        ]),
    )
    .parse_next(input)?;
    consume_tail(input)?;
    Ok(StatementReplacementSurfaceKind::KickedMultiZoneToBattlefield)
}

fn parse_clash_win_top_replacement(
    input: &mut LexStream<'_>,
) -> WResult<StatementReplacementSurfaceKind> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["clash", "with", "an", "opponent"]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::phrase(&["if", "you", "win"]))
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["on", "top", "of", "its"]),
    )
    .parse_next(input)?;
    alt((
        primitives::kw("owner's"),
        primitives::kw("owners"),
        primitives::kw("owner"),
    ))
    .parse_next(input)?;
    primitives::phrase(&["library", "instead"]).parse_next(input)?;
    consume_tail(input)?;
    Ok(StatementReplacementSurfaceKind::ClashWinTopOfLibrary)
}

fn parse_morbid_search_to_battlefield(
    input: &mut LexStream<'_>,
) -> WResult<StatementReplacementSurfaceKind> {
    alt((
        (
            seek_morbid_condition,
            seek_search_to_battlefield_replacement,
        )
            .void(),
        (
            seek_search_to_battlefield_replacement,
            seek_morbid_condition,
        )
            .void(),
    ))
    .parse_next(input)?;
    consume_tail(input)?;
    Ok(StatementReplacementSurfaceKind::MorbidSearchToBattlefield)
}

fn seek_morbid_condition(input: &mut LexStream<'_>) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["creature", "died", "this", "turn"]),
    )
    .void()
    .parse_next(input)
}

fn seek_search_to_battlefield_replacement(input: &mut LexStream<'_>) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&[
            "put",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "instead",
            "of",
            "putting",
            "it",
            "into",
            "your",
            "hand",
        ]),
    )
    .void()
    .parse_next(input)
}

fn consume_tail(input: &mut LexStream<'_>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)
}

#[cfg(test)]
#[path = "statement_replacements_tests.rs"]
mod tests;
