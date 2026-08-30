use super::*;

pub fn parse_for_each_count_value_words(words: &[&str]) -> Option<(Value, usize)> {
    let head = parse_for_each_head(words)?;
    let idx = head.item_start;

    if let Some(value) = parse_mana_from_source_spent_count(words, idx) {
        return Some(value);
    }

    // Commander products use this count in several otherwise unrelated
    // consumers (spell copies, token creation, P/T changes, and cost
    // reductions).  Recover the history metric before the permissive object
    // filter fallback can reinterpret "commander" as a current object set.
    // The runtime value deliberately counts casts from the command zone, not
    // commanders that happen to exist in a zone now.
    let commander_count_end = value_boundary(&words[idx..]) + idx;
    if let Some(player) = super::super::value_helper_shapes::parse_commander_cast_count_player(
        &words[idx..commander_count_end],
    ) {
        let mut value = Value::CommanderCastCount(player);
        if crate::word_primitives::sequence_occurs(
            &words[idx..commander_count_end],
            &["a", "commander"],
        ) {
            value = value
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::IndefiniteCommanderReference);
        }
        return Some((value, commander_count_end));
    }

    if crate::word_primitives::first_is_any(&words[idx..], &["counter", "counters"])
        && crate::word_primitives::parse_sequence_prefix(
            &words[idx + 1..],
            &["removed", "this", "way"],
        )
    {
        return Some((
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed),
            )
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay),
            idx + 4,
        ));
    }
    let mut removed_counter_descriptor_start = idx;
    if words
        .get(removed_counter_descriptor_start)
        .is_some_and(|word| is_article(word))
        || permission_shapes::starts_at_words(words, removed_counter_descriptor_start, &["one"])
    {
        removed_counter_descriptor_start += 1;
    }
    if let Some(counter_idx) = first_counter_word(&words[removed_counter_descriptor_start..])
        .map(|relative_idx| removed_counter_descriptor_start + relative_idx)
        .filter(|counter_idx| counter_idx.saturating_sub(removed_counter_descriptor_start) <= 2)
        && crate::word_primitives::parse_sequence_prefix(
            &words[counter_idx + 1..],
            &["removed", "this", "way"],
        )
    {
        let counter_type = (counter_idx > removed_counter_descriptor_start)
            .then(|| {
                parse_counter_type_words(&words[removed_counter_descriptor_start..=counter_idx])
            })
            .flatten();
        return Some((
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed)
                .with_counter_type(counter_type),
            )
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay),
            counter_idx + 4,
        ));
    }

    if let Some(value) = super::super::value_expr::colored_mana_symbols_in_costs(words) {
        return Some(value);
    }

    let history_end = value_boundary(&words[idx..]) + idx;
    let history_tokens = synthetic_word_tokens(&words[..history_end]);
    if permission_shapes::find_words(&words[idx..history_end], &["this", "way"]).is_none()
        && let Some(value) =
            super::super::value_semantics::parse_turn_history_count_value(&history_tokens)
    {
        return Some((value, history_end));
    }

    let mut counter_descriptor_start = idx;
    if words
        .get(counter_descriptor_start)
        .is_some_and(|word| is_article(word))
        || permission_shapes::starts_at_words(words, counter_descriptor_start, &["one"])
    {
        counter_descriptor_start += 1;
    }
    if let Some(counter_idx) = first_counter_word(&words[counter_descriptor_start..])
        .map(|relative_idx| counter_descriptor_start + relative_idx)
        .filter(|counter_idx| counter_idx.saturating_sub(counter_descriptor_start) <= 2)
    {
        let parsed_counter_type = if counter_idx > counter_descriptor_start {
            parse_counter_type_words(&words[counter_descriptor_start..=counter_idx])
        } else {
            None
        };
        if let Some(counter_type) = parsed_counter_type
            && words
                .get(counter_idx + 1..counter_idx + 3)
                .is_some_and(|tail| exact_one_of(tail, &[&["you", "have"], &["you", "ve"]]))
        {
            return Some((
                Value::PlayerCounters(PlayerFilter::You, counter_type),
                counter_idx + 3,
            ));
        }
        if permission_shapes::starts_at_words(words, counter_idx + 1, &["on"]) {
            let reference_start = counter_idx + 2;
            let reference_end = value_boundary(&words[reference_start..]) + reference_start;
            let reference = &words[reference_start..reference_end];
            if is_source_counter_reference(reference) {
                let value = match parsed_counter_type {
                    Some(counter_type) => match this_source_surface_for_words(reference) {
                        Some(surface) => Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        ),
                        None => Value::CountersOnSource(counter_type),
                    },
                    None => Value::CountersOn(
                        Box::new(
                            this_source_surface_for_words(reference)
                                .map(source_choose_spec_for_surface)
                                .unwrap_or(ChooseSpec::Source),
                        ),
                        None,
                    ),
                };
                return Some((value, reference_end));
            }
            if let Some(surface) = source_reference_surface_for_words(reference) {
                let value = Value::CountersOn(
                    Box::new(source_choose_spec_for_surface(surface)),
                    parsed_counter_type,
                );
                return Some((value, reference_end));
            }
            if is_tagged_counter_reference(reference) {
                let value = Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                    )),
                    parsed_counter_type,
                );
                return Some((value, reference_end));
            }
            if let Ok(filter) = parse_object_filter_words(reference, false) {
                return Some((
                    Value::CountersOn(Box::new(ChooseSpec::All(filter)), parsed_counter_type),
                    reference_end,
                ));
            }
        }
    }

    let filter_end = value_boundary(&words[idx..]) + idx;
    let count_words = &words[idx..filter_end];
    let exact_event_basis = parse_exact_dynamic_count_basis(count_words, filter_end);
    let prior_result_marker =
        permission_shapes::find_words(&words[idx..filter_end], &["this", "way"]);
    // Resolve both structural facts together. An exact complete event phrase
    // such as `cards looked at while scrying this way` denotes the event's
    // numeric amount; only a remaining generic `this way` marker denotes
    // prior-result object memory.
    if let Some(exact) = exact_event_basis {
        return Some(exact);
    }
    if let Some(relative_this_way) = prior_result_marker {
        let this_way_start = idx + relative_this_way;
        let this_way_subject = &words[idx..this_way_start];
        let died_filter_words =
            [&["that", "died"][..], &["died"][..]]
                .into_iter()
                .find_map(|suffix| {
                    permission_shapes::suffix_words(this_way_subject, suffix).then(|| {
                        &this_way_subject[..this_way_subject.len().saturating_sub(suffix.len())]
                    })
                });
        if let Some(died_filter_words) = died_filter_words
            && !died_filter_words.is_empty()
            && let Some(filter) = parse_for_each_object_filter_words(died_filter_words, head.other)
        {
            return Some((
                Value::PendingPriorEffectMetric(
                    ironsmith_core::PriorEffectMetricQuery::new(
                        ironsmith_core::EffectMetricSource::AffectedObjects,
                        ironsmith_core::EffectMetric::Count,
                    )
                    .with_filter(filter)
                    .with_action(ironsmith_core::PriorEffectAction::Destroyed),
                )
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::DiedThisWay),
                filter_end,
            ));
        }
        if exact_one_of(
            this_way_subject,
            &[
                &["creature", "card", "put", "into", "your", "graveyard"],
                &["creature", "cards", "put", "into", "your", "graveyard"],
            ],
        ) {
            let mut filter = parse_object_filter_words(&["creature", "card"], false).ok()?;
            filter.set_explicit_card_noun(true);
            return Some((
                Value::PendingPriorEffectMetric(
                    ironsmith_core::PriorEffectMetricQuery::new(
                        ironsmith_core::EffectMetricSource::AffectedObjects,
                        ironsmith_core::EffectMetric::Count,
                    )
                    .with_filter(filter)
                    .with_action(ironsmith_core::PriorEffectAction::Milled),
                )
                .with_surface_hint(
                    ironsmith_core::ValueSurfaceHint::CardsPutIntoYourGraveyardThisWay,
                ),
                filter_end,
            ));
        }
        // A typed `... this way` object count binds to the exact producer's
        // captured result memory.  Keep the authored action on the query so
        // both runtime evaluation and compiled text distinguish, for example,
        // cards exiled by this instruction from every card currently in exile.
        if let Some((action, action_start)) =
            value_helper_shapes::parse_prior_effect_action(this_way_subject)
        {
            let mut filter_words = &this_way_subject[..action_start];
            let player = if permission_shapes::suffix_words(filter_words, &["they"])
                || permission_shapes::suffix_words(filter_words, &["their"])
            {
                filter_words = &filter_words[..filter_words.len() - 1];
                Some(PlayerFilter::IteratedPlayer)
            } else if permission_shapes::suffix_words(filter_words, &["you"])
                || permission_shapes::suffix_words(filter_words, &["your"])
            {
                filter_words = &filter_words[..filter_words.len() - 1];
                Some(PlayerFilter::You)
            } else if permission_shapes::suffix_words(filter_words, &["that", "player"]) {
                filter_words = &filter_words[..filter_words.len() - 2];
                Some(PlayerFilter::IteratedPlayer)
            } else {
                None
            };
            let coordinated_stack_filter = crate::word_primitives::parse_choice_sequence_complete(
                filter_words,
                &[&["spell", "spells"], &["and"], &["ability", "abilities"]],
            )
            .then(|| {
                let mut filter = ObjectFilter::spell_or_ability();
                filter.set_conjunctive_set_surface(true);
                filter
            });
            if !filter_words.is_empty()
                && let Some(mut filter) = coordinated_stack_filter
                    .or_else(|| parse_for_each_object_filter_words(filter_words, head.other))
            {
                let action_words = &this_way_subject[action_start..];
                if action == ironsmith_core::PriorEffectAction::Returned
                    && permission_shapes::suffix_words(
                        action_words,
                        &["returned", "to", "your", "hand"],
                    )
                {
                    filter.owner = Some(PlayerFilter::You);
                } else if action == ironsmith_core::PriorEffectAction::Returned
                    && permission_shapes::suffix_words(
                        action_words,
                        &["returned", "to", "their", "hand"],
                    )
                {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if filter_words
                    .iter()
                    .any(|word| matches!(*word, "card" | "cards"))
                {
                    filter.set_explicit_card_noun(true);
                }
                let mut query = ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::AffectedObjects,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_filter(filter)
                .with_action(action);
                if let Some(player) = player {
                    query = query.with_player(player);
                }
                let value = Value::PendingPriorEffectMetric(query);
                let value = match action {
                    ironsmith_core::PriorEffectAction::Discarded => value
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsDiscardedThisWay),
                    ironsmith_core::PriorEffectAction::Drawn => {
                        value.with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsDrawnThisWay)
                    }
                    ironsmith_core::PriorEffectAction::Revealed => value
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsRevealedThisWay),
                    _ => value,
                };
                return Some((value, filter_end));
            }
        }
        let has_explicit_card_noun = this_way_subject
            .iter()
            .any(|word| matches!(*word, "card" | "cards"));
        for candidate_end in (idx + 1..this_way_start).rev() {
            if let Some(mut filter) =
                parse_for_each_object_filter_words(&words[idx..candidate_end], head.other)
            {
                if has_explicit_card_noun {
                    filter.set_explicit_card_noun(true);
                }
                return Some((
                    Value::Count(filter.match_tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                        TaggedOpbjectRelation::IsTaggedObject,
                    )),
                    filter_end,
                ));
            }
        }
    }

    if let Some(value) =
        value_helper_shapes::parse_aggregate_scope_value_words(&words[idx..filter_end])
    {
        return Some((value, filter_end));
    }

    if let Some(player) = value_helper_shapes::parse_party_size_player(count_words) {
        return Some((Value::PartySize(player), filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["time", "it", "regenerated", "this", "turn"],
            &["times", "it", "regenerated", "this", "turn"],
        ],
    ) {
        return Some((Value::SourceRegeneratedThisTurnCount, filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["card", "youve", "drawn", "this", "turn"],
            &["cards", "youve", "drawn", "this", "turn"],
            &["card", "you've", "drawn", "this", "turn"],
            &["cards", "you've", "drawn", "this", "turn"],
            &["card", "you", "have", "drawn", "this", "turn"],
            &["cards", "you", "have", "drawn", "this", "turn"],
        ],
    ) {
        return Some((Value::MaxCardsDrawnThisTurn(PlayerFilter::You), filter_end));
    }
    if exact_one_of(
        count_words,
        &[
            &["card", "an", "opponent", "has", "drawn", "this", "turn"],
            &["cards", "an", "opponent", "has", "drawn", "this", "turn"],
            &["card", "opponents", "have", "drawn", "this", "turn"],
            &["cards", "opponents", "have", "drawn", "this", "turn"],
        ],
    ) {
        return Some((
            Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
            filter_end,
        ));
    }
    if is_kick_count(count_words) {
        return Some((Value::KickCount, filter_end));
    }
    if let Some(counter_idx) =
        first_counter_word(count_words).filter(|counter_idx| *counter_idx <= 2)
    {
        let counter_tokens = synthetic_word_tokens(count_words);
        if let Some(counter_type) = parse_counter_type_from_tokens(&counter_tokens) {
            if count_words
                .get(counter_idx + 1..counter_idx + 3)
                .is_some_and(|tail| exact_one_of(tail, &[&["you", "have"], &["you", "ve"]]))
            {
                return Some((
                    Value::PlayerCounters(PlayerFilter::You, counter_type),
                    filter_end,
                ));
            }
            if permission_shapes::starts_at_words(count_words, counter_idx + 1, &["on"]) {
                let reference = &count_words[counter_idx + 2..];
                if is_source_counter_reference(reference) {
                    if let Some(surface) = this_source_surface_for_words(reference) {
                        return Some((
                            Value::CountersOn(
                                Box::new(source_choose_spec_for_surface(surface)),
                                Some(counter_type),
                            ),
                            filter_end,
                        ));
                    }
                    return Some((Value::CountersOnSource(counter_type), filter_end));
                }
                if let Some(surface) = source_reference_surface_for_words(reference) {
                    return Some((
                        Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        ),
                        filter_end,
                    ));
                }
                if is_tagged_counter_reference(reference) {
                    return Some((
                        Value::CountersOn(
                            Box::new(ChooseSpec::Tagged(
                                crate::tag::CompilerReferenceTag::It.key(),
                            )),
                            Some(counter_type),
                        ),
                        filter_end,
                    ));
                }
                if let Ok(filter) = parse_object_filter_words(reference, false) {
                    return Some((
                        Value::CountersOn(Box::new(ChooseSpec::All(filter)), Some(counter_type)),
                        filter_end,
                    ));
                }
            }
            if permission_shapes::starts_at_words(count_words, counter_idx + 1, &["among"]) {
                let reference = &count_words[counter_idx + 2..];
                if let Ok(filter) = parse_object_filter_words(reference, false) {
                    return Some((
                        Value::CountersOn(Box::new(ChooseSpec::All(filter)), Some(counter_type))
                            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersAmong),
                        filter_end,
                    ));
                }
            }
        }
    }

    let filter = parse_for_each_object_filter_words(&words[idx..filter_end], head.other)?;
    Some((Value::Count(filter), filter_end))
}

