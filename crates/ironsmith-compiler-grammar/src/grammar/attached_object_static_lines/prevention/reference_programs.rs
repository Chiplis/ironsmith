use super::*;

pub(super) fn parse_this_source<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    semantic_kw("this").parse_next(input)?;
    opt(alt((semantic_kw("creature"), semantic_kw("permanent"))))
        .void()
        .parse_next(input)
}

pub(super) fn validate_source_reference(tokens: &[OwnedLexToken]) -> WResult<()> {
    let words = parser_token_word_refs(tokens);
    if leaf::parse_leaf_this_source_reference_words(&words).is_some()
        || crate::util::source_reference_surface_for_words(&words).is_some()
    {
        Ok(())
    } else {
        Err(primitives::backtrack_err(
            "damage-prevention subject",
            "source reference",
        ))
    }
}
