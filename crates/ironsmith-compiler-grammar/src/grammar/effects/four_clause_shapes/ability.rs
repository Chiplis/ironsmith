use super::*;

pub(super) fn bargained<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["if", "this", "spell", "was", crate::keyword_forms::BARGAINED])
        .void()
        .parse_next(input)
}
