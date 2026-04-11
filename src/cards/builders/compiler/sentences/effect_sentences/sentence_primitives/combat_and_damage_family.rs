pub(crate) fn parse_sentence_destroy_creature_type_of_choice(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if grammar::strip_lexed_prefix_phrase(tokens, &["destroy", "all", "creatures"]).is_none() {
        return Ok(None);
    }
    if find_creature_type_choice_phrase(tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::ChooseCreatureType {
            player: PlayerAst::You,
            excluded_subtypes: vec![],
        },
        EffectAst::DestroyAll {
            filter: ObjectFilter::creature().of_chosen_creature_type(),
        },
    ]))
}

pub(crate) fn parse_sentence_pump_creature_type_of_choice(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(get_idx) = find_index(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    }) else {
        return Ok(None);
    };
    if get_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..get_idx]);
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(&subject_tokens) else {
        return Ok(None);
    };
    let trailing_subject = trim_commas(&subject_tokens[choice_idx + consumed..]);
    if !trailing_subject.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }
    let trimmed_subject_tokens = trim_commas(&subject_tokens[..choice_idx]).to_vec();
    if trimmed_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_tokens = trimmed_subject_tokens.clone();
    gain_candidate_tokens.extend_from_slice(&tokens[get_idx..]);
    if let Some(mut gain_effects) = parse_gain_ability_sentence(&gain_candidate_tokens)? {
        let mut patched = false;
        for effect in &mut gain_effects {
            match effect {
                EffectAst::PumpAll { filter, .. }
                | EffectAst::GrantAbilitiesAll { filter, .. }
                | EffectAst::GrantAbilitiesChoiceAll { filter, .. } => {
                    filter.chosen_creature_type = true;
                    patched = true;
                }
                _ => {}
            }
        }
        if patched {
            let mut effects = vec![EffectAst::ChooseCreatureType {
                player: PlayerAst::You,
                excluded_subtypes: vec![],
            }];
            effects.extend(gain_effects);
            return Ok(Some(effects));
        }
    }

    let mut filter_tokens = trimmed_subject_tokens;
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("all"))
    {
        filter_tokens.remove(0);
    }
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let modifier = tokens
        .get(get_idx + 1)
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        ))
    })?;
    let (power, toughness, duration, condition) =
        parse_get_modifier_values_with_tail(&tokens[get_idx + 1..], base_power, base_toughness)?;
    if condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gets duration in creature-type choice pump clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::ChooseCreatureType {
            player: PlayerAst::You,
            excluded_subtypes: vec![],
        },
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        },
    ]))
}

pub(crate) fn parse_sentence_put_sticker_on(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if !matches!(clause_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let Some(sticker_idx) = find_index(&clause_words, |word| {
        matches!(*word, "sticker" | "stickers")
    }) else {
        return Ok(None);
    };
    let Some(on_idx) = rfind_index(&clause_words, |word| *word == "on") else {
        return Ok(None);
    };
    if on_idx <= sticker_idx || on_idx + 1 >= clause_words.len() {
        return Ok(None);
    }

    let action = if contains_word_sequence(&clause_words[..=sticker_idx], &["name", "sticker"]) {
        crate::events::KeywordActionKind::NameSticker
    } else if contains_word_sequence(&clause_words[..=sticker_idx], &["art", "sticker"]) {
        crate::events::KeywordActionKind::ArtSticker
    } else if contains_word_sequence(&clause_words[..=sticker_idx], &["ability", "sticker"]) {
        crate::events::KeywordActionKind::AbilitySticker
    } else {
        crate::events::KeywordActionKind::Sticker
    };

    let Some(target_start) = token_index_for_word_index(tokens, on_idx + 1) else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[target_start..]);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let target_words = crate::cards::builders::compiler::token_word_refs(&target_tokens);
    if target_words
        .first()
        .is_some_and(|word| matches!(*word, "target" | "it" | "them" | "that" | "those" | "this"))
    {
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(Some(vec![EffectAst::PutSticker { target, action }]));
    }

    let mut filter = parse_object_filter(&target_tokens, false)?;
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
    Ok(Some(vec![EffectAst::PutSticker {
        target: TargetAst::Object(filter, None, None),
        action,
    }]))
}

