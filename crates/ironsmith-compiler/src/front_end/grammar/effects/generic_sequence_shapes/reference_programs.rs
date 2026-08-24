use super::*;

pub(super) fn source_tapped_duration<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as"])
        .void()
        .parse_next(input)
}

pub fn parse_source_tapped_lock_shape(tokens: &[OwnedLexToken]) -> bool {
    let clause = trimmed(tokens);
    primitives::parse_prefix(clause, untap_prefix).is_some()
        && primitives::find_prefix(clause, || source_tapped_duration).is_some()
        && primitives::find_prefix(clause, || remains_tapped).is_some()
        && primitives::find_prefix(clause, || source_marker).is_some()
}
