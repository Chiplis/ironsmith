use super::*;

pub(super) fn dies_this_way_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["dealt", "damage", "this", "way", "dies", "this", "turn"]),
        primitives::phrase(&[
            "dealt", "damage", "this", "way", "would", "die", "this", "turn",
        ]),
    ))
    .void()
    .parse_next(input)
}
