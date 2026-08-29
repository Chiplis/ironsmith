use super::*;

pub(super) fn pronoun_trigger_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["when", "it"]),
        primitives::phrase(&["whenever", "it"]),
        primitives::phrase(&["when", "they"]),
        primitives::phrase(&["whenever", "they"]),
    ))
    .parse_next(input)
}
