use winnow::combinator::{alt, eof, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, literal, take_till};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::primitives;
use super::ActivationCostHeadSurface;

pub(super) fn when_one_or_more_followup_head(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        (
            alt((primitives::kw("when"), primitives::kw("whenever"))),
            primitives::phrase(&["one", "or", "more"]),
        )
            .void(),
        (
            alt((primitives::kw("when"), primitives::kw("whenever"))),
            primitives::kw("you"),
            alt((
                primitives::kw("discard"),
                primitives::kw("exile"),
                primitives::kw("mill"),
                primitives::kw("sacrifice"),
            )),
            primitives::phrase(&["one", "or", "more"]),
        )
            .void(),
    ))
    .parse_next(input)
}

pub(super) fn additional_activation_cost_head(
    input: &mut LexStream<'_>,
) -> WResult<ActivationCostHeadSurface> {
    let token: &OwnedLexToken = any.parse_next(input)?;
    if token.kind == TokenKind::ManaGroup {
        return Ok(ActivationCostHeadSurface::ManaGroup);
    }
    if matches!(token.kind, TokenKind::Plus | TokenKind::Dash) {
        return Ok(ActivationCostHeadSurface::Signed);
    }
    match token.parser_text() {
        "untap" => Ok(ActivationCostHeadSurface::Untap),
        "unattach" => Ok(ActivationCostHeadSurface::Unattach),
        signed if token.kind == TokenKind::Word && parse_signed_word(signed) => {
            Ok(ActivationCostHeadSurface::Signed)
        }
        _ => Err(primitives::backtrack_err(
            "activation-cost head",
            "signed loyalty or activation-cost verb",
        )),
    }
}

pub(super) fn alias_face_separator(input: &mut &str) -> WResult<()> {
    let _: &str = take_till(0.., |character: char| character == '/').parse_next(input)?;
    literal('/').void().parse_next(input)
}

fn parse_signed_word(raw: &str) -> bool {
    let mut input = raw;
    let parsed: WResult<()> = (
        alt((literal('+'), literal('-'))),
        repeat::<_, _, (), _, _>(1.., any.void()),
        eof,
    )
        .void()
        .parse_next(&mut input);
    parsed.is_ok()
}

pub(super) fn source_alias_effect_verb(input: &mut LexStream<'_>) -> WResult<()> {
    let word = primitives::word_parser_text.parse_next(input)?;
    matches!(
        word,
        "add"
            | "attach"
            | "become"
            | "convert"
            | "counter"
            | "create"
            | "deal"
            | "destroy"
            | "detain"
            | "discard"
            | "draw"
            | "exchange"
            | "exile"
            | "get"
            | "goad"
            | "incubate"
            | "investigate"
            | "look"
            | "mill"
            | "move"
            | "pay"
            | "proliferate"
            | "regenerate"
            | "remove"
            | "return"
            | "sacrifice"
            | "scry"
            | "search"
            | "shuffle"
            | "skip"
            | "surveil"
            | "suspect"
            | "tap"
            | "transform"
            | "untap"
    )
    .then_some(())
    .ok_or_else(|| primitives::backtrack_err("source alias", "effect verb"))
}
