pub(crate) fn parse_sentence_sacrifice_at_end_of_combat(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    // "sacrifice <object> at [the] end of combat"
    let Some(object_tokens) = grammar::strip_lexed_prefix_phrase(tokens, &["sacrifice"]) else {
        return Ok(None);
    };
    let Some((object_tokens, _timing)) =
        grammar::split_lexed_once_on_separator(object_tokens, || {
            winnow::combinator::alt((
                grammar::phrase(&["at", "end", "of", "combat"]),
                grammar::phrase(&["at", "the", "end", "of", "combat"]),
            ))
        })
    else {
        return Ok(None);
    };

    let object_tokens = trim_commas(object_tokens);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in end-of-combat clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let object_words = crate::cards::builders::compiler::token_word_refs(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["that", "token"]
            | ["this", "token"]
            | ["that", "permanent"]
            | ["this", "permanent"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
            target: None,
        }],
    }]))
}

pub(crate) fn parse_sentence_each_player_choose_and_sacrifice_rest(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_each_player_choose_and_sacrifice_rest(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_exile_instead_of_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_exile_instead_of_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_monstrosity(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_monstrosity_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_counter_removed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_counter_removed_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_counter_kind_put_or_remove(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    // "for each kind of counter on <target>, put another counter of that kind on it or remove one from it"
    let Some(after_prefix) =
        grammar::strip_lexed_prefix_phrase(tokens, &["for", "each", "kind", "of", "counter", "on"])
    else {
        return Ok(None);
    };
    let Some((target_tokens, tail_tokens)) =
        grammar::split_lexed_once_on_delimiter(after_prefix, super::super::lexer::TokenKind::Comma)
    else {
        return Ok(None);
    };

    let target_tokens = trim_commas(target_tokens);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(&target_tokens)?;

    if !grammar::contains_phrase(
        tail_tokens,
        &[
            "put", "another", "counter", "of", "that", "kind", "on", "it", "or", "remove", "one",
            "from",
        ],
    ) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::ForEachCounterKindPutOrRemove {
        target,
    }]))
}

