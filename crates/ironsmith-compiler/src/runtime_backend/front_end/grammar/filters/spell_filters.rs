use super::*;
pub(crate) fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let temporal_graveyard_history = {
        words.windows(2).any(|window| window == ["put", "there"])
            && words.windows(2).any(|window| window == ["this", "turn"])
    };
    // The fast simple-filter parser intentionally accepts unknown relative
    // tails. These two authored shapes carry executable relationships, so a
    // noun-only result is lossy: Gilt-Leaf's P/T inequality disappears and
    // Nashi's `legendary or Rat` becomes a conjunctive selector. Give the
    // relational parser first refusal only when the complete typed phrase is
    // present.
    let requires_relational_parser = words
        .windows(4)
        .any(|window| matches!(window, ["power", "and", "toughness", "aren't" | "arent"]))
        || words
            .windows(5)
            .any(|window| window == ["power", "and", "toughness", "are", "not"])
        || words.windows(3).any(|window| {
            window[0] == "legendary" && window[1] == "or" && parse_subtype_word(window[2]).is_some()
        })
        || words.ends_with(&["with", "a", "single", "target"])
        || words.ends_with(&["with", "a", "single", "targets"]);
    let has_shared_terminal_noun =
        crate::runtime_backend::families::object_filters::has_shared_terminal_object_noun(tokens);
    let mut filter = if requires_relational_parser {
        parse_object_filter(tokens, other)?
    } else if has_shared_terminal_noun
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
