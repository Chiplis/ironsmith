use super::*;

pub fn parse_copy_modifier_words(tail_words: &[&str]) -> Result<CopyModifierSpec, CardTextError> {
    let modifier_words = last_class_location(tail_words, CreationWordClass::Except)
        .and_then(|idx| tail_words.get(idx + 1..))
        .unwrap_or_default();
    let surface = CreationWords::new(modifier_words);
    let mut spec = CopyModifierSpec::default();
    if modifier_words.is_empty() {
        return Ok(spec);
    }

    if surface.has(CreationWordClass::LoseVerb) && surface.has(CreationWordClass::Soulbond) {
        // "loses soulbond" (Mirage Phalanx): the copy is created without the
        // soulbond pairing ability. Only the adjacent "lose(s) soulbond" pair
        // has that meaning; anything else stays unsupported.
        if crate::slice_primitives::find_window_by(modifier_words, 2, |pair| {
            matches!(pair[0], "lose" | "loses") && pair[1] == "soulbond"
        })
        .is_some()
        {
            spec.loses_soulbond = true;
        } else {
            return Err(CardTextError::ParseError(
                "removing soulbond requires non-marker semantics".to_string(),
            ));
        }
    }
    if surface.has_phrase(CreationPhrase::NotLegendary) {
        spec.removed_supertypes.push(Supertype::Legendary);
    }
    spec.set_base_power_toughness = modifier_words
        .iter()
        .filter_map(|word| parse_unsigned_pt_word(word))
        .next();
    let power_is_source_total = contains_words(
        modifier_words,
        &[
            "its",
            "power",
            "is",
            "equal",
            "to",
            "the",
            "total",
            "power",
            "of",
            "those",
            "creatures",
        ],
    );
    let toughness_is_source_total = contains_words(
        modifier_words,
        &[
            "its",
            "toughness",
            "is",
            "equal",
            "to",
            "the",
            "total",
            "toughness",
            "of",
            "those",
            "creatures",
        ],
    );
    if power_is_source_total != toughness_is_source_total {
        return Err(CardTextError::ParseError(
            "copy aggregate power/toughness exception must preserve both values".to_string(),
        ));
    }
    spec.set_base_power_toughness_to_source_totals = power_is_source_total;

    spec.starting_loyalty = crate::slice_primitives::find_window_by(modifier_words, 4, |words| {
        crate::word_primitives::parse_sequence_complete(&words[..3], &["starting", "loyalty", "is"])
    })
    .and_then(|start| modifier_words.get(start..start + 4))
    .and_then(|words| crate::util::parse_number_word_u32(words[3]));

    let grants_keyword = |phrase, keyword: &str| {
        surface.has_phrase(phrase)
            || (surface.has(CreationWordClass::GrantVerb)
                && CreationWords::new(modifier_words).has_literal(keyword))
    };
    if grants_keyword(CreationPhrase::WithFlying, "flying") {
        spec.granted_abilities.push(StaticAbility::flying());
    }
    if grants_keyword(CreationPhrase::WithTrample, "trample") {
        spec.granted_abilities.push(StaticAbility::trample());
    }
    if let Some(amount) =
        crate::slice_primitives::find_window_by(modifier_words, 2, |pair| pair[0] == "toxic")
            .and_then(|start| modifier_words.get(start..start + 2))
            .and_then(|pair| crate::util::decimal_count(pair[1]))
    {
        push_unique(
            &mut spec.granted_abilities,
            StaticAbility::keyword_marker(format!("toxic {amount}")),
        );
    }

    if let Some(idx) = surface.phrase_location(CreationPhrase::GetsForEach) {
        let mut tail = modifier_words.get(idx + 6..).unwrap_or_default();
        while CreationWords::new(tail).first_is(CreationWordClass::ArticleOrThe) {
            tail = &tail[1..];
        }
        if let Some(subtype) = tail
            .first()
            .and_then(|word| crate::util::parse_subtype_flexible(word))
            && CreationWords::new(tail).has_phrase(CreationPhrase::YouControl)
        {
            let mut filter = ObjectFilter::default();
            filter.zone = Some(Zone::Battlefield);
            filter.controller = Some(PlayerFilter::You);
            filter.subtypes = vec![subtype];
            let count = AnthemCountExpression::MatchingFilter(filter);
            let anthem = crate::model::CompilerAnthem::for_source(0, 0).with_values(
                AnthemValue::scaled(1, count.clone()),
                AnthemValue::scaled(1, count),
            );
            spec.granted_abilities.push(StaticAbility::new(anthem));
        }
    }

    if let Some(addition) = surface.phrase_location(CreationPhrase::AdditionToOtherTypes) {
        let mut colors = ColorSet::new();
        for word in &modifier_words[..addition] {
            if let Some(color) = crate::util::parse_color(word) {
                colors = colors.union(color);
            }
            if let Some(card_type) = crate::util::parse_card_type(word) {
                push_unique(&mut spec.added_card_types, card_type);
            }
            if let Some(subtype) = crate::util::parse_subtype_flexible(word) {
                push_unique(&mut spec.added_subtypes, subtype);
            }
        }
        let explicit_colorless =
            crate::word_primitives::contains_word(&modifier_words[..addition], "colorless");
        spec.set_colors = (explicit_colorless || !colors.is_empty()).then_some(colors);
    } else if surface.starts(CreationPhrase::IdentityClause) {
        let descriptor_end = surface
            .location(CreationWordClass::DescriptorEnd)
            .unwrap_or(modifier_words.len());
        let mut colors = ColorSet::new();
        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &modifier_words[..descriptor_end] {
            if CreationWords::new(&[*word]).first_is(CreationWordClass::Article)
                || matches!(
                    *word,
                    "its"
                        | "it"
                        | "is"
                        | "s"
                        | "it's"
                        | "it’s"
                        | "they"
                        | "are"
                        | "re"
                        | "theyre"
                        | "they're"
                        | "they’re"
                )
                || parse_pt_word(word).is_some()
            {
                continue;
            }
            if let Some(color) = crate::util::parse_color(word) {
                colors = colors.union(color);
            }
            if let Some(card_type) = crate::util::parse_card_type(word) {
                push_unique(&mut card_types, card_type);
            }
            if let Some(subtype) = crate::util::parse_subtype_flexible(word) {
                push_unique(&mut subtypes, subtype);
            }
        }
        let explicit_colorless =
            crate::word_primitives::contains_word(&modifier_words[..descriptor_end], "colorless");
        spec.set_colors = (explicit_colorless || !colors.is_empty()).then_some(colors);
        spec.set_card_types = (!card_types.is_empty()).then_some(card_types);
        spec.set_subtypes = (!subtypes.is_empty()).then_some(subtypes);
    }
    Ok(spec)
}
