use super::*;

fn subject_verb_put_counters_target(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::PutCounters { target, .. } => Some(target.clone()),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn parse_sentence_sacrifice_at_end_of_combat(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "sacrifice <object> at [the] end of combat"
    const END_OF_COMBAT_TIMING: &[&[&str]] = &[
        &["at", "end", "of", "combat"],
        &["at", "the", "end", "of", "combat"],
    ];
    let Some(object_clause) = clause.strip_prefix_clause(&["sacrifice"]) else {
        return Ok(None);
    };
    let Some((_timing, object_clause, _tail)) =
        object_clause.split_once_on_any_phrase(END_OF_COMBAT_TIMING)
    else {
        return Ok(None);
    };

    let object_clause = object_clause.trimmed();
    if object_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in end-of-combat clause (clause: '{}')",
            clause.text()
        )));
    }

    let object_words = object_clause.word_refs();
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
        parse_object_filter(object_clause.tokens(), false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            filter,
            1,
            None,
        )],
    }]))
}

pub(crate) fn parse_sentence_for_each_counter_kind_put_or_remove(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "for each kind of counter on <target>, put another counter of that kind on it or remove one from it"
    let Some(after_prefix) =
        clause.strip_prefix_clause(&["for", "each", "kind", "of", "counter", "on"])
    else {
        return Ok(None);
    };
    let Some((target_clause, tail_clause)) = after_prefix.split_once_on_comma() else {
        return Ok(None);
    };

    let target_clause = target_clause.trimmed();
    if target_clause.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(target_clause.tokens())?;

    if !tail_clause.contains_phrase(&[
        "put", "another", "counter", "of", "that", "kind", "on", "it", "or", "remove", "one",
        "from",
    ]) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_for_each_counter_kind_put_or_remove(target),
    ]))
}

