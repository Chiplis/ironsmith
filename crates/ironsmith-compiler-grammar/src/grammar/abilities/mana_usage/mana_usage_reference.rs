use super::*;

pub(super) fn parse_filter_cast_shape(tokens: &[OwnedLexToken]) -> Option<FilterCastShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let spec_start = parse_any_prefix_word_count(&words, SPEND_MANA_CAST_PREFIXES)?;
    if spec_start >= words.len() {
        return None;
    }
    if let Some(suffix) = last_exact_suffix_offset(words.get(spec_start..)?, UNCOUNTERABLE_TAILS) {
        if suffix == 0 {
            return None;
        }
        return Some(FilterCastShape {
            spec_tokens: token_slice_for_words(tokens, &view, spec_start, spec_start + suffix)?,
            grant_uncounterable: true,
        });
    }
    Some(FilterCastShape {
        spec_tokens: token_slice_for_words(tokens, &view, spec_start, words.len())?,
        grant_uncounterable: false,
    })
}

pub(super) fn parse_mana_usage_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    parse_special_spell_filter(tokens)
        .or_else(|| parse_simple_subtype_spell_filter(tokens))
        .or_else(|| {
            let filter = parse_spell_filter_with_grammar_entrypoint(tokens);
            (filter != ObjectFilter::default()).then_some(filter)
        })
}

pub(super) fn parse_simple_subtype_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = strip_article(trim_lexed_commas(tokens));
    let words = TokenWordView::new(tokens).word_refs();
    let [subtype_word, spell_word] = words.as_slice() else {
        return None;
    };
    matches!(*spell_word, "spell" | "spells").then_some(())?;
    Some(
        ObjectFilter::default().with_subtype(crate::grammar::primitives::probe_shape(
            leaf::parse_leaf_subtype_flexible_complete(subtype_word),
        )?),
    )
}

pub(super) fn parse_ability_source_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = strip_article(trim_lexed_commas(tokens));
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let semantic_end = if words
        .last()
        .is_some_and(|word| matches!(*word, "source" | "sources"))
    {
        words.len().saturating_sub(1)
    } else {
        words.len()
    };
    if semantic_end == 0 {
        return None;
    }
    let semantic = token_slice_for_words(tokens, &view, 0, semantic_end)?;
    let parsed = parse_spell_filter_with_grammar_entrypoint(semantic);
    if parsed != ObjectFilter::default() {
        return Some(parsed);
    }

    let semantic_words = TokenWordView::new(semantic).word_refs();
    let [kind] = semantic_words.as_slice() else {
        return None;
    };
    match *kind {
        "artifact" | "artifacts" => Some(ObjectFilter::default().with_type(CardType::Artifact)),
        "creature" | "creatures" => Some(ObjectFilter::default().with_type(CardType::Creature)),
        "land" | "lands" => Some(ObjectFilter::default().with_type(CardType::Land)),
        _ => Some(
            ObjectFilter::default().with_subtype(crate::grammar::primitives::probe_shape(
                leaf::parse_leaf_subtype_flexible_complete(kind),
            )?),
        ),
    }
}

pub(super) fn parse_special_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    if let Some(filter) = parse_alternative_cast_spell_with_origin(tokens) {
        return Some(filter);
    }
    let tokens = strip_article(tokens);
    if matches_any_exact_tokens(
        tokens,
        &[
            &["monocolored", "spell", "of", "that", "color"],
            &["monocolored", "spells", "of", "that", "color"],
            &["monocolored", "spell", "of", "the", "chosen", "color"],
            &["monocolored", "spells", "of", "the", "chosen", "color"],
        ],
    ) {
        return Some(ObjectFilter::default().monocolored().of_chosen_color());
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["your", "commander"],
            &["your", "commander", "spell"],
            &["your", "commander", "spells"],
        ],
    ) {
        return Some(
            ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
        );
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["spell", "from", "your", "graveyard"],
            &["spells", "from", "your", "graveyard"],
        ],
    ) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }
    if matches_any_exact_tokens(
        tokens,
        &[&["spell", "from", "exile"], &["spells", "from", "exile"]],
    ) {
        return Some(ObjectFilter::default().in_zone(Zone::Exile));
    }
    if matches_any_exact_tokens(
        tokens,
        &[&["spell", "with", "devoid"], &["spells", "with", "devoid"]],
    ) {
        return Some(ObjectFilter::default().with_static_ability(StaticAbilityId::MakeColorless));
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["creature", "spell", "with", "no", "abilities"],
            &["creature", "spells", "with", "no", "abilities"],
        ],
    ) {
        let mut filter = ObjectFilter::default().with_type(CardType::Creature);
        filter.no_abilities = true;
        return Some(filter);
    }
    if matches_any_exact_tokens(
        tokens,
        &[
            &["spell", "you", "don't", "own"],
            &["spell", "you", "dont", "own"],
            &["spells", "you", "don't", "own"],
            &["spells", "you", "dont", "own"],
        ],
    ) {
        return Some(ObjectFilter::default().owned_by(PlayerFilter::NotYou));
    }
    None
}

pub(super) fn parse_nondefault_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter = parse_spell_filter_with_grammar_entrypoint(tokens);
    (filter != ObjectFilter::default()).then_some(filter)
}
