use super::*;

pub(super) fn source_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("this"),
        primitives::kw("thiss"),
        primitives::kw("source"),
        primitives::kw("artifact"),
        primitives::kw("creature"),
        primitives::kw("permanent"),
    ))
    .void()
    .parse_next(input)
}