pub(crate) fn parse_put_counter_ladder_segments(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_comma(tokens);
    if segments.len() != 3 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = trim_commas(segment).to_vec();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }
        if clause.is_empty() {
            return Ok(None);
        }

        let Some(on_idx) = find_index(&clause, |token| token.is_word("on")) else {
            return Ok(None);
        };
        let descriptor = trim_commas(&clause[..on_idx]);
        let target_tokens = trim_commas(&clause[on_idx + 1..]);
        if descriptor.is_empty() || target_tokens.is_empty() {
            return Ok(None);
        }

        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        let target = parse_target_phrase(&target_tokens)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target,
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_put_counter_sequence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if !tokens.first().is_some_and(|token| token.is_word("put")) {
        return Ok(None);
    }
    if !tokens
        .iter()
        .any(|token| token.is_word("counter") || token.is_word("counters"))
    {
        return Ok(None);
    }

    let (head_tokens, tail_tokens) = if let Some((head, tail)) =
        split_lexed_once_on_comma_then(tokens).or_else(|| {
            grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
        }) {
        (head.to_vec(), trim_commas(tail))
    } else {
        (tokens.to_vec(), Vec::new())
    };
    if !tail_tokens.is_empty() {
        let mut effects = parse_effect_chain(&head_tokens)?;
        if effects.is_empty() {
            return Ok(None);
        }
        effects.extend(parse_effect_chain(&tail_tokens)?);
        return Ok(Some(effects));
    }

    if let Some(effects) = parse_put_counter_ladder_segments(tokens)? {
        return Ok(Some(effects));
    }

    if let Some(on_idx) = find_index(tokens, |token| token.is_word("on")) {
        let descriptor_tokens = trim_commas(&tokens[1..on_idx]);
        let target_tokens = trim_commas(&tokens[on_idx + 1..]);
        if !descriptor_tokens.is_empty() && !target_tokens.is_empty() {
            let mut descriptors: Vec<Vec<OwnedLexToken>> = Vec::new();
            let comma_segments = split_lexed_slices_on_comma(&descriptor_tokens);
            if comma_segments.len() >= 2 {
                for segment in comma_segments {
                    let mut clause = trim_commas(segment);
                    if clause.first().is_some_and(|token| token.is_word("and")) {
                        clause.remove(0);
                    }
                    if clause.is_empty() {
                        descriptors.clear();
                        break;
                    }
                    descriptors.push(clause);
                }
            } else if let Some(and_idx) =
                find_index(&descriptor_tokens, |token| token.is_word("and"))
            {
                let first = trim_commas(&descriptor_tokens[..and_idx]);
                let second = trim_commas(&descriptor_tokens[and_idx + 1..]);
                if !first.is_empty() && !second.is_empty() {
                    descriptors.push(first);
                    descriptors.push(second);
                }
            }

            if descriptors.len() >= 2 {
                let target = parse_target_phrase(&target_tokens)?;
                let mut effects = Vec::new();
                for descriptor in descriptors {
                    let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
                    effects.push(EffectAst::PutCounters {
                        counter_type,
                        count: Value::Fixed(count as i32),
                        target: target.clone(),
                        target_count: None,
                        distributed: false,
                    });
                }
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... counter on X and it gains ... until end of turn."
    if let Some(and_idx) = find_window_by(tokens, 2, |window| {
        window[0].is_word("and") && window[1].is_word("it")
    }) {
        let first_clause = trim_commas(&tokens[1..and_idx]);
        let second_clause = trim_commas(&tokens[and_idx + 1..]);
        if !first_clause.is_empty()
            && !second_clause.is_empty()
            && second_clause.iter().any(|token| {
                token.is_word("gain")
                    || token.is_word("gains")
                    || token.is_word("has")
                    || token.is_word("have")
            })
            && let Ok(first) = parse_put_counters(&first_clause)
            && let Some(mut gain_effects) = parse_gain_ability_sentence(&second_clause)?
        {
            let source_target = match &first {
                EffectAst::PutCounters { target, .. } => Some(target.clone()),
                EffectAst::Conditional { if_true, .. }
                    if if_true.len() == 1
                        && matches!(if_true.first(), Some(EffectAst::PutCounters { .. })) =>
                {
                    if let Some(EffectAst::PutCounters { target, .. }) = if_true.first() {
                        Some(target.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(source_target) = source_target {
                for effect in &mut gain_effects {
                    match effect {
                        EffectAst::Pump { target, .. }
                        | EffectAst::GrantAbilitiesToTarget { target, .. }
                        | EffectAst::GrantToTarget { target, .. }
                        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                            if let TargetAst::Tagged(tag, _) = target
                                && tag.as_str() == IT_TAG
                            {
                                *target = source_target.clone();
                            }
                        }
                        _ => {}
                    }
                }

                let mut effects = vec![first];
                effects.append(&mut gain_effects);
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... and ... counter on ..." without comma separation.
    if let Some(and_idx) = find_index(tokens, |token| token.is_word("and")) {
        let first_clause = trim_commas(&tokens[1..and_idx]);
        let second_clause = trim_commas(&tokens[and_idx + 1..]);
        if !first_clause.is_empty() && !second_clause.is_empty() {
            if let (Ok(first), Ok(second)) = (
                parse_put_counters(&first_clause),
                parse_put_counters(&second_clause),
            ) {
                return Ok(Some(vec![first, second]));
            }
        }
    }

    let segments = split_lexed_slices_on_comma(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = segment.to_vec();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }

        if clause.is_empty() {
            return Ok(None);
        }

        if !grammar::contains_word(&clause, "counter")
            && !grammar::contains_word(&clause, "counters")
        {
            return Ok(None);
        }

        let Ok(effect) = parse_put_counters(&clause) else {
            return Ok(None);
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

pub(crate) fn is_pump_like_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::Pump { .. }
            | EffectAst::PumpByLastEffect { .. }
            | EffectAst::SetBasePowerToughness { .. }
            | EffectAst::SetBasePower { .. }
    )
}

pub(crate) fn parse_gets_then_fights_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let body_tokens = grammar::strip_lexed_prefix_phrase(tokens, &["then"]).unwrap_or(tokens);
    if body_tokens.is_empty() {
        return Ok(None);
    }

    // Split on "fight"/"fights"
    let fight_split =
        grammar::split_lexed_once_on_separator(body_tokens, || grammar::kw("fight").void())
            .or_else(|| {
                grammar::split_lexed_once_on_separator(body_tokens, || grammar::kw("fights").void())
            });
    let Some((left_slice, right_slice)) = fight_split else {
        return Ok(None);
    };

    let mut left_tokens = trim_commas(left_slice).to_vec();
    while left_tokens.last().is_some_and(|token| token.is_word("and")) {
        left_tokens.pop();
    }
    let left_tokens = trim_commas(&left_tokens);
    let right_tokens = trim_commas(right_slice);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return Ok(None);
    }

    // Split left side on "get"/"gets" to extract subject
    let get_split =
        grammar::split_lexed_once_on_separator(&left_tokens, || grammar::kw("get").void()).or_else(
            || grammar::split_lexed_once_on_separator(&left_tokens, || grammar::kw("gets").void()),
        );
    let Some((subject_slice, _modifier_slice)) = get_split else {
        return Ok(None);
    };

    let pump_effect = parse_effect_clause(&left_tokens)?;
    if !is_pump_like_effect(&pump_effect) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(subject_slice);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let creature1 = parse_target_phrase(&subject_tokens)?;
    let creature2 = parse_target_phrase(&right_tokens)?;
    if matches!(
        creature1,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) || matches!(
        creature2,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "fight target must be a creature (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![
        pump_effect,
        EffectAst::Fight {
            creature1,
            creature2,
        },
    ]))
}

pub(crate) fn parse_sentence_gets_then_fights(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gets_then_fights_sentence(tokens)
}

pub(crate) fn parse_return_with_counters_on_it_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter(|word| !is_article(word))
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }

    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before destination (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let destination_tokens = trim_commas(&tokens[to_idx + 1..]);
    if destination_tokens.is_empty() {
        return Ok(None);
    }
    if !grammar::contains_word(&destination_tokens, "battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = find_token_word(&destination_tokens, "with") else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_tokens.len() {
        return Ok(None);
    }

    let base_destination_word_storage =
        crate::cards::builders::compiler::token_word_refs(&destination_tokens[..with_idx]);
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    let Some(battlefield_idx) = find_index(&base_destination_words, |word| *word == "battlefield")
    else {
        return Ok(None);
    };
    let tapped = slice_contains(&base_destination_words, &"tapped");
    let destination_tail: Vec<&str> = base_destination_words[battlefield_idx + 1..]
        .iter()
        .copied()
        .filter(|word| *word != "tapped")
        .collect();
    let battlefield_controller = if destination_tail.is_empty()
        || destination_tail == ["under", "its", "control"]
        || destination_tail == ["under", "their", "control"]
    {
        ReturnControllerAst::Preserve
    } else if destination_tail == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail == ["under", "its", "owner", "control"]
        || destination_tail == ["under", "their", "owner", "control"]
        || destination_tail == ["under", "his", "owner", "control"]
        || destination_tail == ["under", "her", "owner", "control"]
        || destination_tail == ["under", "that", "player", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    let counter_clause_tokens = trim_commas(&destination_tokens[with_idx + 1..]);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::compiler::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    let timing_words =
        if on_target_words.starts_with(&["it"]) || on_target_words.starts_with(&["them"]) {
            &on_target_words[1..]
        } else {
            return Ok(None);
        };
    let delayed_timing = if timing_words.is_empty() {
        None
    } else {
        super::zone_handlers::parse_delayed_return_timing_words(timing_words)
    };
    if !timing_words.is_empty() && delayed_timing.is_none() {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let mut descriptors = Vec::new();
    for descriptor in split_lexed_slices_on_and(&descriptor_tokens) {
        let descriptor = trim_commas(&descriptor);
        if descriptor.is_empty() {
            continue;
        }
        descriptors.push(descriptor);
    }
    if descriptors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let mut effects = vec![EffectAst::ReturnToBattlefield {
        target: parse_target_phrase(&target_tokens)?,
        tapped,
        transformed: false,
        converted: false,
        controller: battlefield_controller,
    }];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: tagged_target.clone(),
            target_count: None,
            distributed: false,
        });
    }

    let wrapped = if let Some(timing) = delayed_timing {
        match timing {
            super::zone_handlers::DelayedReturnTimingAst::NextEndStep(player) => {
                vec![EffectAst::DelayedUntilNextEndStep { player, effects }]
            }
            super::zone_handlers::DelayedReturnTimingAst::NextUpkeep(player) => {
                vec![EffectAst::DelayedUntilNextUpkeep { player, effects }]
            }
            super::zone_handlers::DelayedReturnTimingAst::EndOfCombat => {
                vec![EffectAst::DelayedUntilEndOfCombat { effects }]
            }
        }
    } else {
        effects
    };

    Ok(Some(wrapped))
}

pub(crate) fn parse_put_onto_battlefield_with_counters_on_it_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter(|word| !is_article(word))
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    if !tokens
        .first()
        .is_some_and(|token| token.is_word("put") || token.is_word("puts"))
    {
        return Ok(None);
    }

    let Some(onto_idx) = find_index(tokens, |token| token.is_word("onto")) else {
        return Ok(None);
    };
    if onto_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..onto_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing put target before destination (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let destination_tokens = trim_commas(&tokens[onto_idx + 1..]);
    if destination_tokens.is_empty() {
        return Ok(None);
    }
    if !grammar::contains_word(&destination_tokens, "battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = find_token_word(&destination_tokens, "with") else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_tokens.len() {
        return Ok(None);
    }

    let base_destination_word_storage =
        crate::cards::builders::compiler::token_word_refs(&destination_tokens[..with_idx]);
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    if base_destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }

    let destination_tail = &base_destination_words[1..];
    let supported_control_tail = destination_tail.is_empty()
        || destination_tail == ["under", "your", "control"]
        || destination_tail == ["under", "its", "owner", "control"]
        || destination_tail == ["under", "their", "owner", "control"]
        || destination_tail == ["under", "his", "owner", "control"]
        || destination_tail == ["under", "her", "owner", "control"]
        || destination_tail == ["under", "that", "player", "control"];
    if !supported_control_tail {
        return Ok(None);
    }
    let battlefield_controller = if destination_tail == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail == ["under", "its", "owner", "control"]
        || destination_tail == ["under", "their", "owner", "control"]
        || destination_tail == ["under", "his", "owner", "control"]
        || destination_tail == ["under", "her", "owner", "control"]
        || destination_tail == ["under", "that", "player", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };

    let counter_clause_tokens = trim_commas(&destination_tokens[with_idx + 1..]);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::compiler::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() || !grammar::contains_word(&descriptor_tokens, "counter") {
        return Ok(None);
    }

    let mut descriptors = Vec::new();
    for descriptor in split_lexed_slices_on_and(&descriptor_tokens) {
        let descriptor = trim_commas(&descriptor);
        if descriptor.is_empty() {
            continue;
        }
        descriptors.push(descriptor);
    }
    if descriptors.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::MoveToZone {
        target: parse_target_phrase(&target_tokens)?,
        zone: Zone::Battlefield,
        to_top: false,
        battlefield_controller,
        battlefield_tapped: false,
        attached_to: None,
    }];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: tagged_target.clone(),
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_with_counters_on_it(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(tokens)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_put_onto_battlefield_with_counters_on_it_sentence(tokens)
}

pub(crate) fn replace_target_subtype(target: &mut TargetAst, subtype: Subtype) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.subtypes = vec![subtype];
            true
        }
        TargetAst::WithCount(inner, _) => replace_target_subtype(inner, subtype),
        _ => false,
    }
}