pub(super) fn parse_exact_dynamic_count_basis(
    words: &[&str],
    consumed: usize,
) -> Option<(Value, usize)> {
    let value = if exact_one_of(
        words,
        &[
            &["card", "that", "player", "has", "drawn", "this", "turn"],
            &["cards", "that", "player", "has", "drawn", "this", "turn"],
        ],
    ) {
        Value::MaxCardsDrawnThisTurn(PlayerFilter::IteratedPlayer)
    } else if exact_one_of(
        words,
        &[&["opponent", "you", "have"], &["opponents", "you", "have"]],
    ) {
        Value::CountPlayers(PlayerFilter::Opponent)
    } else if exact_one_of(
        words,
        &[
            &[
                "modified",
                "creature",
                "you",
                "controlled",
                "as",
                "you",
                "cast",
                "this",
                "spell",
            ],
            &[
                "modified",
                "creatures",
                "you",
                "controlled",
                "as",
                "you",
                "cast",
                "this",
                "spell",
            ],
        ],
    ) {
        Value::Count(crate::target::ObjectFilter::tagged(
            crate::tag::CompilerReferenceTag::CastModifiedCreatures.key(),
        ))
    } else if exact_one_of(
        words,
        &[
            &["creature", "chosen", "before", "it"],
            &["creatures", "chosen", "before", "it"],
        ],
    ) {
        Value::Count(crate::target::ObjectFilter::tagged(
            crate::tag::CompilerReferenceTag::PreviousIteratedObjects.key(),
        ))
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CreaturesChosenBeforeIt)
    } else if exact_one_of(
        words,
        &[
            &["creature", "blocking", "it"],
            &["creatures", "blocking", "it"],
        ],
    ) {
        Value::EventValueOffset(
            ironsmith_core::EventValueSpec::BlockersBeyondFirst { multiplier: 1 },
            1,
        )
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CreaturesBlockingIt)
    } else if exact_one_of(
        words,
        &[
            &["creature", "blocking", "it", "beyond", "the", "first"],
            &["creatures", "blocking", "it", "beyond", "the", "first"],
        ],
    ) {
        Value::EventValue(ironsmith_core::EventValueSpec::BlockersBeyondFirst { multiplier: 1 })
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CreaturesBlockingIt)
    } else if exact_one_of(
        words,
        &[
            &["card", "looked", "at", "while", "scrying", "this", "way"],
            &["cards", "looked", "at", "while", "scrying", "this", "way"],
        ],
    ) {
        Value::EventValue(ironsmith_core::EventValueSpec::Amount)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsLookedAtWhileScryingThisWay)
    } else {
        return None;
    };
    Some((value, consumed))
}

