use super::*;

pub fn parse_keyword_mechanic_tokens(tokens: &[OwnedLexToken]) -> Option<KeywordMechanicShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_keyword_mechanic_lexed,
        "keyword mechanic clause",
    )
    .ok()
}
