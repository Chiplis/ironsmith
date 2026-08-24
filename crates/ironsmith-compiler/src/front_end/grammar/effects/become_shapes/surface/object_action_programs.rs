use super::*;

pub(super) fn parse_structured_become_copy_exception_shape(
    tokens: &[OwnedLexToken],
) -> Option<BecomeCopyExceptionShape> {
    const TYPE_ADDITION_TAILS: &[&[&str]] = &[
        &["in", "addition", "to", "its", "other", "types"],
        &["in", "addition", "to", "his", "other", "types"],
        &["in", "addition", "to", "her", "other", "types"],
        &["in", "addition", "to", "their", "other", "types"],
    ];

    let tokens = trim_lexed_commas(tokens);
    let mut parsed = BecomeCopyExceptionShape {
        surface: Some(render_token_slice(tokens).trim().to_string()),
        ..Default::default()
    };
    let (mut descriptor_tokens, prefixed_has_tokens) = if let Some((_, name_tail)) =
        primitives::strip_lexed_prefix_phrases(tokens, COPY_NAME_PREFIXES)
    {
        let (start, end, kind) = find_copy_exception_followup(name_tail, true)?;
        let name_tokens = trim_lexed_commas(&name_tail[..start]);
        if name_tokens.is_empty() {
            return None;
        }
        let name_words = parser_token_word_refs(name_tokens);
        parsed.name_override_surface = crate::util::source_reference_surface_for_words(&name_words);
        parsed.name_override = Some(render_token_slice(name_tokens).trim().to_string());
        match kind {
            CopyExceptionFollowupKind::Copula => (trim_lexed_commas(&name_tail[end..]), None),
            CopyExceptionFollowupKind::Has => (&[][..], Some(trim_lexed_commas(&name_tail[end..]))),
        }
    } else {
        // Match only copular contractions here. Bare possessive `its` is a
        // different surface and must remain available to the name-prefix
        // and legacy exception parsers.
        let (_, rest) = primitives::strip_lexed_prefix_phrases(
            tokens,
            &[
                &["it's"],
                &["it’s"],
                &["it", "s"],
                &["he's"],
                &["he’s"],
                &["he", "s"],
                &["she's"],
                &["she’s"],
                &["she", "s"],
            ],
        )?;
        (trim_lexed_commas(rest), None)
    };

    let ability_tokens = if let Some(tokens) = prefixed_has_tokens {
        Some(tokens)
    } else if let Some((start, end, kind)) = find_copy_exception_followup(descriptor_tokens, false)
    {
        if kind != CopyExceptionFollowupKind::Has {
            return None;
        }
        let ability_tokens = trim_lexed_commas(&descriptor_tokens[end..]);
        descriptor_tokens = trim_lexed_commas(&descriptor_tokens[..start]);
        Some(ability_tokens)
    } else {
        None
    };

    if let Some(ability_tokens) = ability_tokens {
        if permission_shapes::exact_tokens(ability_tokens, &["this", "ability"]) {
            parsed.preserve_source_abilities = true;
        } else {
            let (ability_tokens, preserve_source_abilities) = if let Some((_, head)) =
                primitives::strip_lexed_suffix_phrases(
                    ability_tokens,
                    &[&["and", "this", "ability"], &["this", "ability"]],
                ) {
                (trim_lexed_commas(head), true)
            } else {
                (trim_lexed_commas(ability_tokens), false)
            };
            if ability_tokens.is_empty() && !preserve_source_abilities {
                return None;
            }
            parsed.preserve_source_abilities = preserve_source_abilities;
            if !ability_tokens.is_empty() {
                parsed.granted_ability_tokens = Some(ability_tokens.to_vec());
            }
        }
    }

    let mut descriptor_words = parser_token_word_refs(descriptor_tokens);
    let preserve_other_types = TYPE_ADDITION_TAILS.iter().any(|tail| {
        if permission_shapes::suffix_words(&descriptor_words, tail) {
            descriptor_words.truncate(descriptor_words.len().saturating_sub(tail.len()));
            true
        } else {
            false
        }
    });
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for word in descriptor_words {
        if matches!(word, "a" | "an" | "and" | "its" | "hes" | "shes") {
            continue;
        }
        if let Some(power_toughness) = parse_fixed_power_toughness(word) {
            if parsed
                .set_base_power_toughness
                .replace(power_toughness)
                .is_some()
            {
                return None;
            }
        } else if let Ok(supertype) = leaf::parse_leaf_supertype_complete(word) {
            if !crate::slice_primitives::contains(&parsed.add_supertypes, &supertype) {
                parsed.add_supertypes.push(supertype);
            }
        } else if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
            if !crate::slice_primitives::contains(&card_types, &card_type) {
                card_types.push(card_type);
            }
        } else if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            if !crate::slice_primitives::contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
        } else {
            return None;
        }
    }

    if preserve_other_types {
        parsed.add_card_types = card_types;
        parsed.add_subtypes = subtypes;
    } else {
        // The compiled continuous model represents creature-subtype setting
        // as "remove all creature types, then add these." Do not claim a
        // broader replacement shape until its subtype family is modeled.
        if subtypes.iter().any(|subtype| !subtype.is_creature_type()) {
            return None;
        }
        parsed.set_card_types = card_types;
        parsed.set_subtypes = subtypes;
    }
    let has_typed_exception = parsed.name_override.is_some()
        || parsed.preserve_source_abilities
        || parsed.set_base_power_toughness.is_some()
        || !parsed.add_supertypes.is_empty()
        || !parsed.add_card_types.is_empty()
        || !parsed.set_card_types.is_empty()
        || !parsed.add_subtypes.is_empty()
        || !parsed.set_subtypes.is_empty()
        || parsed.granted_ability_tokens.is_some();
    has_typed_exception.then_some(parsed)
}