pub(crate) fn parse_put_counter_ladder_segments(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_comma_segments();
    if segments.len() != 3 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let segment_clause = if idx == 0 {
            if !segment.first_is_word("put") {
                return Ok(None);
            }
            segment.from(1).trimmed()
        } else if segment.first_is_word("and") {
            segment.from(1).trimmed()
        } else {
            segment.trimmed()
        };
        if segment_clause.is_empty() {
            return Ok(None);
        }

        let Some((descriptor_clause, target_clause)) = segment_clause.split_once_on_word("on")
        else {
            return Ok(None);
        };
        let descriptor_clause = descriptor_clause.trimmed();
        let target_clause = target_clause.trimmed();
        if descriptor_clause.is_empty() || target_clause.is_empty() {
            return Ok(None);
        }

        let (count, counter_type) = parse_counter_descriptor(descriptor_clause.tokens())?;
        let target = parse_target_phrase(target_clause.tokens())?;
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            target,
            None,
            false,
        ));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_put_counter_sequence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause.first_is_word("put") {
        return Ok(None);
    }
    if !clause.contains_any_word(&["counter", "counters"]) {
        return Ok(None);
    }

    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
        let mut effects = parse_effect_chain(head_clause.tokens())?;
        if effects.is_empty() {
            return Ok(None);
        }
        effects.extend(parse_effect_chain(tail_clause.tokens())?);
        return Ok(Some(effects));
    }

    if let Some(effects) = parse_put_counter_ladder_segments(clause)? {
        return Ok(Some(effects));
    }

    if let Some((descriptor_clause, target_clause)) = clause.split_once_on_word("on") {
        let descriptor_clause = descriptor_clause.from(1).trimmed();
        let target_clause = target_clause.trimmed();
        if !descriptor_clause.is_empty() && !target_clause.is_empty() {
            let mut descriptors: Vec<SubjectVerbPrimitiveOwnedClause> = Vec::new();
            let comma_segments = descriptor_clause.trimmed_comma_segments();
            if comma_segments.len() >= 2 {
                for segment in comma_segments {
                    let mut segment_clause = SubjectVerbPrimitiveOwnedClause::from_clause(segment);
                    segment_clause.remove_leading_word("and");
                    if segment_clause.is_empty() {
                        descriptors.clear();
                        break;
                    }
                    descriptors.push(segment_clause);
                }
            } else if let Some((first_clause, second_clause)) =
                descriptor_clause.split_once_on_word("and")
            {
                let first_clause = first_clause.trimmed();
                let second_clause = second_clause.trimmed();
                if !first_clause.is_empty() && !second_clause.is_empty() {
                    descriptors.push(SubjectVerbPrimitiveOwnedClause::from_clause(first_clause));
                    descriptors.push(SubjectVerbPrimitiveOwnedClause::from_clause(second_clause));
                }
            }

            if descriptors.len() >= 2 {
                let target = parse_target_phrase(target_clause.tokens())?;
                let mut effects = Vec::new();
                for descriptor in descriptors {
                    let (count, counter_type) = parse_counter_descriptor(descriptor.tokens())?;
                    effects.push(EffectAst::subject_verb_put_counters(
                        counter_type,
                        Value::Fixed(count as i32),
                        target.clone(),
                        None,
                        false,
                    ));
                }
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... counter on X and it gains ... until end of turn."
    if let Some((first_clause, second_clause)) = clause.split_once_on_phrase(&["and", "it"]) {
        let first_clause = first_clause.from(1).trimmed();
        let second_clause = second_clause.trimmed();
        if !first_clause.is_empty()
            && !second_clause.is_empty()
            && second_clause.contains_any_word(&["gain", "gains", "has", "have"])
            && let Ok(first) = parse_put_counters(first_clause.tokens())
            && let Some(mut gain_effects) = parse_gain_ability_sentence(second_clause.tokens())?
        {
            let source_target = match &first {
                effect if subject_verb_put_counters_target(effect).is_some() => {
                    subject_verb_put_counters_target(effect)
                }
                EffectAst::Conditional { if_true, .. } if if_true.len() == 1 => {
                    if_true.first().and_then(subject_verb_put_counters_target)
                }
                _ => None,
            };

            if let Some(source_target) = source_target {
                for effect in &mut gain_effects {
                    match effect {
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action:
                                SubjectVerbActionAst::Pump { target, .. }
                                | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
                                | SubjectVerbActionAst::GrantToTarget { target, .. }
                                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. },
                            ..
                        }) => {
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
    if let Some((first_clause, second_clause)) = clause.split_once_on_word("and") {
        let first_clause = first_clause.from(1).trimmed();
        let second_clause = second_clause.trimmed();
        if !first_clause.is_empty() && !second_clause.is_empty() {
            if let (Ok(first), Ok(second)) = (
                parse_put_counters(first_clause.tokens()),
                parse_put_counters(second_clause.tokens()),
            ) {
                return Ok(Some(vec![first, second]));
            }
        }
    }

    let segments = clause.trimmed_comma_segments();
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let segment_clause = if idx == 0 {
            if !segment.first_is_word("put") {
                return Ok(None);
            }
            segment.from(1).trimmed()
        } else if segment.first_is_word("and") {
            segment.from(1).trimmed()
        } else {
            *segment
        };

        if segment_clause.is_empty() {
            return Ok(None);
        }

        if !segment_clause.contains_any_word(&["counter", "counters"]) {
            return Ok(None);
        }

        let Ok(effect) = parse_put_counters(segment_clause.tokens()) else {
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
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Pump { .. }
                | SubjectVerbActionAst::PumpByLastEffect { .. }
                | SubjectVerbActionAst::SetBasePowerToughness { .. }
                | SubjectVerbActionAst::SetBasePower { .. },
            ..
        })
    )
}

pub(crate) fn parse_gets_then_fights_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let body_clause = clause.strip_prefix_clause(&["then"]).unwrap_or(clause);
    if body_clause.is_empty() {
        return Ok(None);
    }

    // Split on "fight"/"fights"
    let Some((left_clause, right_clause)) =
        body_clause.split_once_on_word_any(&["fight", "fights"])
    else {
        return Ok(None);
    };

    let left_clause = left_clause.without_trailing_words_clause(&["and"]);
    let right_clause = right_clause.trimmed();
    if left_clause.is_empty() || right_clause.is_empty() {
        return Ok(None);
    }

    // Split left side on "get"/"gets" to extract subject
    let Some((subject_clause, _modifier_clause)) =
        left_clause.split_once_on_word_any(&["get", "gets"])
    else {
        return Ok(None);
    };

    let pump_effect = parse_effect_clause(left_clause.tokens())?;
    if !is_pump_like_effect(&pump_effect) {
        return Ok(None);
    }

    let subject_clause = subject_clause.trimmed();
    if subject_clause.is_empty() {
        return Ok(None);
    }
    let creature1 = parse_target_phrase(subject_clause.tokens())?;
    let creature2 = parse_target_phrase(right_clause.tokens())?;
    if matches!(
        creature1,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) || matches!(
        creature2,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "fight target must be a creature (clause: '{}')",
            clause.text()
        )));
    }

    Ok(Some(vec![
        pump_effect,
        EffectAst::subject_verb_fight(creature1, creature2),
    ]))
}

pub(crate) fn parse_sentence_gets_then_fights(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gets_then_fights_sentence(clause)
}