pub(crate) fn parse_sentence_return_targets_of_creature_type_of_choice(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    if !grammar::contains_word(&tokens[to_idx + 1..], "hand")
        && !grammar::contains_word(&tokens[to_idx + 1..], "hands")
    {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    let inline_creature_choice = find_creature_type_choice_phrase(&target_tokens);
    let referenced_type_choice = if inline_creature_choice.is_none() {
        find_type_choice_phrase(&target_tokens)
    } else {
        None
    };
    if inline_creature_choice.is_none() && referenced_type_choice.is_none() {
        return Ok(None);
    }

    let (filter, needs_inline_choice_effect) =
        if let Some((choice_idx, consumed)) = inline_creature_choice {
            let mut base_filter_tokens = target_tokens[..choice_idx].to_vec();
            base_filter_tokens.extend_from_slice(&target_tokens[choice_idx + consumed..]);
            let base_filter_tokens = trim_commas(&base_filter_tokens).to_vec();
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return target before chosen-type qualifier (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
            let mut filter = parse_object_filter(&base_filter_tokens, false)?;
            filter.chosen_creature_type = true;
            (filter, true)
        } else {
            let (choice_idx, consumed) = referenced_type_choice.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "type-choice return target must mention the chosen type (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                ))
            })?;
            let mut start_idx = choice_idx;
            let mut excluded = false;
            if choice_idx >= 2
                && target_tokens[choice_idx - 2].is_word("that")
                && (target_tokens[choice_idx - 1].is_word("arent")
                    || target_tokens[choice_idx - 1].is_word("aren't"))
            {
                start_idx = choice_idx - 2;
                excluded = true;
            } else if choice_idx >= 3
                && target_tokens[choice_idx - 3].is_word("that")
                && target_tokens[choice_idx - 2].is_word("are")
                && target_tokens[choice_idx - 1].is_word("not")
            {
                start_idx = choice_idx - 3;
                excluded = true;
            } else if choice_idx >= 2
                && target_tokens[choice_idx - 2].is_word("that")
                && target_tokens[choice_idx - 1].is_word("are")
            {
                start_idx = choice_idx - 2;
            }

            let mut base_filter_tokens = target_tokens[..start_idx].to_vec();
            base_filter_tokens.extend_from_slice(&target_tokens[choice_idx + consumed..]);
            let base_filter_tokens = trim_commas(&base_filter_tokens).to_vec();
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return target before chosen-type qualifier (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }

            let mut filter = parse_object_filter(&base_filter_tokens, false)?;
            if excluded {
                filter.excluded_chosen_creature_type = true;
            } else {
                filter.chosen_creature_type = true;
            }
            (filter, false)
        };

    let mut effects = Vec::new();
    if needs_inline_choice_effect {
        effects.push(EffectAst::ChooseCreatureType {
            player: PlayerAst::You,
            excluded_subtypes: vec![],
        });
    }
    effects.push(EffectAst::ReturnAllToHand { filter });

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_any_prefix(tokens, CHOOSE_ALL_OR_PUT_ALL_PREFIXES).is_none() {
        return Ok(None);
    }
    let starts_choose_all = grammar::words_match_any_prefix(tokens, CHOOSE_ALL_PREFIXES).is_some();
    if !((grammar::contains_word(tokens, "battlefield")
        || grammar::contains_word(tokens, "command"))
        && grammar::contains_word(tokens, "graveyard")
        && grammar::contains_word(tokens, "hand"))
    {
        return Ok(None);
    }

    let Some(from_idx) = find_index(&clause_words, |word| *word == "from") else {
        return Ok(None);
    };
    let zone_pair = if contains_word_window(
        &clause_words[from_idx..],
        &[
            "from",
            "the",
            "battlefield",
            "and",
            "from",
            "your",
            "graveyard",
        ],
    ) {
        [Zone::Battlefield, Zone::Graveyard]
    } else if contains_word_window(
        &clause_words[from_idx..],
        &[
            "from",
            "the",
            "command",
            "zone",
            "and",
            "from",
            "your",
            "graveyard",
        ],
    ) {
        [Zone::Command, Zone::Graveyard]
    } else {
        return Ok(None);
    };
    if from_idx <= 2 {
        return Ok(None);
    }

    let Some(from_token_idx) = token_index_for_word_index(tokens, from_idx) else {
        return Ok(None);
    };

    let filter_tokens = trim_commas(&tokens[2..from_token_idx]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if starts_choose_all {
        let Some(put_idx) = find_index(&clause_words, |word| *word == "put") else {
            return Ok(None);
        };
        let Some(put_token_idx) = token_index_for_word_index(tokens, put_idx) else {
            return Ok(None);
        };
        if grammar::words_match_prefix(
            &tokens[put_token_idx..],
            &["put", "them", "into", "your", "hand"],
        )
        .is_none()
            && grammar::words_match_prefix(
                &tokens[put_token_idx..],
                &["put", "them", "in", "your", "hand"],
            )
            .is_none()
        {
            return Ok(None);
        }
    } else if grammar::words_match_suffix(tokens, &["into", "your", "hand"]).is_none()
        && grammar::words_match_suffix(tokens, &["in", "your", "hand"]).is_none()
    {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    base_filter.controller = None;

    let mut battlefield_filter = base_filter.clone();
    battlefield_filter.zone = Some(zone_pair[0]);

    let mut graveyard_filter = base_filter;
    graveyard_filter.zone = Some(zone_pair[1]);

    Ok(Some(vec![
        EffectAst::ReturnAllToHand {
            filter: battlefield_filter,
        },
        EffectAst::ReturnAllToHand {
            filter: graveyard_filter,
        },
    ]))
}

pub(crate) fn return_segment_mentions_zone(tokens: &[OwnedLexToken]) -> bool {
    grammar::contains_word(tokens, "graveyard")
        || grammar::contains_word(tokens, "graveyards")
        || grammar::contains_word(tokens, "battlefield")
        || grammar::contains_word(tokens, "hand")
        || grammar::contains_word(tokens, "hands")
        || grammar::contains_word(tokens, "library")
        || grammar::contains_word(tokens, "libraries")
        || grammar::contains_word(tokens, "exile")
}

pub(crate) fn parse_sentence_return_multiple_targets(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let dest_tokens = &tokens[to_idx + 1..];
    let is_hand =
        grammar::contains_word(dest_tokens, "hand") || grammar::contains_word(dest_tokens, "hands");
    let is_battlefield = grammar::contains_word(dest_tokens, "battlefield");
    let tapped = grammar::contains_word(dest_tokens, "tapped");
    if !is_hand && !is_battlefield {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    let has_multi_separator = target_tokens.iter().any(|token| {
        token.is_word("and") || token.is_comma() || token.is_word("or") || token.is_word("and/or")
    });
    if !has_multi_separator {
        return Ok(None);
    }

    let mut segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    for and_segment in split_lexed_slices_on_and(&target_tokens) {
        for comma_segment in split_lexed_slices_on_comma(and_segment) {
            let trimmed = trim_commas(&comma_segment);
            if !trimmed.is_empty() {
                let trimmed_words = crate::cards::builders::compiler::token_word_refs(&trimmed);
                let starts_new_target = trimmed_words.first().is_some_and(|word| {
                    matches!(
                        *word,
                        "target"
                            | "up"
                            | "another"
                            | "other"
                            | "this"
                            | "that"
                            | "it"
                            | "them"
                            | "all"
                            | "each"
                    )
                });
                let mentions_target = grammar::contains_word(&trimmed, "target");
                let starts_like_zone_suffix = trimmed_words
                    .first()
                    .is_some_and(|word| matches!(*word, "from" | "to" | "in" | "on" | "under"));
                if !segments.is_empty()
                    && !starts_new_target
                    && !mentions_target
                    && !starts_like_zone_suffix
                {
                    let last = segments.last_mut().expect("segments is non-empty");
                    last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                    last.extend(trimmed.to_vec());
                } else {
                    segments.push(trimmed.to_vec());
                }
            }
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(|segment| segment.first())
        .and_then(OwnedLexToken::as_word)
        .filter(|word| matches!(*word, "all" | "each"))
        .map(str::to_string);

    let shared_suffix = segments
        .last()
        .and_then(|segment| {
            find_index(segment, |token| token.is_word("from")).map(|idx| segment[idx..].to_vec())
        })
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        if !return_segment_mentions_zone(&segment) && !shared_suffix.is_empty() {
            segment.extend(shared_suffix.clone());
        }
        if let Some(quantifier) = shared_quantifier.as_deref() {
            let segment_words = crate::cards::builders::compiler::token_word_refs(&segment);
            let has_explicit_quantifier =
                matches!(segment_words.first().copied(), Some("all" | "each"));
            let starts_like_target_reference = matches!(
                segment_words.first().copied(),
                Some("target" | "up" | "this" | "that" | "it" | "them" | "another")
            );
            if !has_explicit_quantifier
                && !starts_like_target_reference
                && !grammar::contains_word(&segment, "target")
            {
                segment.insert(
                    0,
                    OwnedLexToken::word(quantifier.to_string(), TextSpan::synthetic()),
                );
            }
        }
        let segment_words = crate::cards::builders::compiler::token_word_refs(&segment);
        if matches!(segment_words.first().copied(), Some("all" | "each")) {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
            let filter = parse_object_filter(&segment[1..], false)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnAllToBattlefield { filter, tapped });
            } else {
                effects.push(EffectAst::ReturnAllToHand { filter });
            }
        } else {
            let target = parse_target_phrase(&segment)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnToBattlefield {
                    target,
                    tapped,
                    transformed: false,
                    converted: false,
                    controller: ReturnControllerAst::Preserve,
                });
            } else {
                effects.push(EffectAst::ReturnToHand {
                    target,
                    random: false,
                });
            }
        }
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_for_each_of_target_objects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if grammar::strip_lexed_prefix_phrase(tokens, &["for", "each"]).is_none()
        && !tokens.first().is_some_and(|t| t.is_word("each"))
    {
        return Ok(None);
    }

    let Some((subject_slice, effect_slice)) =
        grammar::split_lexed_once_on_delimiter(tokens, super::super::lexer::TokenKind::Comma)
    else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(subject_slice);
    let Some((mut filter, count)) = parse_for_each_targeted_object_subject(&subject_tokens)? else {
        return Ok(None);
    };
    if filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.tagged_constraints.is_empty()
    {
        // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects
        // compilation for plain "target permanent(s)" clauses.
        filter.controller = Some(PlayerFilter::Any);
    }

    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let effect_tokens = trim_commas(effect_slice);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after for-each target subject (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut per_target_effects = parse_effect_chain(&effect_tokens)?;
    for effect in &mut per_target_effects {
        bind_implicit_player_context(effect, PlayerAst::You);
    }
    if per_target_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "for-each target follow-up produced no effects (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player: PlayerAst::Implicit,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: per_target_effects,
        },
    ]))
}