pub(super) fn parse_for_each_head(words: &[&str]) -> Option<ForEachHead> {
    let mut item_start = if permission_shapes::prefix_words(words, &["for", "each"]) {
        2
    } else if permission_shapes::prefix_words(words, &["each"]) {
        1
    } else {
        return None;
    };
    if permission_shapes::starts_at_words(words, item_start, &["of"]) {
        item_start += 1;
    }
    if item_start >= words.len() {
        return None;
    }

    let other = permission_shapes::starts_at_words(words, item_start, &["other"])
        || permission_shapes::starts_at_words(words, item_start, &["another"]);
    if other {
        item_start += 1;
    }
    (item_start < words.len()).then_some(ForEachHead { item_start, other })
}

pub(super) fn value_boundary(words: &[&str]) -> usize {
    ["plus", "minus"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
        .unwrap_or(words.len())
}

pub(super) fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

pub(super) fn is_kick_count(words: &[&str]) -> bool {
    let Some((first, rest)) = words.split_first() else {
        return false;
    };
    if !permission_shapes::exact_words(&[*first], &["time"])
        && !permission_shapes::exact_words(&[*first], &["times"])
    {
        return false;
    }
    if rest.len() < 2 || !permission_shapes::suffix_words(rest, &["was", "kicked"]) {
        return false;
    }
    let source_words = &rest[..rest.len() - 2];
    exact_one_of(
        source_words,
        &[
            &["this"],
            &["this", "spell"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "card"],
            &["it"],
        ],
    ) || source_reference_surface_for_words(source_words).is_some()
}