pub(crate) fn parse_return_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter(|word| !is_article(word))
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word: &&str| !word.is_empty())
            .collect()
    }

    if !clause.first_is_word("return") {
        return Ok(None);
    }

    let Some((target_clause, destination_clause)) = clause.rsplit_once_on_word("to") else {
        return Ok(None);
    };
    if target_clause.len() <= 1 {
        return Ok(None);
    }

    let target_clause = target_clause.from(1).trimmed();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before destination (clause: '{}')",
            clause.text()
        )));
    }

    let destination_clause = destination_clause.trimmed();
    if destination_clause.is_empty() {
        return Ok(None);
    }
    if !destination_clause.contains_word("battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = destination_clause.find_token_word("with") else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_clause.len() {
        return Ok(None);
    }

    let base_destination_word_storage = destination_clause.before(with_idx).word_refs();
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    let Some(battlefield_idx) = base_destination_words
        .iter()
        .position(|word| *word == "battlefield")
    else {
        return Ok(None);
    };
    let tapped = word_slice_contains_word(&base_destination_words, "tapped");
    let destination_tail: Vec<&str> = base_destination_words[battlefield_idx + 1..]
        .iter()
        .copied()
        .filter(|word| *word != "tapped")
        .collect();
    const PRESERVE_CONTROL_TAILS: &[&[&str]] =
        &[&["under", "its", "control"], &["under", "their", "control"]];
    const OWNER_CONTROL_TAILS: &[&[&str]] = &[
        &["under", "its", "owner", "control"],
        &["under", "their", "owner", "control"],
        &["under", "his", "owner", "control"],
        &["under", "her", "owner", "control"],
        &["under", "that", "player", "control"],
    ];
    let battlefield_controller = if destination_tail.is_empty()
        || crate::runtime_backend::lexer::word_slice_eq_any(
            &destination_tail,
            PRESERVE_CONTROL_TAILS,
        ) {
        ReturnControllerAst::Preserve
    } else if crate::runtime_backend::lexer::word_slice_eq(
        &destination_tail,
        &["under", "your", "control"],
    ) {
        ReturnControllerAst::You
    } else if crate::runtime_backend::lexer::word_slice_eq_any(
        &destination_tail,
        OWNER_CONTROL_TAILS,
    ) {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    let counter_clause = destination_clause.from(with_idx + 1).trimmed();
    let Some(on_idx) = counter_clause.rfind_token_word("on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause.len() {
        return Ok(None);
    }

    let on_target_words = counter_clause.from(on_idx + 1).word_refs();
    let timing_words = if word_slice_starts_with(&on_target_words, &["it"])
        || word_slice_starts_with(&on_target_words, &["them"])
    {
        &on_target_words[1..]
    } else {
        return Ok(None);
    };
    let delayed_timing = if timing_words.is_empty() {
        None
    } else {
        super::super::zone_handlers::parse_delayed_return_timing_words(timing_words)
    };
    if !timing_words.is_empty() && delayed_timing.is_none() {
        return Ok(None);
    }

    let descriptor_clause = counter_clause.before(on_idx).trimmed();
    if descriptor_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            clause.text()
        )));
    }

    let descriptors = descriptor_clause.trimmed_and_segments();
    if descriptors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            clause.text()
        )));
    }

    let mut effects = vec![EffectAst::subject_verb_return_to_battlefield(
        parse_target_phrase(target_clause.tokens())?,
        tapped,
        false,
        false,
        battlefield_controller,
        None,
    )];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(descriptor.tokens())?;
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            tagged_target.clone(),
            None,
            false,
        ));
    }

    let wrapped = if let Some(timing) = delayed_timing {
        match timing {
            super::super::zone_handlers::DelayedReturnTimingAst::NextEndStep(player) => {
                vec![EffectAst::DelayedUntilNextEndStep { player, effects }]
            }
            super::super::zone_handlers::DelayedReturnTimingAst::NextUpkeep(player) => {
                vec![EffectAst::DelayedUntilNextUpkeep { player, effects }]
            }
            super::super::zone_handlers::DelayedReturnTimingAst::EndOfCombat => {
                vec![EffectAst::DelayedUntilEndOfCombat { effects }]
            }
        }
    } else {
        effects
    };

    Ok(Some(wrapped))
}

