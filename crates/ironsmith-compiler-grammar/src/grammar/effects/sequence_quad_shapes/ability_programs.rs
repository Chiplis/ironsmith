use super::*;

pub(super) fn bargained<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "this", "spell", "was", "bargained"])
        .void()
        .parse_next(input)
}
