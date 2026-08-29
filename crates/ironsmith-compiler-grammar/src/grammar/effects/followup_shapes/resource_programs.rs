use super::*;

pub(super) fn lifecycle_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("exile"), primitives::kw("sacrifice")))
        .void()
        .parse_next(input)
}