pub(crate) fn parse_put_onto_battlefield_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter(|word| !is_article(word))
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word: &&str| !word.is_empty())
            .collect()
    }

    if !clause
        .token(0)
        .is_some_and(|token| token.is_word("put") || token.is_word("puts"))
    {
        return Ok(None);
    }

    let Some((target_clause, destination_clause)) = clause.split_once_on_word("onto") else {
        return Ok(None);
    };
    if target_clause.len() <= 1 {
        return Ok(None);
    }

    let target_clause = target_clause.from(1).trimmed();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing put target before destination (clause: '{}')",
            clause.text()
        )));
    }

    let destination_clause = destination_clause.trimmed();
    if destination_clause.is_empty() {
        return Ok(None);
    }
    if !destination_clause.contains_word("battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = destination_clause.find_token_word("with") else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_clause.len() {
        return Ok(None);
    }

    let base_destination_word_storage = destination_clause.before(with_idx).word_refs();
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    if base_destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }

    let destination_tail = &base_destination_words[1..];
    const OWNER_CONTROL_TAILS: &[&[&str]] = &[
        &["under", "its", "owner", "control"],
        &["under", "their", "owner", "control"],
        &["under", "his", "owner", "control"],
        &["under", "her", "owner", "control"],
        &["under", "that", "player", "control"],
    ];
    let supported_control_tail = destination_tail.is_empty()
        || crate::runtime_backend::lexer::word_slice_eq(
            destination_tail,
            &["under", "your", "control"],
        )
        || crate::runtime_backend::lexer::word_slice_eq_any(destination_tail, OWNER_CONTROL_TAILS);
    if !supported_control_tail {
        return Ok(None);
    }
    let battlefield_controller = if crate::runtime_backend::lexer::word_slice_eq(
        destination_tail,
        &["under", "your", "control"],
    ) {
        ReturnControllerAst::You
    } else if crate::runtime_backend::lexer::word_slice_eq_any(
        destination_tail,
        OWNER_CONTROL_TAILS,
    ) {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };

    let counter_clause = destination_clause.from(with_idx + 1).trimmed();
    let Some(on_idx) = counter_clause.rfind_token_word("on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause.len() {
        return Ok(None);
    }

    let on_target_words = counter_clause.from(on_idx + 1).word_refs();
    if !crate::runtime_backend::lexer::word_slice_eq_any(&on_target_words, &[&["it"], &["them"]]) {
        return Ok(None);
    }

    let descriptor_clause = counter_clause.before(on_idx).trimmed();
    if descriptor_clause.is_empty()
        || !descriptor_clause.contains_any_word(&["counter", "counters"])
    {
        return Ok(None);
    }

    let descriptors = descriptor_clause.trimmed_and_segments();
    if descriptors.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::subject_verb_move_to_zone(
        parse_target_phrase(target_clause.tokens())?,
        Zone::Battlefield,
        false,
        battlefield_controller,
        false,
        None,
    )];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(descriptor.tokens())?;
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            tagged_target.clone(),
            None,
            false,
        ));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(clause)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_put_onto_battlefield_with_counters_on_it_sentence(clause)
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
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ReturnToHand { target, random } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then_some(
                    EffectAst::subject_verb_return_to_hand(cloned_target, *random),
                )
            }
            SubjectVerbActionAst::ReturnAllToHand { filter } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(EffectAst::subject_verb_return_all_to_hand(cloned_filter))
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                target,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura,
            } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then(|| {
                    let mut effect = EffectAst::subject_verb_return_to_battlefield(
                        cloned_target,
                        *tapped,
                        *transformed,
                        *converted,
                        *controller,
                        count_value.clone(),
                    );
                    if let EffectAst::SubjectVerb(subject_verb) = &mut effect
                        && let SubjectVerbActionAst::ReturnToBattlefield { as_aura: dst, .. } =
                            &mut subject_verb.action
                    {
                        *dst = as_aura.clone();
                    }
                    effect
                })
            }
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                controller,
            } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(EffectAst::subject_verb_return_all_to_battlefield(
                    cloned_filter,
                    *tapped,
                    *controller,
                ))
            }
            _ => None,
        },
        _ => None,
    }
}
pub(crate) fn parse_draw_then_connive_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    if !tail_clause.contains_any_word(&["connive", "connives"]) {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(tail_clause.tokens())? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_draw_then_connive(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(clause)
}

fn parse_additional_counter_descriptor_on_target(
    counter_clause: SubjectVerbPrimitiveClause<'_>,
    accepted_targets: &[&[&str]],
) -> Result<Option<(u32, crate::object::CounterType)>, CardTextError> {
    let counter_clause = counter_clause.trimmed();
    let Some((descriptor_clause, on_target_clause)) =
        counter_clause.rsplit_once_on_word_trimmed("on")
    else {
        return Ok(None);
    };
    if descriptor_clause.is_empty()
        || !descriptor_clause.contains_word("additional")
        || !accepted_targets
            .iter()
            .any(|target_words| on_target_clause.word_refs() == *target_words)
    {
        return Ok(None);
    }

    parse_counter_descriptor(descriptor_clause.tokens()).map(Some)
}

pub(crate) fn parse_if_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "if <predicate>, it enters with <counter descriptor> on it"
    let Some(after_if) = clause.strip_prefix_clause(&["if"]) else {
        return Ok(None);
    };
    let Some((predicate_clause, followup_clause)) = after_if.split_once_on_comma() else {
        return Ok(None);
    };

    let predicate_words: Vec<&str> = predicate_clause
        .trimmed_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let predicate_is_supported = crate::runtime_backend::lexer::word_slice_eq_any(
        &predicate_words,
        &[
            &["creature", "enters", "this", "way"],
            &["it", "enters", "as", "creature"],
        ],
    );
    if !predicate_is_supported {
        return Ok(None);
    }

    let Some(counter_clause) = followup_clause
        .trimmed()
        .strip_prefix_clause(&["it", "enters", "with"])
    else {
        return Ok(None);
    };

    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"]])?
    else {
        return Ok(None);
    };
    let put_counter = EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    );
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

