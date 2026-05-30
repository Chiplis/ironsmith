use super::*;

pub(crate) fn parse_sentence_destroy_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if clause
        .strip_prefix(&["destroy", "all", "creatures"])
        .is_none()
    {
        return Ok(None);
    }
    if find_creature_type_choice_phrase(clause).is_none() {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_destroy_all(ObjectFilter::creature().of_chosen_creature_type()),
    ]))
}

pub(crate) fn parse_sentence_pump_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(get_idx) = clause.find_token_word_any(&["get", "gets"]) else {
        return Ok(None);
    };
    if get_idx == 0 {
        return Ok(None);
    }

    let subject_clause = clause.before(get_idx).trimmed();
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(subject_clause) else {
        return Ok(None);
    };
    if !subject_clause
        .from(choice_idx + consumed)
        .trimmed()
        .is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            clause.text()
        )));
    }
    let trimmed_subject_clause =
        subject_clause.without_token_range_trimmed_clause(choice_idx, consumed);
    if trimmed_subject_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_clause = trimmed_subject_clause.clone();
    gain_candidate_clause.append_clause(clause.from(get_idx));
    if let Some(mut gain_effects) = parse_gain_ability_sentence(gain_candidate_clause.tokens())? {
        let mut patched = false;
        for effect in &mut gain_effects {
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::PumpAll { filter, .. }
                        | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. }
                        | SubjectVerbActionAst::GrantAbilitiesChoiceAll { filter, .. },
                    ..
                }) => {
                    filter.chosen_creature_type = true;
                    patched = true;
                }
                _ => {}
            }
        }
        if patched {
            let mut effects = vec![EffectAst::subject_verb_choose_creature_type(
                PlayerAst::You,
                vec![],
            )];
            effects.extend(gain_effects);
            return Ok(Some(effects));
        }
    }

    let mut filter_clause = trimmed_subject_clause;
    filter_clause.remove_leading_word("all");
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }

    let modifier = clause
        .token(get_idx + 1)
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                clause.text()
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            clause.text()
        ))
    })?;
    let (power, toughness, duration, condition) = parse_get_modifier_values_with_tail(
        clause.from(get_idx + 1).tokens(),
        base_power,
        base_toughness,
    )?;
    if condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gets duration in creature-type choice pump clause (clause: '{}')",
            clause.text()
        )));
    }

    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_pump_all(filter, power, toughness, duration),
    ]))
}

pub(crate) fn parse_sentence_must_attack_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::effect::Until;

    const MUST_ATTACK_SUFFIXES: &[&[&str]] = &[
        &["attack", "this", "turn", "if", "able"],
        &["attacks", "this", "turn", "if", "able"],
    ];
    let Some((_suffix, subject_clause)) = clause.strip_any_suffix(MUST_ATTACK_SUFFIXES) else {
        return Ok(None);
    };
    let subject_clause = subject_clause.trimmed();
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(subject_clause) else {
        return Ok(None);
    };
    if !subject_clause
        .from(choice_idx + consumed)
        .trimmed()
        .is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }
    let mut filter_clause = subject_clause.without_token_range_trimmed_clause(choice_idx, consumed);
    filter_clause.remove_leading_word("all");
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice attack subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }
    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_grant_abilities_all(
            filter,
            vec![crate::runtime_backend::GrantedAbilityAst::MustAttack],
            Until::EndOfTurn,
        ),
    ]))
}

pub(crate) fn parse_sentence_put_sticker_on(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = clause.word_refs();
    if !matches!(clause_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let Some(sticker_idx) = clause.find_word_any(&["sticker", "stickers"]) else {
        return Ok(None);
    };
    let Some(on_idx) = clause.rfind_word("on") else {
        return Ok(None);
    };
    if on_idx <= sticker_idx || on_idx + 1 >= clause_words.len() {
        return Ok(None);
    }

    let Some(sticker_head) = clause.before_words(sticker_idx + 1) else {
        return Ok(None);
    };
    let action = if sticker_head.contains_phrase(&["name", "sticker"]) {
        crate::events::KeywordActionKind::NameSticker
    } else if sticker_head.contains_phrase(&["art", "sticker"]) {
        crate::events::KeywordActionKind::ArtSticker
    } else if sticker_head.contains_phrase(&["ability", "sticker"]) {
        crate::events::KeywordActionKind::AbilitySticker
    } else if sticker_head.contains_phrase(&["power", "and", "toughness", "sticker"]) {
        crate::events::KeywordActionKind::PowerToughnessSticker
    } else {
        crate::events::KeywordActionKind::Sticker
    };

    let Some(target_clause) = clause
        .from_word(on_idx + 1)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if target_clause.is_empty() {
        return Ok(None);
    }

    let target_words = target_clause.word_refs();
    if target_words
        .first()
        .is_some_and(|word| matches!(*word, "target" | "it" | "them" | "that" | "those" | "this"))
    {
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![EffectAst::subject_verb_put_sticker(
            target, action,
        )]));
    }

    let mut filter = parse_object_filter(target_clause.tokens(), false)?;
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
    Ok(Some(vec![EffectAst::subject_verb_put_sticker(
        TargetAst::Object(filter, None, None),
        action,
    )]))
}

