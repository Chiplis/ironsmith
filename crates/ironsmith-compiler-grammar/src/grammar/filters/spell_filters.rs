use super::*;
pub fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let temporal_graveyard_history = {
        crate::word_primitives::sequence_occurs(&words, &["put", "there"])
            && crate::word_primitives::sequence_occurs(&words, &["this", "turn"])
    };
    // The fast simple-filter parser intentionally accepts unknown relative
    // tails. These two authored shapes carry executable relationships, so a
    // noun-only result is lossy: Gilt-Leaf's P/T inequality disappears and
    // Nashi's `legendary or Rat` becomes a conjunctive selector. Give the
    // relational parser first refusal only when the complete typed phrase is
    // present.
    let requires_relational_parser = crate::slice_primitives::find_window_by(&words, 4, |window| {
        crate::word_primitives::parse_choice_sequence_complete(
            window,
            &[&["power"], &["and"], &["toughness"], &["aren't", "arent"]],
        )
    })
    .is_some()
        || crate::word_primitives::sequence_occurs(
            &words,
            &["power", "and", "toughness", "are", "not"],
        )
        || crate::slice_primitives::find_window_by(&words, 3, |window| {
            window[0] == "legendary" && window[1] == "or" && parse_subtype_word(window[2]).is_some()
        })
        .is_some()
        || crate::word_primitives::parse_any_sequence_suffix(
            &words,
            &[
                &["with", "a", "single", "target"],
                &["with", "a", "single", "targets"],
            ],
        );
    let has_shared_terminal_noun = crate::object_filters::has_shared_terminal_object_noun(tokens);
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

pub fn parse_spell_filter_with_grammar_entrypoint_lexed(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let words_view = GrammarFilterNormalizedWords::new(tokens);
    let words = non_article_word_refs(&words_view.to_word_refs());

    parse_spell_filter_from_words(&words)
}

pub fn parse_spell_filter_with_grammar_entrypoint(tokens: &[OwnedLexToken]) -> ObjectFilter {
    let words = non_article_token_word_refs(tokens);

    parse_spell_filter_from_words(&words)
}