pub fn parse_become_copy_exception_shape(
    tokens: &[OwnedLexToken],
) -> Option<BecomeCopyExceptionShape> {
    let tokens = trim_lexed_commas(tokens);
    if let Some(parsed) = parse_structured_become_copy_exception_shape(tokens) {
        return Some(parsed);
    }
    if permission_shapes::exact_tokens(tokens, &["it", "isn't", "legendary"])
        || permission_shapes::exact_tokens(tokens, &["it", "isnt", "legendary"])
        || permission_shapes::exact_tokens(tokens, &["it", "is", "not", "legendary"])
    {
        return Some(BecomeCopyExceptionShape {
            remove_supertypes: vec![Supertype::Legendary],
            ..Default::default()
        });
    }
    if permission_shapes::exact_tokens(tokens, &["it", "has", "this", "ability"]) {
        return Some(BecomeCopyExceptionShape {
            preserve_source_abilities: true,
            ..Default::default()
        });
    }
    if let Some((_, ability_tokens)) = primitives::strip_lexed_prefix_phrases(
        tokens,
        &[&["it", "has"], &["he", "has"], &["she", "has"]],
    ) {
        let ability_tokens = trim_lexed_commas(ability_tokens);
        if !ability_tokens.is_empty() {
            return Some(BecomeCopyExceptionShape {
                granted_ability_tokens: Some(ability_tokens.to_vec()),
                ..Default::default()
            });
        }
    }

    let (_, mut name_tokens) = primitives::strip_lexed_prefix_phrases(tokens, COPY_NAME_PREFIXES)?;
    let mut parsed = BecomeCopyExceptionShape::default();

    // Copy exceptions may preserve several printed characteristics at once,
    // for example "except his name is ..., he's 4/4, and he has flying and
    // this ability." Keep those as typed copy-layer/characteristic-layer
    // adjustments instead of discarding the whole exception tail.
    if let Some(pt_index) = crate::slice_primitives::select_position(name_tokens, |token| {
        parse_fixed_power_toughness(token.parser_text()).is_some()
    }) {
        let (power, toughness) = parse_fixed_power_toughness(name_tokens[pt_index].parser_text())?;
        let contracted_intro =
            crate::slice_primitives::select_last_position(&name_tokens[..pt_index], |token| {
                matches!(
                    token.parser_text(),
                    "hes" | "he's" | "shes" | "she's" | "its" | "it's"
                )
            });
        let split_intro =
            crate::slice_primitives::find_last_window_by(&name_tokens[..pt_index], 2, |pair| {
                matches!(pair[0].parser_text(), "he" | "she" | "it") && pair[1].parser_text() == "s"
            });
        let intro_index = match (contracted_intro, split_intro) {
            (Some(left), Some(right)) => left.max(right),
            (Some(index), None) | (None, Some(index)) => index,
            (None, None) => return None,
        };
        let rendered_name = render_token_slice(trim_lexed_commas(&name_tokens[..intro_index]))
            .trim()
            .to_string();
        if rendered_name.is_empty() {
            return None;
        }
        parsed.name_override_surface = crate::util::source_reference_surface_for_words(
            &parser_token_word_refs(trim_lexed_commas(&name_tokens[..intro_index])),
        );
        parsed.name_override = Some(rendered_name);
        parsed.set_base_power_toughness = Some((power, toughness));

        let after_pt = trim_lexed_commas(&name_tokens[pt_index + 1..]);
        let (_, ability_tokens) = primitives::strip_lexed_prefix_phrases(
            after_pt,
            &[
                &["and", "he", "has"],
                &["and", "she", "has"],
                &["and", "it", "has"],
                &["he", "has"],
                &["she", "has"],
                &["it", "has"],
            ],
        )?;
        let (ability_tokens, preserves_this_ability) = if let Some((_, head)) =
            primitives::strip_lexed_suffix_phrases(
                ability_tokens,
                &[&["and", "this", "ability"], &["this", "ability"]],
            ) {
            (trim_lexed_commas(head), true)
        } else {
            (trim_lexed_commas(ability_tokens), false)
        };
        parsed.preserve_source_abilities = preserves_this_ability;
        if !ability_tokens.is_empty() {
            parsed.granted_ability_tokens = Some(ability_tokens.to_vec());
        }
        return Some(parsed);
    }

    if let Some((_, head)) =
        primitives::strip_lexed_suffix_phrases(name_tokens, COPY_PRESERVE_TAILS)
    {
        parsed.preserve_source_abilities = true;
        name_tokens = head;
    }
    if let Some((_, head)) =
        primitives::strip_lexed_suffix_phrases(name_tokens, COPY_LEGENDARY_TAILS)
    {
        parsed.add_supertypes.push(Supertype::Legendary);
        name_tokens = head;
    }

    name_tokens = trim_lexed_commas(name_tokens);
    let name_words = parser_token_word_refs(name_tokens);
    if name_words.is_empty()
        || (parsed.add_supertypes.is_empty()
            && !parsed.preserve_source_abilities
            && permission_shapes::contains_tokens(name_tokens, &["and"]))
    {
        return None;
    }
    let rendered_name = render_token_slice(name_tokens).trim().to_string();
    if rendered_name.is_empty() {
        return None;
    }
    parsed.name_override_surface = crate::util::source_reference_surface_for_words(&name_words);
    parsed.name_override = Some(rendered_name);
    Some(parsed)
}