pub(crate) fn clone_return_effect_with_subtype(
    base: &EffectAst,
    subtype: Subtype,
) -> Option<EffectAst> {
    match base {
        EffectAst::ReturnToHand { target, random } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(EffectAst::ReturnToHand {
                target: cloned_target,
                random: *random,
            })
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            transformed,
            converted,
            controller,
        } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(
                EffectAst::ReturnToBattlefield {
                    target: cloned_target,
                    tapped: *tapped,
                    transformed: *transformed,
                    converted: *converted,
                    controller: *controller,
                },
            )
        }
        EffectAst::ReturnAllToHand { filter } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToHand {
                filter: cloned_filter,
            })
        }
        EffectAst::ReturnAllToBattlefield { filter, tapped } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToBattlefield {
                filter: cloned_filter,
                tapped: *tapped,
            })
        }
        _ => None,
    }
}

pub(crate) fn parse_draw_then_connive_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !tail_tokens
        .iter()
        .any(|token| token.is_word("connive") || token.is_word("connives"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(&tail_tokens)? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_draw_then_connive(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(tokens)
}

pub(crate) fn parse_if_enters_with_additional_counter_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use super::super::lexer::TokenKind;

    // "if <predicate>, it enters with <counter descriptor> on it"
    let Some(after_if) = grammar::strip_lexed_prefix_phrase(tokens, &["if"]) else {
        return Ok(None);
    };
    let Some((predicate_slice, followup_slice)) =
        grammar::split_lexed_once_on_delimiter(after_if, TokenKind::Comma)
    else {
        return Ok(None);
    };

    let predicate_tokens = trim_commas(predicate_slice);
    let predicate_words: Vec<&str> =
        crate::cards::builders::compiler::token_word_refs(&predicate_tokens)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
    let predicate_is_supported = predicate_words.as_slice()
        == ["creature", "enters", "this", "way"]
        || predicate_words.as_slice() == ["it", "enters", "as", "creature"];
    if !predicate_is_supported {
        return Ok(None);
    }

    let followup_tokens = trim_commas(followup_slice);
    let Some(counter_clause_slice) =
        grammar::strip_lexed_prefix_phrase(&followup_tokens, &["it", "enters", "with"])
    else {
        return Ok(None);
    };

    let counter_clause_tokens = trim_commas(counter_clause_slice);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::compiler::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() || !grammar::contains_word(&descriptor_tokens, "additional") {
        return Ok(None);
    }

    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;
    let put_counter = EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    };
    let apply_only_if_creature = EffectAst::Conditional {
        predicate: PredicateAst::ItMatches(ObjectFilter::creature()),
        if_true: vec![put_counter],
        if_false: Vec::new(),
    };

    Ok(Some(vec![EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![apply_only_if_creature],
    }]))
}