pub(crate) fn parse_sentence_return_targets_of_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause.first_is_word("return") {
        return Ok(None);
    }
    let Some((target_clause, destination_clause)) = clause.rsplit_once_on_word("to") else {
        return Ok(None);
    };
    if target_clause.len() <= 1 {
        return Ok(None);
    }

    if !destination_clause.contains_any_word(&["hand", "hands"]) {
        return Ok(None);
    }

    let target_clause = target_clause.from(1).trimmed();
    let inline_creature_choice = find_creature_type_choice_phrase(target_clause);
    let referenced_type_choice = if inline_creature_choice.is_none() {
        find_type_choice_phrase(target_clause)
    } else {
        None
    };
    if inline_creature_choice.is_none() && referenced_type_choice.is_none() {
        return Ok(None);
    }

    let (filter, needs_inline_choice_effect) =
        if let Some((choice_idx, consumed)) = inline_creature_choice {
            let base_filter_clause =
                target_clause.without_token_range_trimmed_clause(choice_idx, consumed);
            if base_filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return target before chosen-type qualifier (clause: '{}')",
                    clause.text()
                )));
            }
            let mut filter = parse_object_filter(base_filter_clause.tokens(), false)?;
            filter.chosen_creature_type = true;
            (filter, true)
        } else {
            let (choice_idx, consumed) = referenced_type_choice.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "type-choice return target must mention the chosen type (clause: '{}')",
                    clause.text()
                ))
            })?;
            let mut start_idx = choice_idx;
            let mut excluded = false;
            if choice_idx >= 2
                && target_clause
                    .token(choice_idx - 2)
                    .is_some_and(|token| token.is_word("that"))
                && target_clause
                    .token(choice_idx - 1)
                    .is_some_and(|token| token.is_word("arent") || token.is_word("aren't"))
            {
                start_idx = choice_idx - 2;
                excluded = true;
            } else if choice_idx >= 3
                && target_clause
                    .token(choice_idx - 3)
                    .is_some_and(|token| token.is_word("that"))
                && target_clause
                    .token(choice_idx - 2)
                    .is_some_and(|token| token.is_word("are"))
                && target_clause
                    .token(choice_idx - 1)
                    .is_some_and(|token| token.is_word("not"))
            {
                start_idx = choice_idx - 3;
                excluded = true;
            } else if choice_idx >= 2
                && target_clause
                    .token(choice_idx - 2)
                    .is_some_and(|token| token.is_word("that"))
                && target_clause
                    .token(choice_idx - 1)
                    .is_some_and(|token| token.is_word("are"))
            {
                start_idx = choice_idx - 2;
            }

            let base_filter_clause = target_clause.without_token_ranges_trimmed_clause(&[
                (start_idx, choice_idx - start_idx),
                (choice_idx, consumed),
            ]);
            if base_filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return target before chosen-type qualifier (clause: '{}')",
                    clause.text()
                )));
            }

            let mut filter = parse_object_filter(base_filter_clause.tokens(), false)?;
            if excluded {
                filter.excluded_chosen_creature_type = true;
            } else {
                filter.chosen_creature_type = true;
            }
            (filter, false)
        };

    // Check whether the target tokens (before the creature-type choice phrase)
    // mention "target". If so, we need to parse a proper TargetAst (which
    // captures targeting semantics and count such as X) rather than using a
    // mass-return-all filter.
    let has_target = target_clause.contains_word("target");

    let mut effects = Vec::new();
    if needs_inline_choice_effect {
        effects.push(EffectAst::subject_verb_choose_creature_type(
            PlayerAst::You,
            vec![],
        ));
    }

    if has_target {
        // Rebuild the base tokens (stripping the creature-type-of-choice phrase)
        // so that parse_target_phrase can extract count + "target" + filter.
        let base_target_clause = {
            if let Some((choice_idx, consumed)) = inline_creature_choice {
                target_clause.without_token_range_trimmed_clause(choice_idx, consumed)
            } else {
                let (choice_idx, consumed) = referenced_type_choice.unwrap();
                let mut start_idx = choice_idx;
                if choice_idx >= 2
                    && target_clause
                        .token(choice_idx - 2)
                        .is_some_and(|token| token.is_word("that"))
                    && target_clause.token(choice_idx - 1).is_some_and(|token| {
                        token.is_word("arent") || token.is_word("aren't") || token.is_word("are")
                    })
                {
                    start_idx = choice_idx - 2;
                } else if choice_idx >= 3
                    && target_clause
                        .token(choice_idx - 3)
                        .is_some_and(|token| token.is_word("that"))
                    && target_clause
                        .token(choice_idx - 2)
                        .is_some_and(|token| token.is_word("are"))
                    && target_clause
                        .token(choice_idx - 1)
                        .is_some_and(|token| token.is_word("not"))
                {
                    start_idx = choice_idx - 3;
                }
                target_clause.without_token_ranges_trimmed_clause(&[
                    (start_idx, choice_idx - start_idx),
                    (choice_idx, consumed),
                ])
            }
        };
        let mut target = parse_target_phrase(base_target_clause.tokens())?;
        // Recursively patch `chosen_creature_type` / `excluded_chosen_creature_type`
        // on the ObjectFilter buried inside the TargetAst (may be wrapped in WithCount).
        fn patch_chosen_type(t: &mut TargetAst, chosen: bool, excluded: bool) {
            match t {
                TargetAst::Object(f, _, _) => {
                    f.chosen_creature_type |= chosen;
                    f.excluded_chosen_creature_type |= excluded;
                }
                TargetAst::WithCount(inner, _) => patch_chosen_type(inner, chosen, excluded),
                _ => {}
            }
        }
        patch_chosen_type(
            &mut target,
            filter.chosen_creature_type,
            filter.excluded_chosen_creature_type,
        );
        effects.push(EffectAst::subject_verb_return_to_hand(target, false));
    } else {
        effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_text = clause.text();
    if !clause.starts_with_any(CHOOSE_ALL_OR_PUT_ALL_PREFIXES) {
        return Ok(None);
    }
    let starts_choose_all = clause.starts_with_any(CHOOSE_ALL_PREFIXES);
    if !((clause.contains_word("battlefield") || clause.contains_word("command"))
        && clause.contains_all_words(&["graveyard", "hand"]))
    {
        return Ok(None);
    }

    let Some(from_idx) = clause.find_word("from") else {
        return Ok(None);
    };
    if from_idx <= 2 {
        return Ok(None);
    }
    let Some(zone_clause) = clause.from_word(from_idx) else {
        return Ok(None);
    };
    let zone_pair = if zone_clause.contains_phrase(&[
        "from",
        "the",
        "battlefield",
        "and",
        "from",
        "your",
        "graveyard",
    ]) {
        [Zone::Battlefield, Zone::Graveyard]
    } else if zone_clause.contains_phrase(&[
        "from",
        "the",
        "command",
        "zone",
        "and",
        "from",
        "your",
        "graveyard",
    ]) {
        [Zone::Command, Zone::Graveyard]
    } else {
        return Ok(None);
    };

    let Some(filter_clause) = clause
        .after_words(2)
        .and_then(|tail| tail.before_word(from_idx - 2))
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_text
        )));
    }

    if starts_choose_all {
        let Some(put_idx) = clause.find_word("put") else {
            return Ok(None);
        };
        let Some(put_clause) = clause.from_word(put_idx) else {
            return Ok(None);
        };
        if !put_clause.starts_with_any(&[
            &["put", "them", "into", "your", "hand"],
            &["put", "them", "in", "your", "hand"],
        ]) {
            return Ok(None);
        }
    } else if clause
        .strip_any_suffix(&[&["into", "your", "hand"], &["in", "your", "hand"]])
        .is_none()
    {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(filter_clause.tokens(), false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_text
        ))
    })?;
    base_filter.controller = None;

    let mut battlefield_filter = base_filter.clone();
    battlefield_filter.zone = Some(zone_pair[0]);

    let mut graveyard_filter = base_filter;
    graveyard_filter.zone = Some(zone_pair[1]);

    Ok(Some(vec![
        EffectAst::subject_verb_return_all_to_hand(battlefield_filter),
        EffectAst::subject_verb_return_all_to_hand(graveyard_filter),
    ]))
}

