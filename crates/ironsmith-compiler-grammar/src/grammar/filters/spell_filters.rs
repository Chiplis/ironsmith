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

    // The noun-only grammar cannot own references, player/zone scope,
    // event history, attachment/combat relations, or characteristic predicates.
    // Select their established relational grammar before noun/union readers can
    // discard those operands. This is classification, not a fallback after an
    // unsupported characteristic suffix has already been rejected.
    let has_relational_operand = words.iter().any(|word| {
        matches!(
            *word,
            "this"
                | "that"
                | "those"
                | "it"
                | "its"
                | "them"
                | "their"
                | "you"
                | "your"
                | "other"
                | "another"
                | "chosen"
                | "rest"
                | "among"
                | "control"
                | "controls"
                | "controlled"
                | "owner"
                | "owners"
                | "opponent"
                | "opponents"
                | "player"
                | "players"
                | "battlefield"
                | "graveyard"
                | "graveyards"
                | "hand"
                | "hands"
                | "library"
                | "libraries"
                | "exile"
                | "entered"
                | "milled"
                | "revealed"
                | "exiled"
                | "discarded"
                | "sacrificed"
                | "cast"
                | "turn"
                | "turns"
                | "each"
                | "attached"
                | "enchanted"
                | "equipped"
                | "attacking"
                | "blocking"
                | "blocked"
                | "defending"
                | "attacked"
                | "mana"
                | "power"
                | "toughness"
                | "counter"
                | "counters"
                | "ability"
                | "abilities"
                | "type"
                | "types"
                | "kicked"
                | "freerunning"
                | "convoke"
                // A literal name is a reference, not a characteristic: the
                // noun-only grammar has no slot for one and would drop the
                // name while still recognizing the noun.
                | "named"
                | "name"
                | "names"
                // Combat relations. A negated combat relation is still a
                // relation; the characteristic grammar knows neither form.
                | "nonattacking"
                | "nonblocking"
                // Ownership, in the inflections the corpus actually uses
                // beside the `owner`/`owners` already listed.
                | "own"
                | "owns"
                | "owned"
                | "they"
                // `that` is listed; its contraction is the same operand.
                | "thats"
                | "that's"
                // A characteristic predicate is relational whichever number
                // the sentence puts it in.
                | "powers"
                | "toughnesses"
                // A targeting marker survives into the slice whenever a
                // caller strips only a trailing decoration, as the extremum
                // reader does. It is a reference marker, never a
                // characteristic.
                | "target"
                | "targets"
                | "targeted"
        )
    }) || crate::util::is_source_reference_words(&words);
    let has_protector_relation = words
        .iter()
        .any(|word| matches!(*word, "protect" | "protects" | "protected"));
    if has_relational_operand
        || has_protector_relation
        || has_temporal_graveyard_history
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
    if tokens.first().is_some_and(|token| token.is_word("all")) {
        let mut filter = parse_object_filter_with_grammar_entrypoint(&tokens[1..], other)?;
        filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All));
        return Ok(filter);
    }
    if let Some(first) = tokens.first().and_then(OwnedLexToken::as_word)
        && let Ok((power, toughness)) = crate::keyword_static::parse_pt_modifier(first)
    {
        let mut filter = parse_object_filter_with_grammar_entrypoint(&tokens[1..], other)?;
        filter.power = Some(crate::filter::Comparison::Equal(power));
        filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
        return Ok(filter);
    }
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words == ["spell", "or", "permanent"] || words == ["permanent", "or", "spell"] {
        return Ok(ObjectFilter {
            other,
            any_of: vec![ObjectFilter::spell(), ObjectFilter::permanent()],
            ..ObjectFilter::default()
        });
    }
    if let Some((index, introducer_len)) = tokens.iter().enumerate().find_map(|(index, token)| {
        if token.is_word("that's") || token.is_word("thats") {
            Some((index, 1))
        } else if token.is_word("that") && tokens.get(index + 1).is_some_and(|next| next.is_word("is") || next.is_word("are")) {
            Some((index, 2))
        } else { None }
    }) {
        let tail = &tokens[index + introducer_len..];
        let mut colors = crate::ColorSet::new();
        let mut expect_color = true;
        let complete_colors = !tail.is_empty() && tail.iter().all(|token| {
            if expect_color {
                if let Some(color) = crate::util::parse_color(token.parser_text()) {
                    colors = colors.union(color);
                    expect_color = false;
                    true
                } else { false }
            } else if token.is_word("or") {
                expect_color = true;
                true
            } else { false }
        }) && !expect_color;
        if complete_colors {
            let mut filter = parse_object_filter_with_grammar_entrypoint(&tokens[..index], other)?;
            if filter.colors.is_none() {
                filter.colors = Some(colors);
                return Ok(filter);
            }
        }
    }
    // "permanent that's one or more colors" (Ugin, Eye of the Storms),
    // "creatures of one or more colors": a trailing color-count phrase
    // narrows the head noun phrase.
    if let Some((head_tokens, count)) = split_trailing_color_count_phrase_tokens(tokens) {
        if count >= 3 {
            return Err(CardTextError::ParseError(format!(
                "unsupported color-count object filter '{}'",
                crate::lexer::render_token_slice(tokens)
            )));
        }
        let mut filter = parse_object_filter_with_grammar_entrypoint(head_tokens, other)?;
        if count == 1 {
            filter.colors = Some(crate::color::Color::ALL.into_iter().collect());
        } else {
            filter.color_count = Some(crate::filter::Comparison::GreaterThanOrEqual(count as i32));
        }
        preserve_filter_counter_constraint_surface_tokens(&mut filter, tokens);
        return Ok(filter);
    }
    // An extremum owns the outer comparison and parses its comparison set
    // independently. Preserve that structure before classifying its operands.
    if let Some(mut filter) = parse_extremum_object_filter_lexed(tokens, other)? {
        preserve_filter_counter_constraint_surface_tokens(&mut filter, tokens);
        return Ok(filter);
    }
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

    // Consume a recognized complete keyword/counter suffix structurally,
    // leaving the characteristic head subject to the same strict grammar.
    if let Some(split) = parse_filter_tail_decoration_tokens(tokens) {
        let mut filter = parse_object_filter_with_grammar_entrypoint(&split.base_tokens, other)?;
        apply_filter_tail_decoration(&mut filter, split.decoration);
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
    } else if let Some(filter) = parse_simple_object_filter_lexed(tokens, other) {
        filter
    } else {
        // The complete phrase was classified as characteristic-only, but no
        // characteristic grammar consumed it. The relational scanner can
        // recognize a noun while silently ignoring an unknown suffix; it
        // must not turn this failure into a successful partial filter.
        return Err(CardTextError::ParseError(format!(
            "unsupported complete object filter: {}",
            crate::lexer::render_token_slice(tokens)
        )));
    };
    preserve_filter_counter_constraint_surface_tokens(&mut filter, tokens);
    Ok(filter)
}

/// Split "<head> of|that's <one or more> colors" into the head tokens and the
/// minimum color count. The linking word and the count phrase must run to the
/// end of the slice.
fn split_trailing_color_count_phrase_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], u32)> {
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let starts = view.token_start_indices();
    for (link_idx, link) in words.iter().enumerate().skip(1) {
        if !matches!(*link, "of" | "thats" | "that's" | "that") {
            continue;
        }
        let Some((count, consumed)) =
            super::naming_and_reference::parse_color_count_phrase_words(&words[link_idx + 1..])
        else {
            continue;
        };
        if link_idx + 1 + consumed != words.len() {
            continue;
        }
        let link_token = *starts.get(link_idx)?;
        let head = crate::lexer::trim_lexed_commas(&tokens[..link_token]);
        if head.is_empty() {
            return None;
        }
        return Some((head, count));
    }
    None
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
