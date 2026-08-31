use super::*;

pub(super) fn each_player_or_opponent_exiles(
    input: &mut LexStream<'_>,
) -> WResult<EachPlayerExileGroup> {
    primitives::kw("each").parse_next(input)?;
    let group = alt((
        primitives::kw("player").value(EachPlayerExileGroup::Player),
        primitives::kw("players").value(EachPlayerExileGroup::Player),
        primitives::kw("opponent").value(EachPlayerExileGroup::Opponent),
        primitives::kw("opponents").value(EachPlayerExileGroup::Opponent),
    ))
    .parse_next(input)?;
    alt((primitives::kw("exile"), primitives::kw("exiles")))
        .void()
        .parse_next(input)?;
    Ok(group)
}

pub fn parse_each_player_exile_counted_hand_permanent_shape(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerExileCountedHandPermanentShape> {
    let mut input = LexStream::new(tokens);
    let group = crate::grammar::primitives::take_leaf(&mut input, each_player_or_opponent_exiles)?;
    crate::grammar::primitives::take_leaf(&mut input, counted_permanents_and_or_hand_cards)?;
    input
        .is_empty()
        .then_some(EachPlayerExileCountedHandPermanentShape { group })
}