pub(crate) fn return_segment_mentions_zone(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    clause.contains_word("graveyard")
        || clause.contains_word("graveyards")
        || clause.contains_word("battlefield")
        || clause.contains_word("hand")
        || clause.contains_word("hands")
        || clause.contains_word("library")
        || clause.contains_word("libraries")
        || clause.contains_word("exile")
}

pub(crate) fn parse_sentence_return_multiple_targets(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause.first_is_word("return") {
        return Ok(None);
    }
    let Some((target_clause, dest_clause)) = clause.rsplit_once_on_word("to") else {
        return Ok(None);
    };
    if target_clause.len() <= 1 {
        return Ok(None);
    }

    let is_hand = dest_clause.contains_any_word(&["hand", "hands"]);
    let is_battlefield = dest_clause.contains_word("battlefield");
    let tapped = dest_clause.contains_word("tapped");
    if !is_hand && !is_battlefield {
        return Ok(None);
    }

    let targets_clause = target_clause.from(1).trimmed();
    let has_multi_separator = targets_clause.contains_comma_or_any_word(&["and", "or", "and/or"]);
    if !has_multi_separator {
        return Ok(None);
    }

    let mut segments: Vec<SubjectVerbPrimitiveOwnedClause> = Vec::new();
    for segment_clause in targets_clause.trimmed_and_comma_segments() {
        let trimmed_words = segment_clause.word_refs();
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
        let mentions_target = segment_clause.contains_word("target");
        let starts_like_zone_suffix = trimmed_words
            .first()
            .is_some_and(|word| matches!(*word, "from" | "to" | "in" | "on" | "under"));
        if !segments.is_empty()
            && !starts_new_target
            && !mentions_target
            && !starts_like_zone_suffix
        {
            let last = segments.last_mut().expect("segments is non-empty");
            last.append_comma_then(segment_clause);
        } else {
            segments.push(SubjectVerbPrimitiveOwnedClause::from_clause(segment_clause));
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(SubjectVerbPrimitiveOwnedClause::first_word)
        .filter(|word| matches!(*word, "all" | "each"))
        .map(str::to_string);

    let shared_suffix = segments
        .last()
        .and_then(|segment| {
            segment
                .find_token_word("from")
                .map(|idx| segment.from_tokens(idx).to_vec())
        })
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        if !return_segment_mentions_zone(segment.as_clause()) && !shared_suffix.is_empty() {
            segment.extend_from_slice(&shared_suffix);
        }
        if let Some(quantifier) = shared_quantifier.as_deref() {
            let segment_words = segment.word_refs();
            let has_explicit_quantifier =
                matches!(segment_words.first().copied(), Some("all" | "each"));
            let starts_like_target_reference = matches!(
                segment_words.first().copied(),
                Some("target" | "up" | "this" | "that" | "it" | "them" | "another")
            );
            if !has_explicit_quantifier
                && !starts_like_target_reference
                && !segment.contains_word("target")
            {
                segment.insert_leading_word(quantifier);
            }
        }
        let segment_words = segment.word_refs();
        if matches!(segment_words.first().copied(), Some("all" | "each")) {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    clause.text()
                )));
            }
            let filter = parse_object_filter(segment.from_tokens(1), false)?;
            if is_battlefield {
                effects.push(EffectAst::subject_verb_return_all_to_battlefield(
                    filter,
                    tapped,
                    ReturnControllerAst::Owner,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
            }
        } else {
            let target = parse_target_phrase(segment.tokens())?;
            if is_battlefield {
                effects.push(EffectAst::subject_verb_return_to_battlefield(
                    target,
                    tapped,
                    false,
                    false,
                    ReturnControllerAst::Preserve,
                    None,
                    false,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_to_hand(target, false));
            }
        }
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_for_each_of_target_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if clause.strip_prefix(&["for", "each"]).is_none() && clause.first_word() != Some("each") {
        return Ok(None);
    }

    let Some((subject_clause, effect_clause)) = clause.split_once_on_comma() else {
        return Ok(None);
    };

    let subject_clause = subject_clause.trimmed();
    let Some((mut filter, count)) =
        parse_for_each_targeted_object_subject(subject_clause.tokens())?
    else {
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

    let effect_clause = effect_clause.trimmed();
    if effect_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after for-each target subject (clause: '{}')",
            clause.text()
        )));
    }
    let mut per_target_effects = parse_effect_chain(effect_clause.tokens())?;
    for effect in &mut per_target_effects {
        bind_implicit_player_context(effect, PlayerAst::You);
    }
    if per_target_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "for-each target follow-up produced no effects (clause: '{}')",
            clause.text()
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
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    if !clause.first_is_word("distribute") {
        return Ok(None);
    }

    let amount_clause = clause.from(1);
    let (count, used) = parse_number(amount_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed counter amount (clause: '{}')",
            clause.text()
        ))
    })?;
    let rest_clause = clause.from(1 + used);
    let counter_type = parse_counter_type_from_tokens(rest_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause.text()
        ))
    })?;
    let Some((_before_among, target_clause)) = rest_clause.split_once_on_word("among") else {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target clause after 'among' (clause: '{}')",
            clause.text()
        )));
    };
    let target_clause = target_clause.trimmed();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause.text()
        )));
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(target_clause.tokens())?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause.text()
            ))
        })?;
    let target_phrase = target_clause.from(used_count).trimmed();
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause.text()
        )));
    }
    let target = parse_target_phrase(target_phrase.tokens())?;

    Ok(Some(EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        target,
        Some(target_count),
        true,
    )))
}