pub(crate) fn parse_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if clause.first_word() != Some("put") {
        return Ok(None);
    }
    if !clause.contains_all_words(&["onto", "battlefield"]) {
        return Ok(None);
    }

    let Some((move_clause, counter_clause)) = clause.rsplit_once_on_word_trimmed("with") else {
        return Ok(None);
    };
    if move_clause.is_empty() || counter_clause.is_empty() {
        return Ok(None);
    }

    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"], &["them"]])?
    else {
        return Ok(None);
    };
    let mut effects = parse_effect_chain_inner(move_clause.tokens())?;
    if effects.is_empty()
        || !effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::MoveToZone {
                        zone: Zone::Battlefield,
                        ..
                    } | SubjectVerbActionAst::ReturnToBattlefield { .. }
                        | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                    ..
                })
            )
        })
    {
        return Ok(None);
    }

    effects.push(EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    ));

    Ok(Some(effects))
}

pub(crate) fn parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause
        .token(0)
        .is_some_and(|token| token.is_word("sacrifice") || token.is_word("sacrifices"))
    {
        return Ok(None);
    }

    let Some((sacrifice_clause, put_clause)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };
    if sacrifice_clause.is_empty() || put_clause.is_empty() {
        return Ok(None);
    }

    let Some(mut put_effects) =
        parse_put_onto_battlefield_with_additional_counters_sentence(put_clause)?
    else {
        return Ok(None);
    };
    let mut effects = if sacrifice_clause.len() >= 2
        && sacrifice_clause.first_is_word("sacrifice")
        && sacrifice_clause
            .from(1)
            .tokens()
            .iter()
            .all(|token| token.as_word().is_some())
    {
        vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            ObjectFilter {
                source: true,
                ..Default::default()
            },
            1,
            None,
        )]
    } else {
        parse_effect_chain_inner(sacrifice_clause.tokens())?
    };
    if effects.is_empty() {
        return Ok(None);
    }
    effects.append(&mut put_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after_if) = clause.strip_prefix_clause(&["if"]) else {
        return Ok(None);
    };
    let Some((predicate_clause, effect_clause)) = after_if.split_once_on_comma() else {
        return Ok(None);
    };

    let predicate_clause = predicate_clause.trimmed();
    let effect_clause = effect_clause.trimmed();
    if predicate_clause.is_empty() || effect_clause.is_empty() {
        return Ok(None);
    }
    if !effect_clause.first_is_any_word(&["sacrifice", "sacrifices"]) {
        return Ok(None);
    }

    let Some(effects) =
        parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(effect_clause)?
    else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::Conditional {
        predicate: parse_predicate_lexed(predicate_clause.tokens())?,
        if_true: effects,
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_each_player_return_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((_prefix, inner_clause)) = clause.strip_any_prefix_clause(FOR_EACH_PLAYER_PREFIXES)
    else {
        return Ok(None);
    };
    let inner_clause = inner_clause.trimmed();
    if inner_clause.is_empty() {
        return Ok(None);
    }
    if !inner_clause.first_is_any_word(&["return", "returns"]) {
        return Ok(None);
    }

    let Some((return_clause, counter_clause)) = inner_clause.rsplit_once_on_word_trimmed("with")
    else {
        return Ok(None);
    };
    if return_clause.is_empty() {
        return Ok(None);
    }

    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"], &["them"]])?
    else {
        return Ok(None);
    };
    let mut per_player_effects = parse_effect_chain_inner(return_clause.tokens())?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    ));

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}
