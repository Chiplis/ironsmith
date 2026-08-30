use super::*;

pub(super) fn upkeep_pay_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["at", "the", "beginning", "of", "your", "next", "upkeep"]),
        primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "upkeep"]),
    ))
    .void()
    .parse_next(input)?;
    winnow::combinator::opt(primitives::comma())
        .void()
        .parse_next(input)?;
    primitives::kw("pay").void().parse_next(input)
}
