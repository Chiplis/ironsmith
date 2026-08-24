use super::*;

pub fn parse_chosen_object_set_filter_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, filter_tokens) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["the", "chosen"]),
            primitives::kw("chosen").void(),
        )),
    )?;
    (!filter_tokens.is_empty()).then_some(filter_tokens)
}
