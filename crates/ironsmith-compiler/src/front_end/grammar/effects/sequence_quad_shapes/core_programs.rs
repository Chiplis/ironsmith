use super::*;

pub(super) fn where_x_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(primitives::comma()).parse_next(&mut *input)?;
    primitives::phrase(&["where", "x", "is"])
        .void()
        .parse_next(input)
}
