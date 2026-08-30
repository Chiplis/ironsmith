use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectFilterGrammarDomain {
    Characteristic,
    Relational,
}

fn classify_object_filter_grammar_domain(tokens: &[OwnedLexToken]) -> ObjectFilterGrammarDomain {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let has_temporal_graveyard_history =
        crate::word_primitives::sequence_occurs(&words, &["put", "there"])
            && crate::word_primitives::sequence_occurs(&words, &["this", "turn"]);
    let has_power_toughness_relation =
        crate::slice_primitives::find_window_by(&words, 4, |window| {
            crate::word_primitives::parse_choice_sequence_complete(
                window,
                &[&["power"], &["and"], &["toughness"], &["aren't", "arent"]],
            )
        })
        .is_some()
            || crate::word_primitives::sequence_occurs(
                &words,
                &["power", "and", "toughness", "are", "not"],
            );
    let has_supertype_subtype_disjunction =
        crate::slice_primitives::find_window_by(&words, 3, |window| {
            window[0] == "legendary" && window[1] == "or" && parse_subtype_word(window[2]).is_some()
        })
        .is_some();
    let has_target_count_relation = crate::word_primitives::parse_any_sequence_suffix(
        &words,
        &[
            &["with", "a", "single", "target"],
            &["with", "a", "single", "targets"],
        ],
    );

    if has_temporal_graveyard_history
        || has_power_toughness_relation
        || has_supertype_subtype_disjunction
        || has_target_count_relation
    {
        ObjectFilterGrammarDomain::Relational
    } else {
        ObjectFilterGrammarDomain::Characteristic
    }
}

pub fn parse_object_filter_with_grammar_entrypoint(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    // Relationship-bearing phrases and characteristic-only phrases are
    // disjoint grammar domains. Classify the complete token slice before
    // invoking either parser so a tolerant noun parser is never a competing
    // candidate for executable P/T, history, disjunction, or target-count
    // semantics.
    let domain = classify_object_filter_grammar_domain(tokens);
    if domain == ObjectFilterGrammarDomain::Relational {
        let mut filter = parse_object_filter(tokens, other)?;
        preserve_filter_counter_constraint_surface_tokens(&mut filter, tokens);
        return Ok(filter);
    }

    let has_shared_terminal_noun = crate::object_filters::has_shared_terminal_object_noun(tokens);
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
    } else if let Some(filter) = parse_simple_object_filter_lexed(tokens, other) {
        filter
    } else {
        // A characteristic-domain phrase can still require the complete
        // relational parser for less common modifiers not owned by a narrow
        // leaf. Reaching this branch means every characteristic candidate
        // reported no match; it is not a registration-order decision.
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
