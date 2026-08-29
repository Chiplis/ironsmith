use super::*;
use crate::grammar::primitives;

fn half_life_shape<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<HalfLifeShape> {
    primitives::word_slice_exact("half").parse_next(input)?;
    let owner = alt((
        primitives::word_slice_exact("your").value(HalfLifeOwnerShape::You),
        alt((
            primitives::word_slice_exact("their"),
            primitives::word_slice_exact("his"),
            primitives::word_slice_exact("her"),
        ))
        .value(HalfLifeOwnerShape::Contextual),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("life").parse_next(input)?;
    let rounded_down = opt(alt((
        (
            primitives::word_slice_exact("rounded"),
            primitives::word_slice_exact("down"),
        )
            .value(true),
        (
            primitives::word_slice_exact("rounded"),
            primitives::word_slice_exact("up"),
        )
            .value(false),
    )))
    .parse_next(input)?
    .unwrap_or(false);
    eof.parse_next(input)?;
    Ok(HalfLifeShape {
        rounded_down,
        owner,
    })
}

pub fn parse_half_life(words: &[&str]) -> Option<HalfLifeShape> {
    primitives::parse_full_word_slice(words, half_life_shape)
}
