use super::*;

pub(super) fn less_life(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(
        trim(tokens),
        primitives::phrase(&["who", "has", "less", "life", "than", "you"]),
    )
    .map(|(_, rest)| trim(rest))
}
