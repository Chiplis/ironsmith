use super::*;

pub fn parse_keyword_mechanic_tokens(tokens: &[OwnedLexToken]) -> Option<KeywordMechanicShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_keyword_mechanic_lexed,
        "keyword mechanic clause",
    )
}
