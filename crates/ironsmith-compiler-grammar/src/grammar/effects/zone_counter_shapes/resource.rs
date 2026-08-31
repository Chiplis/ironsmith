use super::*;

pub(super) fn parse_half_starting_life<'a>(
    input: &mut LexStream<'a>,
) -> WResult<HalfStartingLifeShape> {
    primitives::kw("half").parse_next(input)?;
    let player = half_starting_life_player.parse_next(input)?;
    primitives::phrase(&["starting", "life", "total"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let rounding = opt((
        primitives::kw("rounded"),
        alt((
            primitives::kw("up").value(HalfStartingLifeRounding::Up),
            primitives::kw("down").value(HalfStartingLifeRounding::Down),
        )),
    ))
    .map(|rounding| {
        rounding
            .map(|(_, value)| value)
            .unwrap_or(HalfStartingLifeRounding::Up)
    })
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(HalfStartingLifeShape { player, rounding })
}

pub fn parse_half_starting_life_shape(tokens: &[OwnedLexToken]) -> Option<HalfStartingLifeShape> {
    crate::grammar::primitives::probe_all(
        trim_shape_edges(tokens),
        parse_half_starting_life,
        "half starting life total",
    )
}
