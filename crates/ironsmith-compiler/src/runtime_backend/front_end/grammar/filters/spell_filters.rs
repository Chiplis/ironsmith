use super::*;
pub(crate) fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let mut filter = if let Some(filter) = parse_domain_union_object_filter_lexed(tokens, other) {
        filter
    } else if let Some(filter) = parse_extremum_object_filter_lexed(tokens, other)? {
        filter
    } else if let Some(filter) = parse_simple_object_filter_lexed(tokens, other) {
        filter
    } else {
        parse_object_filter(tokens, other)?
    };
    preserve_filter_counter_constraint_surface_tokens(&mut filter, tokens);
    Ok(filter)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> ObjectFilter {
    let words_view = GrammarFilterNormalizedWords::new(tokens);
    let words = non_article_word_refs(&words_view.to_word_refs());

    parse_spell_filter_from_words(&words)
}

pub(crate) fn parse_spell_filter_with_grammar_entrypoint(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let words = non_article_token_word_refs(tokens);

    parse_spell_filter_from_words(&words)
}
