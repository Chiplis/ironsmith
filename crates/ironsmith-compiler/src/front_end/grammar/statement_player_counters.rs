use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerCounterSubject {
    EachOpponent,
    EachPlayer,
    TargetOpponent,
    TargetPlayer,
    ThatPlayer,
    You,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerCounterKind {
    Poison,
    Energy,
    Experience,
    Ticket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerGetsCountersShape {
    pub(crate) subject: PlayerCounterSubject,
    pub(crate) count: u32,
    pub(crate) kind: PlayerCounterKind,
}

pub(crate) fn parse_player_gets_counters_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayerGetsCountersShape> {
    primitives::find_prefix(tokens, || player_gets_counters_clause)
        .map(|(_, shape, _)| shape)
        .or_else(|| {
            primitives::find_prefix(tokens, || conjoined_player_gets_counters_clause)
                .map(|(_, shape, _)| shape)
        })
}

fn player_counter_subject(
    input: &mut LexStream<'_>,
) -> winnow::error::ModalResult<PlayerCounterSubject> {
    alt((
        primitives::phrase(&["each", "opponent"]).value(PlayerCounterSubject::EachOpponent),
        primitives::phrase(&["each", "player"]).value(PlayerCounterSubject::EachPlayer),
        primitives::phrase(&["target", "opponent"]).value(PlayerCounterSubject::TargetOpponent),
        primitives::phrase(&["target", "player"]).value(PlayerCounterSubject::TargetPlayer),
        primitives::phrase(&["that", "player"]).value(PlayerCounterSubject::ThatPlayer),
        primitives::kw("you").value(PlayerCounterSubject::You),
    ))
    .parse_next(input)
}

fn player_gets_counters_clause(
    input: &mut LexStream<'_>,
) -> winnow::error::ModalResult<PlayerGetsCountersShape> {
    let subject = player_counter_subject.parse_next(input)?;
    alt((primitives::kw("get"), primitives::kw("gets"))).parse_next(input)?;
    let (count, kind) = player_counter_tail.parse_next(input)?;
    player_counter_clause_end.parse_next(input)?;
    Ok(PlayerGetsCountersShape {
        subject,
        count,
        kind,
    })
}

fn conjoined_player_gets_counters_clause(
    input: &mut LexStream<'_>,
) -> winnow::error::ModalResult<PlayerGetsCountersShape> {
    let subject = alt((
        primitives::phrase(&["each", "opponent"]).value(PlayerCounterSubject::EachOpponent),
        primitives::phrase(&["each", "player"]).value(PlayerCounterSubject::EachPlayer),
    ))
    .parse_next(input)?;
    let _: &[OwnedLexToken] = repeat_till(
        1..,
        any.void(),
        peek((
            primitives::kw("and"),
            alt((primitives::kw("get"), primitives::kw("gets"))),
            player_counter_tail,
            player_counter_clause_end,
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("get"), primitives::kw("gets"))).parse_next(input)?;
    let (count, kind) = player_counter_tail.parse_next(input)?;
    player_counter_clause_end.parse_next(input)?;
    Ok(PlayerGetsCountersShape {
        subject,
        count,
        kind,
    })
}

fn player_counter_tail(
    input: &mut LexStream<'_>,
) -> winnow::error::ModalResult<(u32, PlayerCounterKind)> {
    let count = opt(alt((
        primitives::kw("another").value(1),
        leaf::parse_leaf_number_prefix_lexed,
    )))
    .parse_next(input)?
    .unwrap_or(1);
    let kind = alt((
        primitives::kw("poison").value(PlayerCounterKind::Poison),
        primitives::kw("energy").value(PlayerCounterKind::Energy),
        primitives::kw("experience").value(PlayerCounterKind::Experience),
        primitives::kw("ticket").value(PlayerCounterKind::Ticket),
    ))
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    Ok((count, kind))
}

fn player_counter_clause_end(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    alt((primitives::period().void(), eof.void())).parse_next(input)
}
