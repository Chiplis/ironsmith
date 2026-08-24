use super::*;

pub fn parse_trailing_if_fallback_shape(
    tokens: &[OwnedLexToken],
) -> Option<TrailingIfFallbackShape<'_>> {
    let mut offset = 1usize;
    let mut found = None;
    while offset < tokens.len() {
        let Some((relative, _, _)) =
            primitives::find_prefix(tokens.get(offset..)?, || primitives::kw("if"))
        else {
            break;
        };
        let split = offset + relative;
        if let Some(predicate) =
            crate::grammar::structure::parse_trailing_if_predicate_lexed(tokens.get(split..)?)
        {
            found = Some(TrailingIfFallbackShape {
                head_tokens: trim_lexed_commas(tokens.get(..split)?),
                predicate,
            });
        }
        offset = split + 1;
    }
    found
}