pub(crate) fn parse_distribute_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.first().copied() != Some("distribute") {
        return Ok(None);
    }

    let (count, used) = parse_number(&tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed counter amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let rest = &tokens[1 + used..];
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let among_idx = find_index(rest, |token| token.is_word("among")).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed target clause after 'among' (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let target_tokens = trim_commas(&rest[among_idx + 1..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(&target_tokens)?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_phrase = &target_tokens[used_count..];
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target = parse_target_phrase(target_phrase)?;

    Ok(Some(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target,
        target_count: Some(target_count),
        distributed: true,
    }))
}

pub(crate) fn parse_sentence_distribute_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let (head_tokens, tail_tokens) = if let Some((head, tail)) =
        split_lexed_once_on_comma_then(tokens).or_else(|| {
            grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
        }) {
        (head.to_vec(), trim_commas(tail))
    } else {
        (tokens.to_vec(), Vec::new())
    };

    let Some(primary) = parse_distribute_counters_sentence(&head_tokens)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if !tail_tokens.is_empty() {
        effects.extend(parse_effect_chain(&tail_tokens)?);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_take_extra_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_take_extra_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_earthbend(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(earthbend) = parse_earthbend_sentence(tokens)? else {
        return Ok(None);
    };

    // Support chained text like "earthbend 8, then untap that land."
    let Some((_, used)) = parse_number(&tokens[1..]) else {
        return Ok(Some(vec![earthbend]));
    };
    let mut tail = trim_commas(&tokens[1 + used..]).to_vec();
    while tail.first().is_some_and(|token| token.is_word("then")) {
        tail.remove(0);
    }
    if tail.is_empty() {
        return Ok(Some(vec![earthbend]));
    }

    let mut effects = vec![earthbend];
    effects.extend(parse_effect_chain(&tail)?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_transform_with_followup(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = tokens.first() else {
        return Ok(None);
    };
    let is_transform = first.is_word("transform");
    let is_convert = first.is_word("convert");
    if !is_transform && !is_convert {
        return Ok(None);
    }

    let (head_tokens, tail_tokens) = if let Some((head, tail)) =
        split_lexed_once_on_comma_then(tokens).or_else(|| {
            super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
                super::super::grammar::primitives::kw("then").void()
            })
        }) {
        (head.to_vec(), trim_commas(tail))
    } else {
        (tokens.to_vec(), Vec::new())
    };

    let target_tokens = trim_commas(&head_tokens[1..]);
    let transform = if is_transform {
        parse_transform(&target_tokens)?
    } else {
        parse_convert(&target_tokens)?
    };
    if tail_tokens.is_empty() {
        return Ok(Some(vec![transform]));
    }

    let mut effects = vec![transform];
    effects.extend(parse_effect_chain(&tail_tokens)?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_enchant(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_enchant_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_cant_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence(tokens)
}

pub(crate) fn parse_sentence_prevent_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_prevent_damage_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_gain_ability_to_source(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_gain_ability_to_source_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_gain_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_ability_sentence(tokens)
}

pub(crate) fn parse_sentence_you_and_each_opponent_voted_with_you(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_each_opponent_voted_with_you_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_life_equal_to_power(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_power_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_x_plus_life(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_x_plus_life_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_exiled_this_way(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_exiled_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_put_into_graveyard_this_way(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_put_into_graveyard_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_put_permanent_cards_exiled_with_source_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_destroyed_this_way(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_destroyed_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_then_return_same_object(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_then_return_same_object_sentence(tokens)
}

pub(crate) fn parse_sentence_search_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_search_library_sentence(tokens)
}

pub(crate) fn parse_sentence_shuffle_graveyard_into_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shuffle_graveyard_into_library_sentence(tokens)
}

pub(crate) fn parse_sentence_shuffle_object_into_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shuffle_object_into_library_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_hand_and_graveyard_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_hand_and_graveyard_bundle_sentence(tokens)
}

pub(crate) fn parse_sentence_target_player_exiles_creature_and_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_target_player_exiles_creature_and_graveyard_sentence(tokens)
}

pub(crate) fn parse_sentence_play_from_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_play_from_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_look_at_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_hand_sentence(tokens)
}

pub(crate) fn parse_sentence_look_at_top_then_exile_one(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_top_then_exile_one_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_life_equal_to_age(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_age_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_opponent_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_opponent_doesnt(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_player_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_player_doesnt(tokens)?.map(|effect| vec![effect]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DelayedNextStepKind {
    Upkeep,
    DrawStep,
}

fn delayed_next_step_marker(
    tokens: &[OwnedLexToken],
) -> Option<(usize, usize, DelayedNextStepKind, PlayerAst)> {
    let word_storage = SentencePrimitiveNormalizedWords::new(tokens);
    let words = word_storage.to_word_refs();
    let patterns: &[(&[&str], DelayedNextStepKind, PlayerAst)] = &[
        (
            &["at", "the", "beginning", "of", "your", "next", "upkeep"],
            DelayedNextStepKind::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::You,
        ),
        (
            &["at", "the", "beginning", "of", "their", "next", "upkeep"],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::That,
        ),
    ];

    for (pattern, step, player) in patterns {
        if let Some(start) = find_word_sequence_start(&words, pattern) {
            return Some((start, start + pattern.len(), *step, *player));
        }
    }

    None
}

