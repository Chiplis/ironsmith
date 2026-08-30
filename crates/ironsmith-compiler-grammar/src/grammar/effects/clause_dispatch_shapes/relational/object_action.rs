use super::*;

pub fn parse_modifier_duration_for_each_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let after_modifier = tokens.get(1..)?;
    let (_, rest) = primitives::parse_prefix(
        after_modifier,
        primitives::phrase(&["until", "end", "of", "turn"]),
    )?;
    primitives::parse_prefix(rest, primitives::phrase(&["for", "each"]))?;
    Some(rest)
}
