use super::*;
pub(crate) fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let temporal_graveyard_history = {
        let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
        words.windows(2).any(|window| window == ["put", "there"])
            && words.windows(2).any(|window| window == ["this", "turn"])
    };
    let has_shared_terminal_noun =
        crate::runtime_backend::families::object_filters::has_shared_terminal_object_noun(tokens);
    let mut filter = if has_shared_terminal_noun
        && let Some(filter) = parse_repeated_selector_domain_union_lexed(tokens, other)
    {
        // A single terminal noun can still follow two independently scoped
        // instances of the same selector, as in "creatures you control and
        // creature cards in your graveyard." Preserve that proven domain
        // union before taking the shared-noun path.
        filter
    } else if !has_shared_terminal_noun
        && let Some(filter) = parse_domain_union_object_filter_lexed(tokens, other)
    {
        filter
    } else if let Some(filter) = parse_extremum_object_filter_lexed(tokens, other)? {
        filter
    } else if !temporal_graveyard_history
        && let Some(filter) = parse_simple_object_filter_lexed(tokens, other)
    {
        filter
    } else {
        // The simple characteristic parser deliberately tolerates descriptive
        // tails, but a graveyard-entry history clause is executable target
        // legality. Let the relational parser consume that clause before a
        // noun-only filter can claim the input.
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
