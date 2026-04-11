use super::*;

pub(crate) fn parse_remove(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if let Some(from_idx) = find_index(tokens, |token| token.is_word("from")) {
        let tail_words = crate::cards::builders::parser::token_word_refs(&tokens[from_idx + 1..]);
        if tail_words == ["combat"] {
            let target_tokens = trim_commas(&tokens[..from_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing remove-from-combat target (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            }
            let target = parse_target_phrase(&target_tokens)?;
            return Ok(EffectAst::RemoveFromCombat { target });
        }
    }

    if tokens.first().is_some_and(|token| token.is_word("all"))
        && let Some(counter_idx) = find_index(tokens, |token: &OwnedLexToken| {
            token.is_word("counter") || token.is_word("counters")
        })
        && counter_idx > 1
    {
        let counter_descriptor = trim_commas(&tokens[1..counter_idx]);
        let counter_type = parse_counter_type_from_descriptor_tokens(&counter_descriptor);
        let mut target_tokens = trim_commas(&tokens[counter_idx + 1..]);
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("from"))
        {
            target_tokens = trim_commas(&target_tokens[1..]);
        }

        let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
        let source_like_target = matches!(
            target_words.as_slice(),
            ["it"]
                | ["this"]
                | ["this", "creature"]
                | ["this", "artifact"]
                | ["this", "enchantment"]
                | ["this", "permanent"]
                | ["this", "card"]
        );
        if source_like_target {
            let amount = match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            };
            return Ok(EffectAst::RemoveUpToAnyCounters {
                amount,
                target: TargetAst::Source(span_from_tokens(&target_tokens)),
                counter_type,
                up_to: false,
            });
        }
    }

    let mut idx = 0;
    let mut up_to = false;
    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        up_to = true;
        idx += 2;
    }

    let (amount, used) = parse_value(&tokens[idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter removal amount (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;
    idx += used;

    let counter_idx = find_index(&tokens[idx..], |token: &OwnedLexToken| {
        token.is_word("counter") || token.is_word("counters")
    })
    .map(|offset| idx + offset)
    .ok_or_else(|| CardTextError::ParseError("missing counter keyword".to_string()))?;
    let counter_descriptor = trim_commas(&tokens[idx..counter_idx]);
    let counter_type = parse_counter_type_from_descriptor_tokens(&counter_descriptor);
    if counter_idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing counter keyword".to_string(),
        ));
    }
    idx = counter_idx + 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("from")) {
        idx += 1;
    }

    let target_tokens = trim_commas(&tokens[idx..]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("each") || token.is_word("all"))
    {
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(EffectAst::RemoveCountersAll {
            amount,
            filter,
            counter_type,
            up_to,
        });
    }

    let for_each_idx = find_window_by(&target_tokens, 2, |window: &[OwnedLexToken]| {
        window[0].is_word("for") && window[1].is_word("each")
    });
    if let Some(for_each_idx) = for_each_idx {
        let base_target_tokens = trim_commas(&target_tokens[..for_each_idx]);
        let count_filter_tokens = trim_commas(&target_tokens[for_each_idx + 2..]);
        if !base_target_tokens.is_empty() && !count_filter_tokens.is_empty() {
            if let (Ok(target), Ok(count_filter)) = (
                parse_target_phrase(&base_target_tokens),
                parse_object_filter(&count_filter_tokens, false),
            ) {
                return Ok(EffectAst::ForEachObject {
                    filter: count_filter,
                    effects: vec![EffectAst::RemoveUpToAnyCounters {
                        amount,
                        target,
                        counter_type,
                        up_to,
                    }],
                });
            }
        }
    }

    let target_tokens = trim_commas(&tokens[idx..]);
    let target = parse_target_phrase(&target_tokens)?;

    Ok(EffectAst::RemoveUpToAnyCounters {
        amount,
        target,
        counter_type,
        up_to,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedDestroyTimingAst {
    EndOfCombat,
    NextEndStep,
}

pub(crate) fn parse_delayed_destroy_timing_words(
    words: &[&str],
) -> Option<DelayedDestroyTimingAst> {
    if matches!(
        words,
        ["at", "end", "of", "combat"] | ["at", "the", "end", "of", "combat"]
    ) {
        return Some(DelayedDestroyTimingAst::EndOfCombat);
    }

    if matches!(
        words,
        ["at", "beginning", "of", "next", "end", "step"]
            | ["at", "beginning", "of", "the", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "next", "end", "step"]
            | ["at", "the", "beginning", "of", "the", "next", "end", "step"]
    ) {
        return Some(DelayedDestroyTimingAst::NextEndStep);
    }

    None
}

pub(crate) fn wrap_destroy_with_delayed_timing(
    effect: EffectAst,
    timing: Option<DelayedDestroyTimingAst>,
) -> EffectAst {
    let Some(timing) = timing else {
        return effect;
    };

    match timing {
        DelayedDestroyTimingAst::EndOfCombat => EffectAst::DelayedUntilEndOfCombat {
            effects: vec![effect],
        },
        DelayedDestroyTimingAst::NextEndStep => EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![effect],
        },
    }
}

pub(crate) fn parse_destroy(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let original_clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let mut delayed_timing = None;
    let mut timing_cut_word_idx = original_clause_words.len();
    for word_idx in 0..original_clause_words.len() {
        if original_clause_words[word_idx] != "at" {
            continue;
        }
        if let Some(timing) = parse_delayed_destroy_timing_words(&original_clause_words[word_idx..])
        {
            delayed_timing = Some(timing);
            timing_cut_word_idx = word_idx;
            break;
        }
    }

    let core_tokens = if timing_cut_word_idx < original_clause_words.len() {
        let token_cutoff =
            token_index_for_word_index(tokens, timing_cut_word_idx).unwrap_or(tokens.len());
        trim_commas(&tokens[..token_cutoff])
    } else {
        trim_commas(tokens)
    };
    let clause_words = crate::cards::builders::parser::token_word_refs(&core_tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing destroy target before delayed timing clause (clause: '{}')",
            original_clause_words.join(" ")
        )));
    }

    if delayed_timing.is_none()
        && (grammar::words_find_phrase(tokens, &["end", "of", "combat"]).is_some()
            || (grammar::contains_word(tokens, "beginning")
                && grammar::contains_word(tokens, "end")))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed destroy timing clause (clause: '{}')",
            original_clause_words.join(" ")
        )));
    }
    if let Some(target) = parse_destroy_combat_history_target(&core_tokens)? {
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::Destroy { target },
            delayed_timing,
        ));
    }
    let has_combat_history = (grammar::contains_word(&core_tokens, "dealt")
        && grammar::contains_word(&core_tokens, "damage")
        && grammar::contains_word(&core_tokens, "turn"))
        || find_window_by(&clause_words, 2, |window| {
            matches!(window, ["was", "blocked"] | ["was", "blocking"])
        })
        .is_some()
        || find_window_by(&clause_words, 2, |window| {
            matches!(
                window,
                ["blocking", "it"] | ["blocked", "it"] | ["it", "blocked"]
            )
        })
        .is_some();
    if has_combat_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history destroy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        if let Some(attached_idx) = find_index(&core_tokens, |token: &OwnedLexToken| {
            token.is_word("attached")
        }) && core_tokens
            .get(attached_idx + 1)
            .is_some_and(|token| token.is_word("to"))
            && attached_idx > 1
        {
            let mut filter_tokens = trim_commas(&core_tokens[1..attached_idx]).to_vec();
            while filter_tokens
                .last()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
            {
                filter_tokens.pop();
            }
            let target_tokens = trim_commas(&core_tokens[attached_idx + 2..]);
            let target_words = crate::cards::builders::parser::token_word_refs(&target_tokens);
            let has_timing_tail = target_words.iter().any(|word| {
                matches!(
                    *word,
                    "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
                )
            });
            let supported_target = grammar::words_match_prefix(&target_tokens, &["target"])
                .is_some()
                || target_words == ["it"]
                || grammar::words_match_any_prefix(&target_tokens, ATTACHED_REFERENCE_PREFIXES)
                    .is_some();
            if !filter_tokens.is_empty()
                && !target_tokens.is_empty()
                && supported_target
                && !has_timing_tail
            {
                let filter = parse_object_filter(&filter_tokens, false)?;
                let target = parse_target_phrase(&target_tokens)?;
                return Ok(wrap_destroy_with_delayed_timing(
                    EffectAst::DestroyAllAttachedTo { filter, target },
                    delayed_timing,
                ));
            }
        }
        if let Some(except_for_idx) =
            find_window_by(&core_tokens, 2, |window: &[OwnedLexToken]| {
                window[0].is_word("except") && window[1].is_word("for")
            })
            && except_for_idx > 1
        {
            let base_filter_tokens = trim_commas(&core_tokens[1..except_for_idx]);
            let exception_tokens = trim_commas(&core_tokens[except_for_idx + 2..]);
            if !base_filter_tokens.is_empty() && !exception_tokens.is_empty() {
                let mut filter = parse_object_filter(&base_filter_tokens, false)?;
                let exception_filter = parse_object_filter(&exception_tokens, false)?;
                apply_except_filter_exclusions(&mut filter, &exception_filter);
                return Ok(wrap_destroy_with_delayed_timing(
                    EffectAst::DestroyAll { filter },
                    delayed_timing,
                ));
            }
        }
        let filter_tokens = &core_tokens[1..];
        if let Some((choice_idx, consumed)) = find_color_choice_phrase(filter_tokens) {
            let base_filter_tokens = trim_commas(&filter_tokens[..choice_idx]);
            let trailing = trim_commas(&filter_tokens[choice_idx + consumed..]);
            if !trailing.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing color-choice destroy-all clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if base_filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing destroy-all filter before color-choice clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let filter = parse_object_filter(&base_filter_tokens, false)?;
            return Ok(wrap_destroy_with_delayed_timing(
                EffectAst::DestroyAllOfChosenColor { filter },
                delayed_timing,
            ));
        }
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::DestroyAll { filter },
            delayed_timing,
        ));
    }

    if grammar::contains_word(&core_tokens, "unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported destroy-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(if_idx) = find_index(&core_tokens, |token: &OwnedLexToken| token.is_word("if")) {
        let mut target_tokens = trim_commas(&core_tokens[..if_idx]).to_vec();
        while target_tokens
            .last()
            .is_some_and(|token| token.is_word("instead"))
        {
            target_tokens.pop();
        }
        let target_tokens = trim_commas(&target_tokens);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let target = parse_target_phrase(&target_tokens)?;
        let predicate_tail = parse_conditional_predicate_tail_lexed(&core_tokens[if_idx + 1..])
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported conditional destroy clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;

        return Ok(match predicate_tail {
            ConditionalPredicateTailSpec::InsteadIf {
                base_predicate,
                outer_predicate,
            } => wrap_destroy_with_delayed_timing(
                EffectAst::Conditional {
                    predicate: outer_predicate,
                    if_true: vec![EffectAst::Conditional {
                        predicate: base_predicate,
                        if_true: vec![EffectAst::Destroy {
                            target: target.clone(),
                        }],
                        if_false: Vec::new(),
                    }],
                    if_false: Vec::new(),
                },
                delayed_timing,
            ),
            ConditionalPredicateTailSpec::Plain(predicate) => wrap_destroy_with_delayed_timing(
                EffectAst::Conditional {
                    predicate,
                    if_true: vec![EffectAst::Destroy { target }],
                    if_false: Vec::new(),
                },
                delayed_timing,
            ),
        });
    }
    if let Some(and_idx) = find_index(&core_tokens, |token: &OwnedLexToken| token.is_word("and")) {
        let tail_slice = &core_tokens[and_idx + 1..];
        let starts_multi_target = tail_slice.first().is_some_and(|t| t.is_word("target"))
            || (grammar::words_match_any_prefix(tail_slice, UP_TO_PREFIXES).is_some()
                && grammar::contains_word(tail_slice, "target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if grammar::words_match_any_prefix(&core_tokens, TARGET_BLOCKED_PREFIXES).is_some() {
        let mut target_tokens = core_tokens.to_vec();
        if let Some(blocked_idx) = find_index(&target_tokens, |token: &OwnedLexToken| {
            token.is_word("blocked")
        }) {
            target_tokens.remove(blocked_idx);
        }
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(wrap_destroy_with_delayed_timing(
            EffectAst::Conditional {
                predicate: PredicateAst::TargetIsBlocked,
                if_true: vec![EffectAst::Destroy { target }],
                if_false: Vec::new(),
            },
            delayed_timing,
        ));
    }

    let target = parse_target_phrase(&core_tokens)?;
    Ok(wrap_destroy_with_delayed_timing(
        EffectAst::Destroy { target },
        delayed_timing,
    ))
}

pub(crate) fn parse_destroy_combat_history_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let Some(that_idx) = find_word_sequence_start(
        &clause_words,
        &["that", "was", "dealt", "damage", "this", "turn"],
    ) else {
        return Ok(None);
    };
    if that_idx == 0 || that_idx + 6 != clause_words.len() {
        return Ok(None);
    }
    let target_cutoff = token_index_for_word_index(tokens, that_idx).unwrap_or(tokens.len());
    let target_tokens = trim_commas(&tokens[..target_cutoff]);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let target = parse_target_phrase(&target_tokens)?;
    let TargetAst::Object(mut filter, target_span, it_span) = target else {
        return Ok(None);
    };
    filter.was_dealt_damage_this_turn = true;
    Ok(Some(TargetAst::Object(filter, target_span, it_span)))
}

pub(crate) fn apply_except_filter_exclusions(base: &mut ObjectFilter, exception: &ObjectFilter) {
    for card_type in exception
        .card_types
        .iter()
        .copied()
        .chain(exception.all_card_types.iter().copied())
    {
        if !slice_contains(&base.excluded_card_types, &card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    for subtype in exception.subtypes.iter().copied() {
        if !slice_contains(&base.excluded_subtypes, &subtype) {
            base.excluded_subtypes.push(subtype);
        }
    }
}