pub(crate) fn parse_sentence_distribute_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let Some(primary) = parse_distribute_counters_sentence(head_clause)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
        effects.extend(parse_effect_chain(tail_clause.tokens())?);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_transform_with_followup(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = clause.token(0) else {
        return Ok(None);
    };
    let is_transform = first.is_word("transform");
    let is_convert = first.is_word("convert");
    if !is_transform && !is_convert {
        return Ok(None);
    }

    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let target_clause = head_clause.from(1).trimmed();
    let transform = if is_transform {
        parse_transform(target_clause.tokens())?
    } else {
        parse_convert(target_clause.tokens())?
    };
    let Some(tail_clause) = tail_clause else {
        return Ok(Some(vec![transform]));
    };
    if tail_clause.is_empty() {
        return Ok(Some(vec![transform]));
    }

    let mut effects = vec![transform];
    effects.extend(parse_effect_chain(tail_clause.tokens())?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_cant_effect(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_cant_effect_sentence)
}

pub(crate) fn parse_sentence_gain_x_plus_life(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_x_plus_life_sentence)
}

pub(crate) fn parse_sentence_for_each_exiled_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_for_each_exiled_this_way_sentence)
}

pub(crate) fn parse_sentence_for_each_put_into_graveyard_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_for_each_put_into_graveyard_this_way_sentence)
}