pub(crate) fn parse_each_player_return_with_additional_counter_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    let inner_start_word_idx = if let Some((prefix, _)) =
        grammar::words_match_any_prefix(tokens, FOR_EACH_PLAYER_PREFIXES)
    {
        prefix.len()
    } else {
        return Ok(None);
    };

    let Some(inner_start_token_idx) = token_index_for_word_index(tokens, inner_start_word_idx)
    else {
        return Ok(None);
    };
    let inner_tokens = trim_commas(&tokens[inner_start_token_idx..]);
    if inner_tokens.is_empty() {
        return Ok(None);
    }
    if !inner_tokens
        .first()
        .is_some_and(|token| token.is_word("return") || token.is_word("returns"))
    {
        return Ok(None);
    }

    let Some(with_idx) = rfind_index(&inner_tokens, |token| token.is_word("with")) else {
        return Ok(None);
    };
    if with_idx + 1 >= inner_tokens.len() {
        return Ok(None);
    }

    let return_clause_tokens = trim_commas(&inner_tokens[..with_idx]);
    if return_clause_tokens.is_empty() {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(&inner_tokens[with_idx + 1..]);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::compiler::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() || !grammar::contains_word(&descriptor_tokens, "additional") {
        return Ok(None);
    }

    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;
    let mut per_player_effects = parse_effect_chain_inner(&return_clause_tokens)?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ReturnToBattlefield { .. } | EffectAst::ReturnAllToBattlefield { .. }
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}