pub fn parse_become_rest_shape(tokens: &[OwnedLexToken]) -> BecomeRestShape {
    let tokens = trim_lexed_commas(tokens);
    let rest_tokens = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("become").void(),
            primitives::kw("becomes").void(),
        )),
    )
    .map(|(_, rest)| trim_lexed_commas(rest))
    .unwrap_or(tokens)
    .to_vec();
    // A copy exception is never part of the object being copied. Keep it out
    // of the copy-source tokens even when the exception itself is not yet
    // representable. In particular, this leaves a preceding duration at the
    // end of `body_tokens`, where the shared duration parser can preserve it.
    let copy_split = split_last_except(&rest_tokens)
        .filter(|(body, _)| permission_shapes::contains_tokens(body, &["copy", "of"]));
    let (body_tokens, copy_exception) = copy_split
        .map(|(body, exception)| (body.to_vec(), parse_become_copy_exception_shape(exception)))
        .unwrap_or_else(|| (rest_tokens.clone(), None));
    BecomeRestShape {
        rest_tokens,
        body_tokens,
        copy_exception,
    }
}

pub fn parse_become_body_surface_shape(tokens: &[OwnedLexToken]) -> BecomeBodySurfaceShape<'_> {
    let tokens = trim_lexed_commas(tokens);
    let body_tokens = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("the").void(),
            primitives::kw("a").void(),
            primitives::kw("an").void(),
        )),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens);
    let words = parser_token_word_refs(body_tokens);
    let exact_kind = if permission_shapes::exact_words(&words, &["monarch"]) {
        Some(BecomeExactKind::Monarch)
    } else if permission_shapes::exact_words(
        &words,
        &["basic", "land", "type", "of", "your", "choice"],
    ) {
        Some(BecomeExactKind::BasicLandTypeChoice)
    } else if let Some(subtype) = basic_land_type(&words) {
        Some(BecomeExactKind::BasicLandType(subtype))
    } else if let Some(allow_multiple) =
        COLOR_CHOICES
            .iter()
            .enumerate()
            .find_map(|(index, expected)| {
                permission_shapes::exact_words(&words, expected).then_some(index != 0)
            })
    {
        Some(BecomeExactKind::ColorChoice { allow_multiple })
    } else if permission_shapes::exact_words(&words, &["creature", "type", "of", "your", "choice"])
    {
        Some(BecomeExactKind::CreatureTypeChoice)
    } else if permission_shapes::exact_words(&words, &["colorless"]) {
        Some(BecomeExactKind::Colorless)
    } else if permission_shapes::exact_words(&words, &["saddled"]) {
        Some(BecomeExactKind::Saddled)
    } else {
        None
    };

    let copy_source = if let Some((_, source_tokens)) =
        primitives::parse_prefix(body_tokens, primitives::phrase(&["copy", "of"]).void())
    {
        let source_tokens = trim_lexed_commas(source_tokens);
        if source_tokens.is_empty() {
            BecomeCopySourceShape::Missing
        } else {
            BecomeCopySourceShape::Source(source_tokens)
        }
    } else {
        BecomeCopySourceShape::NotCopy
    };

    let aura_tail = primitives::parse_prefix(
        body_tokens,
        primitives::phrase(&["aura", "enchantment", "with", "enchant", "creature"]).void(),
    )
    .or_else(|| {
        primitives::parse_prefix(
            body_tokens,
            primitives::phrase(&["aura", "with", "enchant", "creature"]).void(),
        )
    })
    .map(|(_, tail)| tail);
    let aura = aura_tail.map(|tail_tokens| BecomeAuraShape {
        attachment_you_control: permission_shapes::prefix_tokens(tail_tokens, &["you", "control"]),
    });
    let equal_to_source_power_toughness =
        primitives::parse_prefix(body_tokens, primitives::phrase(&["equal", "to"]).void())
            .is_some_and(|(_, rhs)| {
                SOURCE_POWER_TOUGHNESS
                    .iter()
                    .any(|expected| permission_shapes::exact_tokens(rhs, expected))
            });

    BecomeBodySurfaceShape {
        body_tokens,
        exact_kind,
        copy_source,
        aura,
        equal_to_source_power_toughness,
    }
}