pub(crate) fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_each_player_put_permanent_cards_exiled_with_source_sentence)
}

pub(crate) fn parse_sentence_for_each_destroyed_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_for_each_destroyed_this_way_sentence)
}

pub(crate) fn parse_sentence_search_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_search_library_sentence)
}

pub(crate) fn parse_sentence_shuffle_graveyard_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_graveyard_into_library_sentence)
}

pub(crate) fn parse_sentence_shuffle_object_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_object_into_library_sentence)
}

pub(crate) fn parse_sentence_exile_hand_and_graveyard_bundle(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_exile_hand_and_graveyard_bundle_sentence)
}

pub(crate) fn parse_sentence_target_player_exiles_creature_and_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_target_player_exiles_creature_and_graveyard_sentence)
}

pub(crate) fn parse_sentence_look_at_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_hand_sentence)
}

pub(crate) fn parse_sentence_look_at_top_then_exile_one(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_top_then_exile_one_sentence)
}

pub(crate) fn parse_sentence_gain_life_equal_to_age(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_life_equal_to_age_sentence)
}

pub(crate) fn parse_sentence_for_each_player_doesnt(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_one_with_lexed(parse_for_each_player_doesnt)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DelayedNextStepKind {
    Upkeep,
    DrawStep,
}

pub(super) fn delayed_next_step_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize, DelayedNextStepKind, PlayerAst)> {
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
        if let Some(start) = clause.find_phrase_start(pattern) {
            return Some((start, start + pattern.len(), *step, *player));
        }
    }

    None
}
