use super::*;

pub(super) fn untap_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["they", "dont", "untap", "during"]),
        primitives::phrase(&["they", "don't", "untap", "during"]),
        primitives::phrase(&["those", "permanents", "dont", "untap", "during"]),
        primitives::phrase(&["those", "permanents", "don't", "untap", "during"]),
    ))
    .void()
    .parse_next(input)
}

pub(super) fn remains_tapped<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["remains", "tapped"])
        .void()
        .parse_next(input)
}

pub fn parse_untap_clause_prefix_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(trimmed(tokens), untap_prefix).is_some()
}
