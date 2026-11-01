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
    parse_repeated_spell_domain_union(tokens)
        .or_else(|| parse_special_spell_filter(tokens))
        .or_else(|| parse_simple_subtype_spell_filter(tokens))
        .or_else(|| {
            let filter = parse_spell_filter_with_grammar_entrypoint(tokens);
            (filter != ObjectFilter::default()).then_some(filter)
        })
}

/// Independently nouned spell alternatives retain their own predicates.
/// A subtype on one arm must not swallow an ability requirement on another.
/// "an instant or sorcery spell or a kicked spell" (Volshe Tideturner): an
/// "or" ends an arm only after its noun ("spell"/"spells") or before a new
/// article; "instant or sorcery spell" stays one arm.
pub(super) fn parse_repeated_spell_domain_union(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let probe = |token: &OwnedLexToken, words: &[&str]| {
        crate::word_claims::probe_only(|| token.is_any_word(words))
    };
    if tokens.iter().any(|token| probe(token, &["and"])) {
        return None;
    }
    let separators = tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            probe(token, &["or"])
                && (index
                    .checked_sub(1)
                    .and_then(|previous| tokens.get(previous))
                    .is_some_and(|previous| probe(previous, &["spell", "spells"]))
                    || tokens
                        .get(index + 1)
                        .is_some_and(|next| probe(next, &["a", "an"])))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if separators.is_empty() {
        return None;
    }
    let mut arms = Vec::with_capacity(separators.len() + 1);
    let mut start = 0;
    for &separator in &separators {
        arms.push(trim_lexed_commas(&tokens[start..separator]));
        start = separator + 1;
    }
    arms.push(trim_lexed_commas(&tokens[start..]));
    if arms.len() < 2
        || !arms.iter().all(|arm| {
            arm.iter().any(|token| probe(token, &["spell", "spells"]))
        })
    {
        return None;
    }
    // Each arm is a spell description, read by the spell-filter grammar
    // that owns spell-only words ("kicked", "Omen" spells), exactly as a
    // single-arm restriction is read.
    let branches = arms
        .into_iter()
        .map(|arm| {
            // A relative clause on the arm ("a spell that has freerunning",
            // "a spell with mana value 3 or less") is the object-filter
            // grammar's; read it there, as a stack object.
            if arm.iter().any(|token| probe(token, &["that", "with", "has", "have"]))
                && let Ok(filter) =
                    crate::grammar::filters::parse_object_filter_with_grammar_entrypoint(arm, false)
                && filter.zone == Some(Zone::Stack)
            {
                return Some(filter);
            }
            let filter = parse_spell_filter_with_grammar_entrypoint(arm);
            (filter != ObjectFilter::default()).then_some(filter)
        })
        .collect::<Option<Vec<_>>>()?;
    // The arm-ending "or"s join the alternatives read above.
    for separator in separators {
        crate::word_claims::claim_tokens(&tokens[separator..=separator]);
    }
    Some(crate::grammar::filters::spell_filter_union(branches))
}

pub(super) fn parse_simple_subtype_spell_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let tokens = strip_article(trim_lexed_commas(tokens));
    let words = TokenWordView::new(tokens).word_refs();
    let [subtype_word, spell_word] = words.as_slice() else {
        return None;
    };
    matches!(*spell_word, "spell" | "spells").then_some(())?;
    if crate::util::is_outlaw_word(subtype_word) {
        let mut filter = ObjectFilter::default();
        crate::util::push_outlaw_subtypes(&mut filter.subtypes);
        crate::word_claims::claim_words(&words);
        return Some(filter);
    }
    let subtype = crate::grammar::primitives::probe_shape(
        leaf::parse_leaf_subtype_flexible_complete(subtype_word),
    )?;
    // "Dragon spells": the subtype and the spell noun this filter reads.
    crate::word_claims::claim_words(&words);
    Some(ObjectFilter::default().with_subtype(subtype))
}

pub(super) fn parse_ability_source_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter = read_ability_source_filter(tokens)?;
    // A trailing "source(s)" is the noun the descriptor qualifies.
    let tokens = strip_article(trim_lexed_commas(tokens));
    if let Some(last) = tokens.iter().rposition(|token| token.as_word().is_some())
        && tokens[last].as_word().is_some_and(|word| matches!(word, "source" | "sources"))
    {
        crate::word_claims::claim_tokens(&tokens[last..=last]);
    }
    Some(filter)
}

fn read_ability_source_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
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
    if matches!(semantic_words.as_slice(), ["outlaw" | "outlaws"]) {
        let mut filter = ObjectFilter::default();
        crate::util::push_outlaw_subtypes(&mut filter.subtypes);
        return Some(filter);
    }

    let [kind] = semantic_words.as_slice() else {
        // "colorless Eldrazi" (Eldrazi Temple): a multi-word permanent
        // descriptor reads through the generic object-filter grammar.
        let mut filter =
            crate::grammar::filters::parse_object_filter_with_grammar_entrypoint(semantic, false)
                .ok()?;
        filter.zone = None;
        return (filter != ObjectFilter::default()).then_some(filter);
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
    {
        let words = TokenWordView::new(tokens).word_refs();
        // "a spell that's one or more colors without {X} in its mana cost"
        // (Titans' Nest).
        if matches!(
            words.as_slice(),
            ["colored", "spell" | "spells", "without", "x", "in", "its" | "their", "mana", "cost" | "costs"]
                | ["spell" | "spells", "that's" | "thats" | "that", "one", "or", "more", "colors", "without", "x", "in", "its" | "their", "mana", "cost" | "costs"]
                | ["spell" | "spells", "that", "is" | "are", "one", "or", "more", "colors", "without", "x", "in", "its" | "their", "mana", "cost" | "costs"]
        ) {
            let mut filter = ObjectFilter::default();
            filter.colors = Some(crate::color::Color::ALL.into_iter().collect());
            filter.no_x_in_cost = true;
            return Some(filter);
        }
        // "[creature] spells with mana value N or greater or [creature]
        // spells with {X} in their mana costs" (Helga, Troyan).
        let card_type_of = |word: &str| crate::util::parse_card_type(word);
        let (card_type, rest) = match words.as_slice() {
            [first, rest @ ..] if card_type_of(first).is_some() => (card_type_of(first), rest),
            rest => (None, rest),
        };
        if let ["spells" | "spell", "with", "mana", "value", amount, "or", "greater", "or", tail @ ..] = rest
            && let Ok(amount) = amount.parse::<i32>()
        {
            let tail = match (card_type, tail) {
                (Some(_), [first, tail @ ..]) if card_type_of(first) == card_type => tail,
                (Some(_), _) => &[][..],
                (None, tail) => tail,
            };
            if matches!(
                tail,
                ["spells" | "spell", "with", "x", "in", "their" | "its", "mana", "costs" | "cost"]
            ) {
                let base = card_type
                    .map(|card_type| ObjectFilter::default().with_type(card_type))
                    .unwrap_or_default();
                let mut high = base.clone();
                high.mana_value = Some(crate::filter::Comparison::GreaterThanOrEqual(amount));
                let mut with_x = base;
                with_x.has_x_in_cost = true;
                let mut filter = ObjectFilter::default();
                filter.any_of = vec![high, with_x];
                return Some(filter);
            }
        }
    }
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
