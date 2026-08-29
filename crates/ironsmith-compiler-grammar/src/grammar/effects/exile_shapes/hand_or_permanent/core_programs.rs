use super::*;

pub(super) fn and_or(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("and/or").void(),
        primitives::phrase(&["and", "or"]),
    ))
    .parse_next(input)
}
