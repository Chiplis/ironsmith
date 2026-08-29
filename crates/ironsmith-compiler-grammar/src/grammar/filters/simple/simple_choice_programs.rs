use super::*;

pub(super) fn parse_chosen_player_location(
    input: &mut WordInput<'_>,
) -> WResult<(Option<PlayerFilter>, Zone)> {
    opt(primitives::word_slice_exact("the"))
        .void()
        .parse_next(input)?;
    primitives::word_slice_exact("chosen")
        .void()
        .parse_next(input)?;
    alt((
        primitives::word_slice_exact("player"),
        primitives::word_slice_exact("players"),
    ))
    .void()
    .parse_next(input)?;
    let zone = alt((
        primitives::word_slice_exact("graveyard").value(Zone::Graveyard),
        primitives::word_slice_exact("hand").value(Zone::Hand),
        primitives::word_slice_exact("library").value(Zone::Library),
    ))
    .parse_next(input)?;
    Ok((Some(PlayerFilter::ChosenPlayer), zone))
}
