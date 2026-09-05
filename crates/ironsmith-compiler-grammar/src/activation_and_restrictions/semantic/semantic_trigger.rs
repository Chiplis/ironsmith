use super::*;

pub(super) fn try_parse_combat_damage_trigger_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if trigger_atom_word(&words, TriggerClauseAtom::Deal).is_none()
        || !trigger_pattern_accepts(&words, COMBAT_DAMAGE_TRIGGER_PATTERN)
    {
        return Ok(None);
    }
    parse_combat_damage_trigger_lexed(tokens, &words).map(Some)
}

fn parse_combat_damage_trigger_lexed(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Result<TriggerSpec, CardTextError> {
    let Some(deals_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Deal) else {
        return Ok(TriggerSpec::ThisDealsCombatDamage);
    };
    let subject_tokens = &tokens[..deals_idx];
    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    let source_surface = source_reference_surface_for_words(&subject_words)
        .or_else(|| this_source_surface_for_words(&subject_words));
    let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
    let one_or_more = has_leading_one_or_more(subject_tokens) || player_subject;
    let source_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?;
    let Some(damage_idx_rel) =
        trigger_atom_token(&tokens[deals_idx + 1..], TriggerClauseAtom::Damage)
    else {
        return Ok(match source_filter {
            Some(filter) => TriggerSpec::DealsCombatDamage(filter),
            None => TriggerSpec::ThisDealsCombatDamage,
        });
    };
    let damage_idx = deals_idx + 1 + damage_idx_rel;
    let Some(to_idx_rel) = trigger_atom_token(&tokens[damage_idx + 1..], TriggerClauseAtom::To)
    else {
        return Ok(match source_filter {
            Some(filter) => TriggerSpec::DealsCombatDamage(filter),
            None => TriggerSpec::ThisDealsCombatDamage,
        });
    };
    let to_idx = damage_idx + 1 + to_idx_rel;
    let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing combat damage recipient filter in trigger clause (clause: '{}')",
            words.join(" ")
        )));
    }
    let target_word_view = ActivationRestrictionCompatWords::new(&target_tokens);
    let target_words = target_word_view.to_word_refs();
    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
        return Ok(match source_filter {
            Some(source) if one_or_more => {
                TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player }
            }
            Some(source) => TriggerSpec::DealsCombatDamageToPlayer { source, player },
            None => TriggerSpec::ThisDealsCombatDamageToPlayer {
                player,
                source_surface,
            },
        });
    }

    if let Some((player, target_filter, player_first)) =
        parse_player_or_object_damage_recipient(&target_tokens)
    {
        let player_trigger = match source_filter.clone() {
            Some(source) if one_or_more => {
                TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player }
            }
            Some(source) => TriggerSpec::DealsCombatDamageToPlayer { source, player },
            None if player == PlayerFilter::Any => TriggerSpec::ThisDealsCombatDamageToPlayer {
                player,
                source_surface: source_surface.clone(),
            },
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported combat damage player recipient filter in trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }
        };
        let object_trigger = match source_filter {
            Some(source) => TriggerSpec::DealsCombatDamageTo {
                source,
                target: target_filter,
            },
            None => TriggerSpec::ThisDealsCombatDamageTo(target_filter),
        };
        return Ok(if player_first {
            TriggerSpec::Either(Box::new(player_trigger), Box::new(object_trigger))
        } else {
            TriggerSpec::Either(Box::new(object_trigger), Box::new(player_trigger))
        });
    }

    let target_one_or_more = has_leading_one_or_more(&target_tokens);
    let target_tokens = strip_leading_one_or_more_lexed(&target_tokens);
    let target_words = ActivationRestrictionCompatWords::new(target_tokens).to_word_refs();
    let target_surface = source_reference_surface_for_words(&target_words)
        .or_else(|| this_source_surface_for_words(&target_words));
    let mut target_filter = if let Some(surface) = target_surface {
        ObjectFilter::source_with_surface(surface)
    } else {
        parse_object_filter_lexed(target_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported combat damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?
    };
    target_filter.set_union_one_or_more(target_one_or_more);
    Ok(match source_filter {
        Some(source) => TriggerSpec::DealsCombatDamageTo {
            source,
            target: target_filter,
        },
        None => TriggerSpec::ThisDealsCombatDamageTo(target_filter),
    })
}

pub(super) fn try_parse_source_with_filtered_attack_count_trigger_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if !matches!(words.last(), Some(&"attack" | &"attacks")) {
        return Ok(None);
    }

    let attack_word_idx = words.len().saturating_sub(1);
    let attack_token_idx =
        trigger_word_token_start(tokens, attack_word_idx).unwrap_or(tokens.len());
    let subject_tokens = &tokens[..attack_token_idx];
    let Some(and_idx) = trigger_atom_token(subject_tokens, TriggerClauseAtom::And) else {
        return Ok(None);
    };
    let left = trim_edge_punctuation(&subject_tokens[..and_idx]);
    let right = trim_edge_punctuation(&subject_tokens[and_idx + 1..]);
    if left.is_empty()
        || !token_slice_at_is(&right, 0, "at")
        || !token_slice_at_is(&right, 1, "least")
    {
        return Ok(None);
    }
    let Some((other_count, used)) = parse_number(&right[2..]) else {
        return Ok(None);
    };
    let other_surface = right
        .get(2 + used)
        .is_some_and(|token| token.is_word("other"));
    let filter_start = 2 + used + usize::from(other_surface);
    if right[filter_start..].is_empty() {
        return Ok(None);
    }
    let Some(other_filter) = parse_attack_trigger_subject_filter_lexed(&right[filter_start..])?
    else {
        return Ok(None);
    };

    let rendered_subject = crate::lexer::render_token_slice(&left).trim().to_string();
    let display_subject = (rendered_subject != "this")
        .then_some(rendered_subject)
        .filter(|subject| !subject.is_empty());
    Ok(Some(TriggerSpec::ThisAttacksWithNOthers {
        other_count,
        display_subject,
        other_filter: Some(other_filter),
        other_surface,
    }))
}

pub(super) fn parse_trigger_clause_lexed_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    {
        let words = crate::lexer::token_word_refs(tokens);
        if crate::word_primitives::parse_any_sequence_complete(
            &words,
            &[
                &["this", "phases", "out"],
                &["this", "creature", "phases", "out"],
                &["this", "permanent", "phases", "out"],
            ],
        ) {
            return Ok(TriggerSpec::ThisPhasesOut);
        }

        if let Some(entering_idx) =
            crate::slice_primitives::select_position(&words, |word| *word == "entering")
            && entering_idx > 0
            && words.get(entering_idx + 1..).is_some_and(|tail| {
                crate::word_primitives::parse_choice_sequence_complete(
                    tail,
                    &[
                        &["under"],
                        &["an"],
                        &["opponent's", "opponents", "opponent"],
                        &["control"],
                        &["causes"],
                        &["a"],
                        &["triggered"],
                        &["ability"],
                        &["of"],
                        &["that"],
                        &["creature"],
                        &["to"],
                        &["trigger"],
                    ],
                )
            })
        {
            let entering_token_idx = trigger_word_token_start(tokens, entering_idx)
                .ok_or_else(|| CardTextError::ParseError("missing entering source".to_string()))?;
            let source_tokens = strip_leading_articles(&tokens[..entering_token_idx]);
            let mut source_filter = parse_object_filter_lexed(&source_tokens, false)?;
            source_filter.controller = Some(PlayerFilter::Opponent);
            return Ok(TriggerSpec::AbilityTriggered {
                another: false,
                source_filter: Some(source_filter),
                caused_by_source_entering: true,
            });
        }
    }
    fn parse_damage_by_dies_trigger_lexed(
        subject_tokens: &[OwnedLexToken],
        other: bool,
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if subject_words.len() < 7 {
            return Ok(None);
        }

        let Some(dealt_word_idx) =
            crate::word_primitives::parse_sequence_start(&subject_words, &["dealt", "damage"])
        else {
            return Ok(None);
        };

        // Both Oracle orders carry the same history predicate:
        //   "dealt damage by [source] this turn"
        //   "dealt damage this turn by [source]"
        let (damager_start_word_idx, damager_end_word_idx) =
            if subject_words.get(dealt_word_idx + 2) == Some(&"by")
                && crate::word_primitives::parse_sequence_suffix(&subject_words, &["this", "turn"])
            {
                (dealt_word_idx + 3, subject_words.len() - 2)
            } else if subject_words
                .get(dealt_word_idx + 2..dealt_word_idx + 5)
                .is_some_and(|tail| {
                    crate::word_primitives::parse_sequence_complete(tail, &["this", "turn", "by"])
                })
            {
                (dealt_word_idx + 5, subject_words.len())
            } else {
                return Ok(None);
            };

        let victim_end = trigger_word_token_start(subject_tokens, dealt_word_idx).unwrap_or(0);
        if victim_end == 0 || victim_end > subject_tokens.len() {
            return Ok(None);
        }

        let victim_tokens = trim_edge_punctuation_tokens(&subject_tokens[..victim_end]);
        let victim_tokens = strip_leading_article_tokens(victim_tokens);
        if victim_tokens.is_empty() {
            return Ok(None);
        }

        let damager_start = trigger_word_token_start(subject_tokens, damager_start_word_idx)
            .unwrap_or(subject_tokens.len());
        let damager_end = trigger_word_token_start(subject_tokens, damager_end_word_idx)
            .unwrap_or(subject_tokens.len());
        if damager_start >= damager_end || damager_end > subject_tokens.len() {
            return Ok(None);
        }

        let damager_tokens =
            trim_edge_punctuation_tokens(&subject_tokens[damager_start..damager_end]);
        let damager_word_view = ActivationRestrictionCompatWords::new(damager_tokens);
        let damager_words = damager_word_view.to_word_refs();
        let has_named_source_words = !damager_words.is_empty()
            && !damager_words.first().is_some_and(|word| {
                trigger_word_accepts_pattern(word, DAMAGER_NAMED_SOURCE_LEADING_EXCLUDED_PATTERN)
            })
            && !damager_words
                .iter()
                .any(|word| trigger_word_accepts_pattern(word, GENERIC_DAMAGE_SOURCE_WORD_PATTERN));

        let damager = if trigger_pattern_accepts(&damager_words, THIS_DAMAGE_SOURCE_TRIGGER_PATTERN)
            || has_named_source_words
        {
            Some(DamageBySpec::ThisCreature)
        } else if trigger_pattern_accepts(
            &damager_words,
            EQUIPPED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN,
        ) {
            Some(DamageBySpec::EquippedCreature)
        } else if trigger_pattern_accepts(
            &damager_words,
            ENCHANTED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN,
        ) {
            Some(DamageBySpec::EnchantedCreature)
        } else {
            None
        };

        let victim = parse_object_filter_lexed(victim_tokens, other).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damaged-by trigger victim filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if let Some(damager) = damager {
            return Ok(Some(TriggerSpec::DiesCreatureDealtDamageByThisTurn {
                victim,
                damager,
            }));
        }

        let damager_filter = parse_object_filter_lexed(damager_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filtered damage source in dies trigger (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        Ok(Some(
            TriggerSpec::DiesCreatureDealtDamageByFilteredSourceThisTurn {
                victim,
                damager_filter,
            },
        ))
    }

    fn parse_simple_spell_activity_trigger_lexed(
        tokens: &[OwnedLexToken],
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        if !trigger_pattern_accepts(clause_words, SIMPLE_SPELL_ACTIVITY_OBJECT_PATTERN) {
            return Ok(None);
        }
        if trigger_pattern_accepts(clause_words, SIMPLE_SPELL_ACTIVITY_EXCLUDED_WORD_PATTERN)
            || trigger_pattern_accepts(clause_words, SIMPLE_SPELL_ACTIVITY_EXCLUDED_PHRASE_PATTERN)
            || clause_words
                .iter()
                .any(|word| matches!(*word, "exile" | "graveyard" | "hand"))
        {
            // Origin-qualified spell activity needs the typed spell-filter
            // parser below. This older fast path only modeled hand origins
            // and otherwise collapsed "from exile/graveyard" to an ordinary
            // stack spell.
            return Ok(None);
        }

        let cast_idx = trigger_atom_token(tokens, TriggerClauseAtom::Cast);
        let copy_idx = trigger_atom_token(tokens, TriggerClauseAtom::Copy);
        if cast_idx.is_none() && copy_idx.is_none() {
            return Ok(None);
        }

        let timing = crate::word_primitives::sequence_occurs(clause_words, &["during", "combat"])
            .then_some(ironsmith_core::TriggerTimingRestriction::DuringCombat);

        // The caster is named before the cast/copy verb; scanning the whole
        // clause lets a "targets you ..." tail shadow "an opponent casts".
        let subject_end = match (cast_idx, copy_idx) {
            (Some(cast), Some(copy)) => cast.min(copy),
            (Some(cast), None) => cast,
            (None, Some(copy)) => copy,
            (None, None) => tokens.len(),
        };
        let subject_view = ActivationRestrictionCompatWords::new(&tokens[..subject_end]);
        let subject_words = subject_view.to_word_refs();
        let actor = parse_subject_clause_player_filter(&subject_words);
        let parse_filter =
            |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
                let envelope =
                    crate::grammar::trigger_subjects::parse_spell_filter_envelope(filter_tokens);
                let filter_tokens = &filter_tokens[..envelope.end];
                let filter_words = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_words.to_word_refs();
                let parser_filter_words = crate::lexer::parser_token_word_refs(filter_tokens);
                let filter_surface_facts =
                    crate::grammar::trigger_subjects::parse_spell_filter_surface_facts(
                        &parser_filter_words,
                    );
                let from_their_hand = matches!(
                    (
                        filter_surface_facts.origin,
                        filter_surface_facts.owner,
                    ),
                    (
                        Some(crate::grammar::trigger_subjects::SpellOriginSurface::Hand),
                        Some(
                            crate::grammar::trigger_subjects::SpellOwnerSurface::SubjectActorPronoun
                        )
                    )
                );
                let from_your_hand = crate::word_primitives::sequence_occurs(
                    &filter_words,
                    &["from", "your", "hand"],
                );
                let from_a_hand =
                    crate::word_primitives::sequence_occurs(&filter_words, &["from", "a", "hand"])
                        || crate::word_primitives::sequence_occurs(
                            &filter_words,
                            &["from", "hand"],
                        );
                // A leading "another" excludes the trigger's own source; the
                // relative-clause filter paths don't self-detect it.
                let other = filter_words
                    .first()
                    .is_some_and(|word| matches!(*word, "another" | "other"));
                let is_unqualified_spell =
                    trigger_pattern_accepts(&filter_words, UNQUALIFIED_SPELL_FILTER_PATTERN);
                if filter_tokens.is_empty() || is_unqualified_spell {
                    return Ok(None);
                }
                parse_object_filter_lexed(filter_tokens, other)
                    .map(|mut filter| {
                        filter.other |= other;
                        // The event object is on the stack, but an authored
                        // "from ... hand" clause constrains the cast's origin.
                        if from_their_hand || from_your_hand || from_a_hand {
                            filter.zone = Some(Zone::Hand);
                            filter.owner = if from_your_hand {
                                Some(PlayerFilter::You)
                            } else if from_their_hand && !matches!(actor, PlayerFilter::Any) {
                                Some(actor.clone())
                            } else {
                                None
                            };
                        }
                        Some(filter)
                    })
                    .map_err(|err| {
                        CardTextError::ParseError(format!(
                            "unsupported spell trigger filter (clause: '{}') [{err:?}]",
                            filter_words.join(" ")
                        ))
                    })
            };

        if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
            let (first, second, first_is_cast) = if cast < copy {
                (cast, copy, true)
            } else {
                (copy, cast, false)
            };
            let between_view = ActivationRestrictionCompatWords::new(&tokens[first + 1..second]);
            let between_words = between_view.to_word_refs();
            if trigger_pattern_accepts(&between_words, CAST_OR_COPY_SEPARATOR_PATTERN) {
                let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
                let cast_trigger = TriggerSpec::SpellCast {
                    filter: filter.clone(),
                    mana_source_filter: None,
                    caster: actor.clone(),
                    timing,
                    during_turn: None,
                    min_spells_this_turn: None,
                    exact_spells_this_turn: None,
                    from_not_hand: false,
                };
                let copied_trigger = TriggerSpec::SpellCopied {
                    filter,
                    copier: actor,
                };
                return Ok(Some(if first_is_cast {
                    TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
                } else {
                    TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
                }));
            }
        }

        if let Some(cast) = cast_idx {
            let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
            if filter_tokens.is_empty() {
                let mut prefix_tokens = &tokens[..cast];
                while let Some(last_word) = prefix_tokens.last().and_then(OwnedLexToken::as_word) {
                    if trigger_word_accepts_pattern(last_word, LINKING_BE_WORD_PATTERN) {
                        prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                    } else {
                        break;
                    }
                }
                let has_spell_noun = prefix_tokens
                    .iter()
                    .any(|token| token_matches_clause_shape(token, SPELL_NOUN_PATTERN));
                if has_spell_noun {
                    filter_tokens = prefix_tokens;
                }
            }
            let filter = parse_filter(filter_tokens)?;
            return Ok(Some(TriggerSpec::SpellCast {
                filter,
                mana_source_filter: None,
                caster: actor,
                timing,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }));
        }

        if let Some(copy) = copy_idx {
            let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
            return Ok(Some(TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            }));
        }

        Ok(None)
    }

    fn parse_spell_countered_trigger_lexed(
        tokens: &[OwnedLexToken],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        let Some(spec) = ability_grammar::parse_spell_countered_trigger_spec_lexed(tokens) else {
            return Ok(None);
        };
        let filter = spec
            .filter_tokens
            .map(|filter_tokens| {
                let filter_words =
                    ActivationRestrictionCompatWords::new(filter_tokens).to_word_refs();
                parse_object_filter_lexed(filter_tokens, false).map_err(|err| {
                    CardTextError::ParseError(format!(
                        "unsupported spell-countered trigger filter (clause: '{}') [{err:?}]",
                        filter_words.join(" ")
                    ))
                })
            })
            .transpose()?;

        Ok(Some(TriggerSpec::SpellCountered {
            filter,
            controller: spec.controller,
        }))
    }

    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "empty trigger clause".to_string(),
        ));
    }

    if let Some(player) = parse_unpaid_cumulative_upkeep_player(&words) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::CumulativeUpkeepNotPaid,
            player,
        });
    }

    // Ordinary named/source counter-removal triggers are event clauses, even
    // when they do not carry the narrower "this way" provenance suffix. Keep
    // the counter descriptor and grouped surface so loyalty-removal triggers
    // expose the removed amount exactly once.
    let grouped_counter_start =
        crate::word_primitives::parse_sequence_prefix(&words, &["one", "or", "more"])
            .then_some((3usize, true))
            .or_else(|| {
                crate::word_primitives::parse_sequence_prefix(&words, &["a"])
                    .then_some((1usize, false))
            });
    if let Some((descriptor_start_word, one_or_more)) = grouped_counter_start
        && let Some(counter_idx) = words
            .iter()
            .enumerate()
            .skip(descriptor_start_word)
            .find_map(|(idx, word)| matches!(*word, "counter" | "counters").then_some(idx))
        && words
            .get(counter_idx + 1..counter_idx + 4)
            .is_some_and(|tail| {
                crate::word_primitives::parse_choice_sequence_complete(
                    tail,
                    &[&["is", "are"], &["removed"], &["from"]],
                )
            })
    {
        let caused_by_source =
            crate::word_primitives::parse_sequence_suffix(&words, &["this", "way"]);
        let source_word_end = words
            .len()
            .saturating_sub(usize::from(caused_by_source) * 2);
        let source_word_start = counter_idx + 4;
        if source_word_start < source_word_end
            && is_source_reference_words(&words[source_word_start..source_word_end])
        {
            let descriptor_start = trigger_word_token_start(tokens, descriptor_start_word)
                .ok_or_else(|| {
                    CardTextError::ParseError("missing counter descriptor start".to_string())
                })?;
            let descriptor_end =
                trigger_word_token_start(tokens, counter_idx).ok_or_else(|| {
                    CardTextError::ParseError("missing counter descriptor end".to_string())
                })?;
            let source_start = trigger_word_token_start(tokens, source_word_start)
                .ok_or_else(|| CardTextError::ParseError("missing counter source".to_string()))?;
            let source_end =
                trigger_word_token_start(tokens, source_word_end).unwrap_or(tokens.len());
            let counter_type = (descriptor_start < descriptor_end)
                .then(|| {
                    trigger_counter_type_from_descriptor(&tokens[descriptor_start..descriptor_end])
                })
                .flatten();
            if descriptor_start == descriptor_end || counter_type.is_some() {
                let filter =
                    source_reference_surface_for_trigger_subject(&tokens[source_start..source_end])
                        .map(ObjectFilter::source_with_surface)
                        .unwrap_or_else(ObjectFilter::source);
                return Ok(TriggerSpec::CounterRemovedFrom {
                    filter,
                    counter_type,
                    last: false,
                    one_or_more,
                    caused_by_source,
                });
            }
        }
    }

    // "the last <type> counter is removed from this <source>" is a
    // counter-removal event with an event-time zero remainder, not a generic
    // object subject. Preserve both the named counter and the source surface.
    if crate::word_primitives::parse_sequence_prefix(&words, &["the", "last"])
        && let Some(counter_idx) =
            crate::slice_primitives::select_position(&words, |word| *word == "counter")
        && counter_idx > 2
        && words
            .get(counter_idx + 1..counter_idx + 4)
            .is_some_and(|tail| {
                crate::word_primitives::parse_sequence_complete(tail, &["is", "removed", "from"])
            })
        && let Some(source_words) = words.get(counter_idx + 4..)
        && is_source_reference_words(source_words)
    {
        let descriptor_start = trigger_word_token_start(tokens, 2).ok_or_else(|| {
            CardTextError::ParseError("missing last-counter descriptor".to_string())
        })?;
        let descriptor_end = trigger_word_token_start(tokens, counter_idx)
            .ok_or_else(|| CardTextError::ParseError("missing last-counter noun".to_string()))?;
        let source_start = trigger_word_token_start(tokens, counter_idx + 4)
            .ok_or_else(|| CardTextError::ParseError("missing last-counter source".to_string()))?;
        let counter_type =
            trigger_counter_type_from_descriptor(&tokens[descriptor_start..descriptor_end])
                .ok_or_else(|| {
                    CardTextError::ParseError("unknown last-counter type".to_string())
                })?;
        let filter = source_reference_surface_for_trigger_subject(&tokens[source_start..])
            .map(ObjectFilter::source_with_surface)
            .unwrap_or_else(ObjectFilter::source);
        return Ok(TriggerSpec::CounterRemovedFrom {
            filter,
            counter_type: Some(counter_type),
            last: true,
            one_or_more: true,
            caused_by_source: false,
        });
    }

    // "... removed from this source this way" is a provenance-qualified
    // counter event, not a damage event whose subject happens to deal damage
    // in the following effect sentence.
    let counter_removed_source = [
        (
            7usize,
            true,
            ONE_OR_MORE_COUNTERS_REMOVED_FROM_PREFIX_PATTERN,
        ),
        (5usize, false, A_COUNTER_REMOVED_FROM_PREFIX_PATTERN),
    ]
    .into_iter()
    .find_map(|(source_word_start, one_or_more, prefix)| {
        if words.len() <= source_word_start + 2
            || !trigger_pattern_accepts(&words[..source_word_start], prefix)
            || !trigger_pattern_accepts(&words[words.len() - 2..], THIS_WAY_EXACT_PATTERN)
        {
            return None;
        }
        let source_word_end = words.len() - 2;
        let source_words = &words[source_word_start..source_word_end];
        if !is_source_reference_words(source_words) {
            return None;
        }
        let source_token_start = trigger_word_token_start(tokens, source_word_start)?;
        let source_token_end = trigger_word_token_start(tokens, source_word_end)?;
        let filter = source_reference_surface_for_trigger_subject(
            &tokens[source_token_start..source_token_end],
        )
        .map(ObjectFilter::source_with_surface)
        .unwrap_or_else(ObjectFilter::source);
        Some(TriggerSpec::CounterRemovedFrom {
            filter,
            counter_type: None,
            last: false,
            one_or_more,
            caused_by_source: true,
        })
    });
    if let Some(trigger) = counter_removed_source {
        return Ok(trigger);
    }

    if trigger_pattern_accepts(&words, CRAFT_EXILED_FROM_BATTLEFIELD_TRIGGER_PATTERN) {
        return Ok(
            TriggerSpec::ThisExiledFromBattlefieldDuringCostOfAbilityWithMarker {
                marker: "craft".to_string(),
            },
        );
    }

    if words.len() > 6
        && trigger_pattern_accepts(&words, FINAL_CHAPTER_ABILITY_RESOLVES_TRIGGER_PATTERN)
    {
        let mut filter =
            parse_object_filter_lexed(&tokens[5..tokens.len() - 1], false).map_err(|err| {
                CardTextError::ParseError(format!(
                    "unsupported final chapter trigger filter: {} [{err:?}]",
                    words[5..words.len() - 1].join(" ")
                ))
            })?;
        filter.zone.get_or_insert(Zone::Battlefield);
        return Ok(TriggerSpec::FinalChapterAbilityResolved(filter));
    }

    if trigger_pattern_accepts(&words, DAY_NIGHT_CHANGED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::DayNightChanged);
    }

    if let Some(enters_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Enter) {
        let tail = &tokens[enters_idx + 1..];
        let tail_words = ActivationRestrictionCompatWords::new(tail).to_word_refs();
        let shared_subject_or_combat_damage = trigger_pattern_accepts(
            &tail_words,
            SHARED_SUBJECT_ETB_OR_COMBAT_DAMAGE_TAIL_PATTERN,
        );
        if shared_subject_or_combat_damage {
            let or_idx = enters_idx + 3;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.extend_from_slice(&tokens[or_idx + 1..]);

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
        let shared_subject_or_attack =
            trigger_pattern_accepts(&tail_words, SHARED_SUBJECT_ETB_OR_ATTACK_TAIL_PATTERN);
        if shared_subject_or_attack {
            let or_idx = if trigger_pattern_accepts(&tail_words[..1], OR_WORD_PATTERN) {
                enters_idx + 1
            } else {
                enters_idx + 3
            };
            let attack_idx = or_idx + 1;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.push(tokens[attack_idx].clone());

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
    }

    // A tagged or filtered subject can occupy either side of one concrete
    // blocking pair: "enchanted creature blocks or becomes blocked by a
    // creature ...". Parse that relationship before the broad `or` splitter;
    // otherwise its right arm is interpreted independently and `becomes`
    // can fall through to an unrelated phase trigger.
    if let Some(blocks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Block)
        && words.get(blocks_word_idx..blocks_word_idx + 5)
            == Some(&["blocks", "or", "becomes", "blocked", "by"])
    {
        let subject_end = trigger_word_token_start(tokens, blocks_word_idx).unwrap_or(tokens.len());
        let other_start =
            trigger_word_token_start(tokens, blocks_word_idx + 5).unwrap_or(tokens.len());
        if subject_end > 0
            && other_start < tokens.len()
            && let Some(subject) =
                parse_attack_trigger_subject_filter_lexed(&tokens[..subject_end])?
        {
            let raw_other_tokens = trim_commas(&tokens[other_start..]);
            let one_or_more = has_leading_one_or_more(&raw_other_tokens);
            let other_tokens = strip_leading_one_or_more_lexed(&raw_other_tokens);
            if !other_tokens.is_empty() {
                let mut other = parse_object_filter_lexed(other_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported opposite blocking-object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
                preserve_trigger_filter_union_surface(&mut other, other_tokens);
                other.set_union_one_or_more(one_or_more);
                return Ok(TriggerSpec::BlocksOrBecomesBlockedByObject { subject, other });
            }
        }
    }

    if let Some(or_idx) = split_trigger_or_index(tokens) {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        let right_words = ActivationRestrictionCompatWords::new(right_tokens).to_word_refs();
        let shared_subject_end = if crate::word_primitives::parse_choice_sequence_complete(
            &right_words,
            &[&["block", "blocks"]],
        ) {
            trigger_atom_token(left_tokens, TriggerClauseAtom::Attack)
        } else if crate::word_primitives::parse_choice_sequence_complete(
            &right_words,
            &[&["attack", "attacks"]],
        ) {
            trigger_atom_token(left_tokens, TriggerClauseAtom::Block)
        } else if right_words
            .first()
            .is_some_and(|word| matches!(*word, "copy" | "copies"))
        {
            trigger_atom_token(left_tokens, TriggerClauseAtom::Cast)
        } else if right_words
            .first()
            .is_some_and(|word| matches!(*word, "cast" | "casts"))
        {
            trigger_atom_token(left_tokens, TriggerClauseAtom::Copy)
        } else {
            None
        };
        if let Some(subject_end) = shared_subject_end.filter(|end| *end > 0) {
            let mut shared_subject_right = left_tokens[..subject_end].to_vec();
            shared_subject_right.extend_from_slice(right_tokens);
            if let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(&shared_subject_right),
            ) {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    if let Some(and_idx) = trigger_atom_token(tokens, TriggerClauseAtom::And)
        && tokens
            .get(and_idx + 1)
            .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::TriggerIntro))
    {
        let left_raw_tokens = &tokens[..and_idx];
        let right_raw_tokens = &tokens[and_idx + 1..];
        let left_tokens = strip_leading_trigger_intro(left_raw_tokens);
        let right_tokens = strip_leading_trigger_intro(right_raw_tokens);
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(
                Box::new(apply_leading_trigger_intro_surface(left, left_raw_tokens)),
                Box::new(apply_leading_trigger_intro_surface(right, right_raw_tokens)),
            ));
        }
    }

    if words.len() >= 2
        && trigger_word_at_accepts_pattern(&words, words.len() - 1, ALONE_WORD_PATTERN)
        && trigger_word_at_accepts_pattern(&words, words.len() - 2, ATTACK_OR_ATTACKS_PATTERN)
    {
        let attacks_word_idx = words.len().saturating_sub(2);
        let attacks_token_idx =
            trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksAlone(filter),
                None => TriggerSpec::AttacksAlone(ObjectFilter::source()),
            },
        );
    }

    if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
        let tail_words = &words[attacks_word_idx + 1..];
        if trigger_pattern_accepts(
            tail_words,
            ATTACKS_YOU_OR_PLANESWALKER_YOU_CONTROL_TAIL_PATTERN,
        ) {
            let attacks_token_idx =
                trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            let subject_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?
                .unwrap_or_else(ObjectFilter::source);
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            return Ok(if player_subject {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(subject_filter)
            } else {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControl(subject_filter)
            });
        }
    }

    if words.len() >= 3
        && trigger_word_at_accepts_pattern(&words, words.len() - 3, ATTACK_OR_ATTACKS_PATTERN)
        && trigger_word_at_accepts_pattern(&words, words.len() - 2, WHILE_WORD_PATTERN)
        && trigger_word_at_accepts_pattern(&words, words.len() - 1, SADDLED_WORD_PATTERN)
    {
        let attacks_word_idx = words.len().saturating_sub(3);
        let attacks_token_idx =
            trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksWhileSaddled(filter),
                None => TriggerSpec::ThisAttacksWhileSaddled,
            },
        );
    }

    if trigger_pattern_accepts(&words, YOU_CAST_THIS_SPELL_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if let Some(spell_countered_trigger) = parse_spell_countered_trigger_lexed(tokens)? {
        return Ok(spell_countered_trigger);
    }
    if let Some(spell_activity_trigger) = parse_simple_spell_activity_trigger_lexed(tokens, &words)?
    {
        return Ok(spell_activity_trigger);
    }
    if let Some(spell_activity_trigger) = parse_spell_activity_trigger(tokens)? {
        return Ok(spell_activity_trigger);
    }

    if let Some(play_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Play) {
        let subject_tokens = &tokens[..play_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let trimmed_object_tokens = trim_commas(&tokens[play_idx + 1..]);
            let object_tokens = strip_leading_articles(&trimmed_object_tokens);
            let object_word_view = ActivationRestrictionCompatWords::new(&object_tokens);
            let object_words = object_word_view.to_word_refs();
            if object_words
                .iter()
                .any(|word| trigger_word_accepts_pattern(word, LAND_OR_LANDS_PATTERN))
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerPlaysLand { player, filter });
            }
        }
    }

    if let Some(search_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Search) {
        let subject_tokens = &tokens[..search_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let searched_tokens = trim_commas(&tokens[search_idx + 1..]);
            let searched_word_view = ActivationRestrictionCompatWords::new(&searched_tokens);
            let searched_words = searched_word_view.to_word_refs();
            if trigger_pattern_accepts(&searched_words, LIBRARY_SEARCH_TARGET_PATTERN) {
                return Ok(TriggerSpec::PlayerSearchesLibrary(player));
            }
        }
    }

    if let Some(shuffle_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Shuffle) {
        let subject_tokens = &tokens[..shuffle_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let shuffled_tokens = trim_commas(&tokens[shuffle_idx + 1..]);
        let shuffled_word_view = ActivationRestrictionCompatWords::new(&shuffled_tokens);
        let shuffled_words = shuffled_word_view.to_word_refs();
        if trigger_pattern_accepts(&shuffled_words, LIBRARY_SHUFFLE_TARGET_PATTERN)
            && let Some((player, caused_by_effect, source_controller_shuffles)) =
                parse_shuffle_trigger_subject(&subject_words)
        {
            return Ok(TriggerSpec::PlayerShufflesLibrary {
                player,
                caused_by_effect,
                source_controller_shuffles,
            });
        }
    }

    if let Some(give_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Give) {
        let subject_tokens = &tokens[..give_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let gifted_tokens = trim_commas(&tokens[give_idx + 1..]);
            let gifted_word_view = ActivationRestrictionCompatWords::new(&gifted_tokens);
            let gifted_words = gifted_word_view.to_word_refs();
            if trigger_pattern_accepts(&gifted_words, GIFT_TAIL_PATTERN) {
                return Ok(TriggerSpec::PlayerGivesGift(player));
            }
        }
    }

    if let Some(create_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Create) {
        let subject_tokens = &tokens[..create_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let object_tokens = trim_commas(&tokens[create_idx + 1..]);
            let one_or_more = has_leading_one_or_more(&object_tokens);
            let object_tokens = strip_leading_one_or_more_lexed(&object_tokens);
            let filter = parse_object_filter_lexed(object_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported token-created trigger filter (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            return Ok(TriggerSpec::TokensCreated {
                player,
                filter,
                one_or_more,
            });
        }
    }

    if let Some(tap_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Tap) {
        let subject_tokens = &tokens[..tap_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let after_tap = &tokens[tap_idx + 1..];
            if let Some(for_idx) = trigger_atom_token(after_tap, TriggerClauseAtom::For)
                && for_idx > 0
            {
                let object_tokens = trim_commas(&after_tap[..for_idx]);
                let object_tokens = strip_leading_articles(&object_tokens);
                if !object_tokens.is_empty()
                    && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
                {
                    return Ok(TriggerSpec::PlayerTapsForMana { player, filter });
                }
            }
        }
    }

    if let Some(tapped_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Tapped)
        && tapped_idx >= 2
        && tokens
            .get(tapped_idx.wrapping_sub(1))
            .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::IsOrAre))
    {
        let subject_tokens = &tokens[..tapped_idx - 1];
        let after_tapped = &tokens[tapped_idx + 1..];
        if crate::lexer::contains_token_word(after_tapped, "for") {
            let object_tokens = trim_commas(subject_tokens);
            let object_tokens = strip_leading_articles(&object_tokens);
            if !object_tokens.is_empty()
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerTapsForMana {
                    player: PlayerFilter::Any,
                    filter,
                });
            }
        }
    }

    if crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &["another", "ability", "triggers"],
            &["another", "triggered", "ability", "triggers"],
        ],
    ) {
        return Ok(TriggerSpec::AbilityTriggered {
            another: true,
            source_filter: None,
            caused_by_source_entering: false,
        });
    }
    if crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &["an", "ability", "triggers"],
            &["a", "triggered", "ability", "triggers"],
        ],
    ) {
        return Ok(TriggerSpec::AbilityTriggered {
            another: false,
            source_filter: None,
            caused_by_source_entering: false,
        });
    }

    if let Some(activate_idx) = trigger_atom_word(&words, TriggerClauseAtom::Activate) {
        let subject_tokens = &tokens[..activate_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(activator) = parse_trigger_subject_player_filter(&subject_words) {
            let raw_tail_words = &words[activate_idx + 1..];
            let tail_tokens = &tokens[activate_idx + 1..];
            let (activation_cost_has_tap, ability_tail_tokens, ability_tail_words) =
                split_activation_cost_tap_condition_tail_lexed(tail_tokens, raw_tail_words);
            let ability_tail_tokens = ability_tail_tokens.as_slice();
            let tail_words = ability_tail_words.as_slice();
            if let Some(filter) =
                parse_loyalty_ability_trigger_tail_lexed(ability_tail_tokens, tail_words)?
            {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only: false,
                    loyalty_only: true,
                    activation_cost_has_tap,
                });
            }
            if let Some(marker) = parse_named_ability_trigger_tail_lexed(ability_tail_tokens) {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter: ObjectFilter::default().with_ability_marker(marker),
                    non_mana_only: false,
                    loyalty_only: false,
                    activation_cost_has_tap,
                });
            }
            if let Some((owner_filter, marker)) =
                parse_possessive_ability_trigger_tail_lexed(ability_tail_tokens, tail_words)?
            {
                let filter = match marker {
                    Some(marker) => owner_filter.with_ability_marker(marker),
                    None => owner_filter,
                };
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only: false,
                    loyalty_only: false,
                    activation_cost_has_tap,
                });
            }
            if let Some((filter, non_mana_only)) =
                parse_ability_of_object_trigger_tail_lexed(ability_tail_tokens, tail_words)?
            {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only,
                    loyalty_only: false,
                    activation_cost_has_tap,
                });
            }
            if trigger_pattern_accepts(tail_words, ACTIVATED_ABILITY_TAIL_PATTERN) {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter: ObjectFilter::default(),
                    non_mana_only: trigger_pattern_accepts(tail_words, MANA_ABILITY_TAIL_PATTERN),
                    loyalty_only: false,
                    activation_cost_has_tap,
                });
            }
        }
    }

    if trigger_pattern_accepts(&words, THIS_LEAVES_BATTLEFIELD_TRIGGER_PATTERN)
        || (words.len() == 5
            && trigger_word_at_accepts_pattern(&words, 0, THIS_WORD_PATTERN)
            && trigger_word_at_accepts_pattern(&words, 2, LEAVES_WORD_PATTERN)
            && trigger_word_at_accepts_pattern(&words, 3, THE_WORD_PATTERN)
            && trigger_word_at_accepts_pattern(&words, 4, BATTLEFIELD_WORD_PATTERN))
    {
        let subject_word_count = if words.len() == 5 { 2 } else { 1 };
        let subject_token_end = trigger_word_token_start(tokens, subject_word_count)
            .unwrap_or(subject_word_count.min(tokens.len()));
        return Ok(this_leaves_battlefield_trigger_spec(
            source_reference_surface_for_trigger_subject(&tokens[..subject_token_end]),
        ));
    }

    if let Some(enters_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Enter)
        && trigger_pattern_accepts(&words, ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN)
    {
        let subject_number = enter_trigger_subject_number(words[enters_word_idx]);
        let enters_token_idx =
            trigger_word_token_start(tokens, enters_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..enters_token_idx];
        if let Some(surface) = source_reference_surface_for_trigger_subject(
            strip_leading_trigger_intro(subject_tokens),
        ) {
            return Ok(TriggerSpec::Either(
                Box::new(this_enters_battlefield_trigger_spec(
                    Some(surface.clone()),
                    subject_number,
                    None,
                )),
                Box::new(this_leaves_battlefield_trigger_spec(Some(surface))),
            ));
        }
        if token_trigger_pattern_accepts(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN) {
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::ThisEntersBattlefield {
                    origin_condition: None,
                }),
                Box::new(TriggerSpec::ThisLeavesBattlefield),
            ));
        }
    }

    if let Some(leaves_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Leave)
        && trigger_pattern_accepts(
            &words[leaves_word_idx..],
            LEAVES_BATTLEFIELD_WITHOUT_DYING_SUFFIX_PATTERN,
        )
    {
        let leaves_token_idx =
            trigger_word_token_start(tokens, leaves_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..leaves_token_idx];

        // This form is destination-sensitive: it fires for battlefield exits
        // to hand, exile, library, and so on, but not for deaths. Keep the
        // exclusion typed instead of compiling a broad leaves trigger and
        // treating "without dying" as presentation.
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in leaves-battlefield-without-dying trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let one_or_more = has_leading_one_or_more(subject_tokens) || filter.union_is_one_or_more();
        filter.set_union_one_or_more(false);
        return Ok(TriggerSpec::LeavesBattlefieldWithoutDying {
            filter,
            one_or_more,
        });
    }

    if let Some(leaves_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Leave)
        && trigger_pattern_accepts(&words[leaves_word_idx..], LEAVES_BATTLEFIELD_SUFFIX_PATTERN)
    {
        let leaves_token_idx =
            trigger_word_token_start(tokens, leaves_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..leaves_token_idx];

        if let Some(surface) = source_reference_surface_for_trigger_subject(
            strip_leading_trigger_intro(subject_tokens),
        ) {
            return Ok(this_leaves_battlefield_trigger_spec(Some(surface)));
        }

        if let Some(or_idx) = trigger_atom_token(subject_tokens, TriggerClauseAtom::Or) {
            let left_tokens = &subject_tokens[..or_idx];
            let mut right_tokens = &subject_tokens[or_idx + 1..];
            let left_words = non_article_word_refs(
                &ActivationRestrictionCompatWords::new(left_tokens).to_word_refs(),
            );
            if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                let mut other = false;
                if token_trigger_pattern_accepts(right_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN) {
                    other = true;
                    right_tokens = &right_tokens[1..];
                }
                let parsed_filter =
                    parse_subtype_list_enters_trigger_filter_lexed(right_tokens, other).or_else(
                        || {
                            crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
                                right_tokens,
                                other,
                            ))
                        },
                    );
                if let Some(filter) = parsed_filter {
                    return Ok(TriggerSpec::Either(
                        Box::new(TriggerSpec::ThisLeavesBattlefield),
                        Box::new(TriggerSpec::LeavesBattlefield(filter)),
                    ));
                }
            }
        }

        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if token_trigger_pattern_accepts(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        // `the chosen <object>` is a durable cross-ability reference. The
        // ordinary object-filter parser accepts the same words as a broad
        // descriptive filter, but that erases the canonical chosen-object
        // tag before lowering can prove that the earlier choice must persist.
        // Route only the grammar-confirmed chosen-object surface through the
        // typed trigger-subject parser before the broad leaves filter.
        if crate::grammar::targets::parse_chosen_object_target(filtered_subject_tokens).is_some()
            && let Some(mut filter) = parse_trigger_subject_filter_lexed(filtered_subject_tokens)?
        {
            if other {
                filter.other = true;
            }
            return Ok(TriggerSpec::LeavesBattlefield(filter));
        }
        let parsed_filter =
            parse_subtype_list_enters_trigger_filter_lexed(filtered_subject_tokens, other).or_else(
                || {
                    crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
                        filtered_subject_tokens,
                        other,
                    ))
                },
            );
        if let Some(filter) = parsed_filter {
            return Ok(TriggerSpec::LeavesBattlefield(filter));
        }
    }

    if let Some(dies_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Die) {
        let dies_token_idx =
            trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..dies_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && trigger_pattern_accepts(
                &words[dies_word_idx + 1..],
                OR_IS_PUT_INTO_EXILE_FROM_BATTLEFIELD_TAIL_PATTERN,
            )
        {
            return Ok(source_reference_surface_for_trigger_subject(subject_tokens)
                .map(TriggerSpec::ThisDiesOrIsExiledWithSurface)
                .unwrap_or(TriggerSpec::ThisDiesOrIsExiled));
        }
    }

    if let Some(enters_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Enter) {
        let enters_during_turn = trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX)
            .then_some(PlayerFilter::You);
        let subject_number = enter_trigger_subject_number(words[enters_word_idx]);
        let enters_token_idx =
            trigger_word_token_start(tokens, enters_word_idx).unwrap_or(tokens.len());
        let origin_condition = parse_moved_or_cast_origin_condition(&words[enters_word_idx + 1..]);
        if trigger_pattern_accepts(&words, ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN) {
            let subject_tokens = &tokens[..enters_token_idx];
            if let Some(surface) = source_reference_surface_for_trigger_subject(
                strip_leading_trigger_intro(subject_tokens),
            ) {
                return Ok(TriggerSpec::Either(
                    Box::new(this_enters_battlefield_trigger_spec(
                        Some(surface.clone()),
                        subject_number,
                        None,
                    )),
                    Box::new(this_leaves_battlefield_trigger_spec(Some(surface))),
                ));
            }
            if token_trigger_pattern_accepts(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN)
            {
                return Ok(TriggerSpec::Either(
                    Box::new(TriggerSpec::ThisEntersBattlefield {
                        origin_condition: None,
                    }),
                    Box::new(TriggerSpec::ThisLeavesBattlefield),
                ));
            }
        }

        let enters_origin =
            trigger_grammar::parse_enters_origin_clause_words(&words[enters_word_idx + 1..])
                .map(|origin| (origin.zone, origin.owner));
        if enters_word_idx == 0 {
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: ObjectFilter::default(),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield { origin_condition }
            });
        }

        let subject_tokens = &tokens[..enters_token_idx];
        if trigger_pattern_accepts(
            &words[enters_word_idx + 1..],
            OR_IS_PUT_INTO_GRAVEYARD_FROM_BATTLEFIELD_TAIL_PATTERN,
        ) {
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::Either(
                    Box::new(this_enters_battlefield_trigger_spec(
                        source_reference_surface_for_trigger_subject(subject_tokens),
                        subject_number,
                        None,
                    )),
                    Box::new(TriggerSpec::PutIntoGraveyardFromZone {
                        filter: ObjectFilter::source(),
                        from: Zone::Battlefield,
                        one_or_more: false,
                    }),
                ));
            }
        }
        if trigger_pattern_accepts(
            &words[enters_word_idx + 1..],
            OR_TRANSFORMS_INTO_TAIL_PREFIX_PATTERN,
        ) {
            let destination_name =
                transform_destination_name_after_into(&word_view, enters_word_idx + 2, tokens);
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::Either(
                    Box::new(this_enters_battlefield_trigger_spec(
                        source_reference_surface_for_trigger_subject(subject_tokens),
                        subject_number,
                        None,
                    )),
                    Box::new(this_transforms_trigger_spec(
                        source_reference_surface_for_trigger_subject(subject_tokens),
                        destination_name,
                    )),
                ));
            }
        }
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(shape) =
            crate::grammar::trigger_subjects::parse_source_or_another_shape(&subject_words)
            && shape.one_or_more
            && (shape.connector_words == 2
                || subject_words.get(shape.connector_word) == Some(&"and/or"))
            && let Some((source_filter, mut other_filter)) =
                parse_source_or_another_trigger_subject_filters(subject_tokens)
        {
            other_filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
            let cause_filter = if contains_window(&words, &["without", "being", "played"]) {
                Some(crate::events::cause::CauseFilter::not_type(
                    crate::events::cause::CauseType::SpecialAction,
                ))
            } else {
                None
            };
            return Ok(TriggerSpec::Either(
                Box::new(this_enters_battlefield_trigger_spec(
                    source_filter.source_surface,
                    ironsmith_core::trigger_model::TriggerSubjectNumber::Singular,
                    origin_condition.clone(),
                )),
                Box::new(TriggerSpec::EntersBattlefieldOneOrMore {
                    filter: other_filter,
                    cause_filter,
                    origin_condition,
                }),
            ));
        }
        if let Some(or_idx) = trigger_atom_token(subject_tokens, TriggerClauseAtom::Or) {
            let or_is_one_or_more_quantifier = or_idx == 1
                && subject_tokens
                    .first()
                    .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::One))
                && subject_tokens
                    .get(or_idx + 1)
                    .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::More));
            if or_is_one_or_more_quantifier {
                // "one or more" is a quantifier for a single ETB trigger, not
                // a source-or-other-subject disjunction like "this creature or a token".
            } else {
                let left_tokens = &subject_tokens[..or_idx];
                let mut right_tokens = &subject_tokens[or_idx + 1..];
                let left_word_view = ActivationRestrictionCompatWords::new(left_tokens);
                let left_words = non_article_word_refs(&left_word_view.to_word_refs());
                if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                    let mut other = false;
                    if token_trigger_pattern_accepts(right_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
                    {
                        other = true;
                        right_tokens = &right_tokens[1..];
                    }
                    let parsed_filter =
                        parse_subtype_list_enters_trigger_filter_lexed(right_tokens, other)
                            .or_else(|| {
                                crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
                                    right_tokens,
                                    other,
                                ))
                            });
                    if let Some(mut filter) = parsed_filter {
                        if trigger_pattern_accepts(&words, UNDER_YOUR_CONTROL_PATTERN) {
                            filter.controller = Some(PlayerFilter::You);
                            filter.set_enters_under_controller_surface(true);
                        } else if trigger_pattern_accepts(&words, UNDER_OPPONENT_CONTROL_PATTERN) {
                            filter.controller = Some(PlayerFilter::Opponent);
                            filter.set_enters_under_controller_surface(true);
                        }
                        let cause_filter =
                            if contains_window(&words, &["without", "being", "played"]) {
                                Some(crate::events::cause::CauseFilter::not_type(
                                    crate::events::cause::CauseType::SpecialAction,
                                ))
                            } else {
                                None
                            };
                        let right_trigger =
                            if trigger_pattern_accepts(&words, UNTAPPED_WORD_PATTERN) {
                                TriggerSpec::EntersBattlefieldUntapped {
                                    filter,
                                    cause_filter,
                                }
                            } else if trigger_pattern_accepts(&words, TAPPED_WORD_PATTERN) {
                                TriggerSpec::EntersBattlefieldTapped {
                                    filter,
                                    cause_filter,
                                }
                            } else {
                                TriggerSpec::EntersBattlefield {
                                    filter,
                                    cause_filter,
                                    origin_condition: origin_condition.clone(),
                                    during_turn: enters_during_turn.clone(),
                                }
                            };
                        return Ok(TriggerSpec::Either(
                            Box::new(this_enters_battlefield_trigger_spec(
                                source_reference_surface_for_trigger_subject(left_tokens),
                                subject_number,
                                origin_condition,
                            )),
                            Box::new(right_trigger),
                        ));
                    }
                }
            }
        }
        if token_trigger_pattern_accepts(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN) {
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: trigger_grammar::parse_source_trigger_subject_words(
                        &subject_words,
                    )
                    .filter,
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield {
                    origin_condition: origin_condition.clone(),
                }
            });
        }
        if let Some(surface) = source_reference_surface_for_trigger_subject(subject_tokens) {
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: ObjectFilter::default(),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefieldWithSurface {
                    surface,
                    subject_number,
                    origin_condition: origin_condition.clone(),
                }
            });
        }

        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if token_trigger_pattern_accepts(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let one_or_more = ActivationRestrictionCompatWords::new(filtered_subject_tokens)
            .to_word_refs()
            .get(..3)
            .is_some_and(|words| trigger_pattern_accepts(words, ONE_OR_MORE_QUANTIFIER_PATTERN));
        filtered_subject_tokens = strip_leading_one_or_more_lexed(filtered_subject_tokens);
        if token_trigger_pattern_accepts(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let parsed_filter =
            parse_subtype_list_enters_trigger_filter_lexed(filtered_subject_tokens, other).or_else(
                || {
                    crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
                        filtered_subject_tokens,
                        other,
                    ))
                },
            );
        if let Some(mut filter) = parsed_filter {
            preserve_trigger_filter_union_surface(&mut filter, filtered_subject_tokens);
            let cause_filter = if contains_window(&words, &["without", "being", "played"]) {
                Some(crate::events::cause::CauseFilter::not_type(
                    crate::events::cause::CauseType::SpecialAction,
                ))
            } else {
                None
            };
            if trigger_pattern_accepts(&words, UNDER_YOUR_CONTROL_PATTERN) {
                filter.controller = Some(PlayerFilter::You);
                filter.set_enters_under_controller_surface(true);
            } else if trigger_pattern_accepts(&words, UNDER_OPPONENT_CONTROL_PATTERN) {
                filter.controller = Some(PlayerFilter::Opponent);
                filter.set_enters_under_controller_surface(true);
            }
            if trigger_pattern_accepts(&words, UNTAPPED_WORD_PATTERN) {
                return Ok(TriggerSpec::EntersBattlefieldUntapped {
                    filter,
                    cause_filter,
                });
            }
            if trigger_pattern_accepts(&words, TAPPED_WORD_PATTERN) {
                return Ok(TriggerSpec::EntersBattlefieldTapped {
                    filter,
                    cause_filter,
                });
            }
            return Ok(if let Some((from, owner)) = enters_origin {
                TriggerSpec::EntersBattlefieldFromZone {
                    filter,
                    from,
                    owner,
                    one_or_more,
                    cause_filter,
                }
            } else if one_or_more {
                TriggerSpec::EntersBattlefieldOneOrMore {
                    filter,
                    cause_filter,
                    origin_condition,
                }
            } else {
                TriggerSpec::EntersBattlefield {
                    filter,
                    cause_filter,
                    origin_condition,
                    during_turn: enters_during_turn,
                }
            });
        }
    }

    if let Some(transforms_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Transform) {
        let transforms_token_idx =
            trigger_word_token_start(tokens, transforms_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..transforms_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && words
                .get(transforms_word_idx + 1)
                .is_some_and(|word| trigger_word_accepts_pattern(word, INTO_WORD_PATTERN))
        {
            let destination_name =
                transform_destination_name_after_into(&word_view, transforms_word_idx, tokens);
            return Ok(this_transforms_trigger_spec(
                source_reference_surface_for_trigger_subject(subject_tokens),
                destination_name,
            ));
        }
    }

    let (zone_change_words, during_turn) =
        if trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX) {
            (
                &words[..words.len().saturating_sub(3)],
                Some(PlayerFilter::You),
            )
        } else {
            (words.as_slice(), None)
        };

    if trigger_pattern_accepts(
        zone_change_words,
        SPELL_OR_ABILITY_YOU_CONTROL_EXILES_PERMANENTS_FROM_BATTLEFIELD_PATTERN,
    ) {
        return Ok(TriggerSpec::PutIntoExileFromZones {
            filter: ObjectFilter::permanent_card(),
            from: vec![Zone::Battlefield],
            one_or_more: true,
            during_turn,
            cause_filter: Some(
                crate::events::cause::CauseFilter::effect_like()
                    .with_controller(crate::events::cause::ControllerFilter::ContextController),
            ),
        });
    }

    for tail in [
        ["leave", "your", "graveyard"].as_slice(),
        ["leaves", "your", "graveyard"].as_slice(),
    ] {
        if trigger_pattern_accepts(zone_change_words, ClauseShape::new().suffix(tail)) {
            let subject_word_len = zone_change_words.len().saturating_sub(tail.len());
            let mut subject_tokens = trigger_word_token_start(tokens, subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let one_or_more = has_leading_one_or_more(subject_tokens);
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            let mut filter = if subject_is_card_or_cards(&subject_words) {
                ObjectFilter::default()
            } else {
                parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported filter in leave-your-graveyard trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            filter.zone = None;
            filter.controller = None;
            filter.owner = None;
            if subject_mentions_card(&subject_words) {
                filter.nontoken = true;
                filter.set_explicit_card_noun(true);
            }
            return Ok(TriggerSpec::CardsLeaveYourGraveyard {
                filter,
                one_or_more,
                during_your_turn: during_turn == Some(PlayerFilter::You),
            });
        }
    }

    if let Some(suffix_word_len) = trigger_suffix_word_len(
        zone_change_words,
        PUT_INTO_GRAVEYARD_OR_EXILE_FROM_BATTLEFIELD_SUFFIXES,
    ) {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, zone_change_words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
        let stripped_subject_words =
            ActivationRestrictionCompatWords::new(subject_tokens).to_word_refs();
        let mut filter = if subject_is_card_or_cards(&stripped_subject_words) {
            ObjectFilter::default()
        } else {
            parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-graveyard-or-exile-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?
        };
        filter.zone = None;
        filter.owner = None;
        if filter.controller.is_none()
            && (trigger_pattern_accepts(&subject_words, UNDER_YOUR_CONTROL_PATTERN)
                || contains_window(&subject_words, &["you", "control"]))
        {
            filter.controller = Some(PlayerFilter::You);
        }
        if subject_mentions_card(&subject_words) {
            filter.card_types.clear();
            filter.nontoken = true;
            filter.set_explicit_card_noun(true);
        }
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::PutIntoGraveyardFromZone {
                filter: filter.clone(),
                from: Zone::Battlefield,
                one_or_more,
            }),
            Box::new(TriggerSpec::PutIntoExileFromZones {
                filter,
                from: vec![Zone::Battlefield],
                one_or_more,
                during_turn,
                cause_filter: None,
            }),
        ));
    }

    for (tail, from_zones) in [
        (["is", "put", "into", "exile"].as_slice(), Vec::new()),
        (["are", "put", "into", "exile"].as_slice(), Vec::new()),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and",
                "or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and",
                "or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and/or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and/or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and",
                "or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and",
                "or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and/or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and/or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            ["is", "put", "into", "exile", "from", "your", "hand"].as_slice(),
            vec![Zone::Hand],
        ),
        (
            ["are", "put", "into", "exile", "from", "your", "hand"].as_slice(),
            vec![Zone::Hand],
        ),
        (
            ["is", "put", "into", "exile", "from", "your", "graveyard"].as_slice(),
            vec![Zone::Graveyard],
        ),
        (
            ["are", "put", "into", "exile", "from", "your", "graveyard"].as_slice(),
            vec![Zone::Graveyard],
        ),
    ] {
        if trigger_pattern_accepts(zone_change_words, ClauseShape::new().suffix(tail)) {
            let from_your_hand = trigger_pattern_accepts(tail, FROM_YOUR_HAND_SUFFIX_PATTERN)
                || crate::word_primitives::parse_sequence_suffix(
                    tail,
                    &["from", "your", "graveyard"],
                );
            let subject_word_len = zone_change_words.len().saturating_sub(tail.len());
            let subject_tokens = trigger_word_token_start(tokens, subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            let one_or_more = subject_starts_one_or_more(&subject_words);
            let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            let stripped_subject_words =
                ActivationRestrictionCompatWords::new(subject_tokens).to_word_refs();
            let mut filter = if subject_is_card_or_cards(&stripped_subject_words) {
                ObjectFilter::default()
            } else {
                parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported filter in put-into-exile-from-zones trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            filter.zone = None;
            filter.controller = None;
            filter.owner = if from_your_hand {
                Some(PlayerFilter::You)
            } else {
                None
            };
            if subject_mentions_card(&subject_words) {
                filter.card_types.clear();
                filter.nontoken = true;
                filter.set_explicit_card_noun(true);
            }
            return Ok(TriggerSpec::PutIntoExileFromZones {
                filter,
                from: from_zones,
                one_or_more,
                during_turn,
                cause_filter: None,
            });
        }
    }

    // Origin-qualified graveyard triggers are more specific than the broad
    // `put into your graveyard` family. Claim them first so the trailing
    // `from your library/battlefield` cannot be swallowed as part of the
    // object subject and downgraded to an any-origin trigger.
    if let Some(trigger) = parse_put_into_your_graveyard_from_exact_zone(
        tokens,
        &words,
        PUT_INTO_YOUR_GRAVEYARD_FROM_LIBRARY_SUFFIXES,
        Zone::Library,
    )? {
        return Ok(trigger);
    }
    if let Some(trigger) = parse_put_into_your_graveyard_from_exact_zone(
        tokens,
        &words,
        PUT_INTO_YOUR_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES,
        Zone::Battlefield,
    )? {
        return Ok(trigger);
    }

    if let Some(suffix_word_len) = trigger_suffix_word_len(&words, PUT_INTO_YOUR_GRAVEYARD_SUFFIXES)
    {
        let mut subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let one_or_more = has_leading_one_or_more(subject_tokens);
        subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported card filter in put-into-your-graveyard trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        filter.zone = None;
        filter.controller = None;
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        if subject_mentions_permanent(&subject_words) {
            filter.card_types = ObjectFilter::permanent_card().card_types;
        }
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
            filter.set_explicit_card_noun(true);
        }
        return Ok(if one_or_more {
            TriggerSpec::PutIntoGraveyardOneOrMore(filter)
        } else {
            TriggerSpec::PutIntoGraveyard(filter)
        });
    }

    if let Some(suffix_word_len) = trigger_suffix_word_len(
        &words,
        PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE_EXCEPT_BATTLEFIELD_SUFFIXES,
    ) {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in put-into-graveyard-from-outside-battlefield trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        filter.zone = None;
        filter.controller = None;
        filter.owner = None;
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
            filter.set_explicit_card_noun(true);
        }
        return Ok(TriggerSpec::PutIntoGraveyardFromAnyExcept {
            filter,
            excluded: Zone::Battlefield,
            one_or_more,
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        if is_source_reference_words(&subject_words) {
            return Ok(TriggerSpec::PutIntoGraveyard(ObjectFilter::source()));
        }
        if let Ok(filter) = parse_object_filter_lexed(subject_tokens, false) {
            return Ok(TriggerSpec::PutIntoGraveyard(filter));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported filter in put-into-graveyard-from-anywhere trigger clause (clause: '{}')",
            words.join(" ")
        )));
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_OPPONENT_GRAVEYARD_FROM_ANYWHERE_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if is_source_reference_words(&subject_words) {
            let mut filter = ObjectFilter::source();
            filter.owner = Some(PlayerFilter::Opponent);
            return Ok(if one_or_more {
                TriggerSpec::PutIntoGraveyardOneOrMore(filter)
            } else {
                TriggerSpec::PutIntoGraveyard(filter)
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-opponents-graveyard-from-anywhere trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        filter.owner = Some(PlayerFilter::Opponent);
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
            filter.set_explicit_card_noun(true);
        }
        return Ok(if one_or_more {
            TriggerSpec::PutIntoGraveyardOneOrMore(filter)
        } else {
            TriggerSpec::PutIntoGraveyard(filter)
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, ATTACHED_OBJECT_PUT_INTO_GRAVEYARD_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        if trigger_pattern_accepts(&subject_words, ATTACHED_OBJECT_PREFIX_PATTERN) {
            let one_or_more = subject_starts_one_or_more(&subject_words);
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in attached-object put-into-graveyard trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.owner = None;
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
                one_or_more,
            });
        }
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_YOUR_GRAVEYARD_FROM_LIBRARY_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard-from-library trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
            filter.set_explicit_card_noun(true);
        }
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Library,
            one_or_more,
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_YOUR_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if is_source_reference_words(&subject_words) {
            let mut filter = source_reference_surface_for_trigger_subject(subject_tokens)
                .map(ObjectFilter::source_with_surface)
                .unwrap_or_else(ObjectFilter::source);
            filter.owner = Some(PlayerFilter::You);
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
                one_or_more,
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
            filter.set_explicit_card_noun(true);
        }
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Battlefield,
            one_or_more,
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if let Some((mut source_filter, mut other_filter)) =
            parse_source_or_another_trigger_subject_filters(subject_tokens)
        {
            source_filter.zone = None;
            source_filter.owner = None;
            other_filter.zone = None;
            other_filter.owner = None;
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::PutIntoGraveyardFromZone {
                    filter: source_filter,
                    from: Zone::Battlefield,
                    one_or_more,
                }),
                Box::new(TriggerSpec::PutIntoGraveyardFromZone {
                    filter: other_filter,
                    from: Zone::Battlefield,
                    one_or_more,
                }),
            ));
        }
        if is_source_reference_words(&subject_words) {
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter: ObjectFilter::source(),
                from: Zone::Battlefield,
                one_or_more,
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-a-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.owner = None;
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
            filter.set_explicit_card_noun(true);
        }
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Battlefield,
            one_or_more,
        });
    }

    if let Some(suffix_word_len) = trigger_suffix_word_len(
        &words,
        PUT_INTO_OPPONENT_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES,
    ) {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if is_source_reference_words(&subject_words) {
            let mut filter = ObjectFilter::source();
            filter.owner = Some(PlayerFilter::Opponent);
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
                one_or_more,
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-opponents-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        filter.owner = Some(PlayerFilter::Opponent);
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Battlefield,
            one_or_more,
        });
    }

    if let Some(put_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Put)
        && let Some(source_controller) = parse_trigger_subject_player_filter(&words[..put_word_idx])
        && let Some(counter_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Counter)
        && counter_word_idx > put_word_idx
        && words
            .get(counter_word_idx + 1..counter_word_idx + 2)
            .is_some_and(|preposition| {
                trigger_pattern_accepts(preposition, COUNTER_RECIPIENT_PREPOSITION_PATTERN)
            })
    {
        let descriptor_word_start = put_word_idx + 1;
        let (descriptor_span, counter_descriptor_tokens) = trigger_counter_descriptor_span(
            tokens,
            descriptor_word_start,
            counter_word_idx,
            &words,
        )?;
        let descriptor_words =
            ActivationRestrictionCompatWords::new(descriptor_span).to_word_refs();
        let one_or_more = trigger_pattern_accepts(&descriptor_words, ONE_OR_MORE_PREFIX_PATTERN);
        let counter_type = trigger_counter_type_from_descriptor(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 2;
        let object_tokens = trigger_counter_recipient_tokens(tokens, object_word_start, &words)?;
        let (object_tokens, include_players) = split_counter_recipient_or_player(&object_tokens);
        let filter = parse_object_filter_lexed(object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: Some(source_controller),
            one_or_more,
            include_players,
        });
    }

    if let Some(get_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Get)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..get_word_idx])
        && words.get(get_word_idx + 1..).is_some_and(|tail| {
            trigger_pattern_accepts(tail, PLAYER_GETS_ONE_OR_MORE_ENERGY_TAIL_PATTERN)
        })
    {
        return Ok(TriggerSpec::PlayerGetsCounters {
            player,
            counter_type: Some(CounterType::Energy),
            one_or_more: true,
        });
    }

    if let Some(get_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Get)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..get_word_idx])
        && let Some(counter_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Counter)
        && counter_word_idx > get_word_idx
    {
        let descriptor_word_start = get_word_idx + 1;
        let (descriptor_span, counter_descriptor_tokens) = trigger_counter_descriptor_span(
            tokens,
            descriptor_word_start,
            counter_word_idx,
            &words,
        )?;
        let descriptor_words =
            ActivationRestrictionCompatWords::new(descriptor_span).to_word_refs();
        let one_or_more = trigger_pattern_accepts(&descriptor_words, ONE_OR_MORE_PREFIX_PATTERN);
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        return Ok(TriggerSpec::PlayerGetsCounters {
            player,
            counter_type,
            one_or_more,
        });
    }

    if trigger_pattern_accepts(&words, PLAYERS_FINISH_VOTING_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
            source_filter: None,
            during_your_turn: false,
        });
    }

    if trigger_pattern_accepts(&words, YOU_CYCLE_THIS_CARD_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            player: PlayerFilter::You,
        });
    }

    if trigger_pattern_accepts(&words, YOU_CYCLE_OR_DISCARD_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Cycle,
                player: PlayerFilter::You,
                source_filter: None,
                during_your_turn: false,
            }),
            Box::new(TriggerSpec::PlayerDiscardsCard {
                player: PlayerFilter::You,
                filter: None,
                cause_controller: None,
                effect_like_only: false,
                one_or_more: false,
            }),
        ));
    }

    let (crime_words, during_your_turn) =
        if trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX) {
            (&words[..words.len() - 3], true)
        } else {
            (words.as_slice(), false)
        };
    if trigger_pattern_accepts(crime_words, YOU_COMMIT_CRIME_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::You,
            source_filter: None,
            during_your_turn,
        });
    }

    if trigger_pattern_accepts(crime_words, OPPONENT_COMMITS_CRIME_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Opponent,
            source_filter: None,
            during_your_turn,
        });
    }

    if trigger_pattern_accepts(crime_words, PLAYER_COMMITS_CRIME_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Any,
            source_filter: None,
            during_your_turn,
        });
    }

    if let Some(trigger) = trigger_grammar::parse_fully_unlock_room_trigger(tokens) {
        return Ok(TriggerSpec::KeywordAction {
            action: trigger.action,
            player: trigger.player,
            source_filter: Some(trigger.source_filter),
            during_your_turn: false,
        });
    }

    if trigger_pattern_accepts(&words, YOU_UNLOCK_THIS_DOOR_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::UnlockDoor,
            player: PlayerFilter::You,
        });
    }

    if trigger_pattern_accepts(&words, THIS_CARD_BECOMES_PLOTTED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Plot,
            player: PlayerFilter::You,
        });
    }

    if words.len() == 3
        && trigger_pattern_accepts(&words, YOU_EXPEND_TRIGGER_PREFIX)
        && let Some(amount) = parse_named_number(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::You,
            amount,
        });
    }

    if words.len() == 4
        && trigger_pattern_accepts(&words, OPPONENT_EXPENDS_WITH_ARTICLE_TRIGGER_PREFIX)
        && let Some(amount) = parse_named_number(words[3])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.len() == 3
        && trigger_pattern_accepts(&words, OPPONENT_EXPENDS_TRIGGER_PREFIX)
        && let Some(amount) = parse_named_number(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if trigger_pattern_accepts(&words, THE_RING_TEMPTS_YOU_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::RingTemptsYou,
            player: PlayerFilter::You,
            source_filter: None,
            during_your_turn: false,
        });
    }

    if trigger_pattern_accepts(&words, CHAOS_ENSUES_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ChaosEnsues,
            player: PlayerFilter::Any,
            source_filter: None,
            during_your_turn: false,
        });
    }

    if trigger_pattern_accepts(&words, YOU_ENCOUNTER_PHENOMENON_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::EncounterPhenomenon,
            player: PlayerFilter::You,
        });
    }

    if trigger_pattern_accepts(&words, YOU_SET_THIS_SCHEME_IN_MOTION_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::SetSchemeInMotion,
            player: PlayerFilter::You,
        });
    }

    if let Some(cycle_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Cycle)
    {
        let subject_words = &words[..cycle_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let tail_words = &words[cycle_word_idx + 1..];
            if trigger_pattern_accepts(tail_words, CYCLE_CARD_TAIL_PATTERN) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: None,
                    during_your_turn: false,
                });
            }
            if trigger_pattern_accepts(tail_words, CYCLE_ANOTHER_CARD_TAIL_PATTERN) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: Some(ObjectFilter::default().other()),
                    during_your_turn: false,
                });
            }
        }
    }

    if let Some(exert_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Exert)
    {
        let subject = &words[..exert_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[exert_word_idx + 1..];
            if trigger_pattern_accepts(tail, EXERT_CREATURE_TAIL_PATTERN) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Exert,
                    player,
                    source_filter: Some(ObjectFilter::creature()),
                    during_your_turn: false,
                });
            }
        }
    }

    let (core_words, during_your_main_phase) =
        if trigger_pattern_accepts(&words, DURING_YOUR_MAIN_PHASE_SUFFIX_PATTERN) {
            (
                &words[..words.len() - DURING_YOUR_MAIN_PHASE_SUFFIX.len()],
                true,
            )
        } else {
            (words.as_slice(), false)
        };
    if let Some(saddle_word_idx) =
        trigger_keyword_action_word(core_words, crate::events::KeywordActionKind::Saddle)
        && let Some(or_word_idx) =
            trigger_atom_word(&core_words[saddle_word_idx + 1..], TriggerClauseAtom::Or)
                .map(|idx| saddle_word_idx + 1 + idx)
        && let Some(crew_word_idx) = trigger_keyword_action_word(
            &core_words[or_word_idx + 1..],
            crate::events::KeywordActionKind::Crew,
        )
        .map(|idx| or_word_idx + 1 + idx)
    {
        let subject_words = &core_words[..saddle_word_idx];
        let saddle_tail = &core_words[saddle_word_idx + 1..or_word_idx];
        let crew_tail = &core_words[crew_word_idx + 1..];
        if is_source_reference_words(subject_words)
            && trigger_pattern_accepts(saddle_tail, SADDLE_MOUNT_TAIL_PATTERN)
            && trigger_pattern_accepts(crew_tail, CREW_VEHICLE_TAIL_PATTERN)
        {
            let source_filter = source_reference_surface_for_words(subject_words)
                .or_else(|| this_source_surface_for_words(subject_words))
                .map(ObjectFilter::source_with_surface)
                .unwrap_or_else(ObjectFilter::source);
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::KeywordActionTaggedObject {
                    action: crate::events::KeywordActionKind::Saddle,
                    player: PlayerFilter::Any,
                    source_filter: source_filter.clone(),
                    object_tag: crate::tag::CompilerReferenceTag::It.bind(),
                    object_filter: ObjectFilter::default()
                        .in_zone(Zone::Battlefield)
                        .with_subtype(Subtype::Mount),
                    during_your_main_phase,
                }),
                Box::new(TriggerSpec::KeywordActionTaggedObject {
                    action: crate::events::KeywordActionKind::Crew,
                    player: PlayerFilter::Any,
                    source_filter,
                    object_tag: crate::tag::CompilerReferenceTag::It.bind(),
                    object_filter: ObjectFilter::default()
                        .in_zone(Zone::Battlefield)
                        .with_subtype(Subtype::Vehicle),
                    during_your_main_phase,
                }),
            ));
        }
    }

    if let Some(crew_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Crew)
    {
        let subject_words = &words[..crew_word_idx];
        let source_becomes_crewed = subject_words.last().is_some_and(|word| *word == "becomes")
            && is_source_reference_words(&subject_words[..subject_words.len().saturating_sub(1)]);
        let source_filter = if source_becomes_crewed {
            Some(ObjectFilter::default())
        } else if is_source_reference_words(subject_words) {
            Some(ObjectFilter::source())
        } else {
            let subject_end = word_view
                .token_index_after_words(crew_word_idx)
                .unwrap_or(crew_word_idx);
            parse_trigger_subject_filter_lexed(&tokens[..subject_end])?
        };
        if let Some(source_filter) = source_filter {
            let tail_start = word_view
                .token_index_after_words(crew_word_idx + 1)
                .unwrap_or(tokens.len());
            let tail_words = &words[crew_word_idx + 1..];
            let object_filter = if source_becomes_crewed {
                ObjectFilter::source().with_subtype(Subtype::Vehicle)
            } else if tail_words.is_empty()
                || trigger_pattern_accepts(tail_words, CREW_VEHICLE_TAIL_PATTERN)
            {
                ObjectFilter::default().with_subtype(Subtype::Vehicle)
            } else {
                let tail_tokens = trim_commas(tokens.get(tail_start..).unwrap_or_default());
                parse_object_filter_lexed(&tail_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported crew object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::KeywordActionTaggedObject {
                action: crate::events::KeywordActionKind::Crew,
                player: PlayerFilter::Any,
                source_filter,
                object_tag: crate::tag::CompilerReferenceTag::It.bind(),
                object_filter,
                during_your_main_phase: false,
            });
        }
    }

    if let Some(explore_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Explore)
    {
        let subject_tokens = &tokens[..explore_word_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            let tail = &words[explore_word_idx + 1..];
            let revealed_filter = if tail.is_empty() {
                None
            } else if trigger_pattern_accepts(tail, EXPLORE_LAND_CARD_TAIL_PATTERN) {
                Some(ObjectFilter::default().with_type(crate::types::CardType::Land))
            } else if trigger_pattern_accepts(tail, EXPLORE_NONLAND_CARD_TAIL_PATTERN) {
                Some(ObjectFilter::default().without_type(crate::types::CardType::Land))
            } else {
                None
            };
            return Ok(match revealed_filter {
                Some(object_filter) => TriggerSpec::KeywordActionTaggedObject {
                    action: crate::events::KeywordActionKind::Explore,
                    player: PlayerFilter::Any,
                    source_filter: filter,
                    object_tag: crate::tag::CompilerReferenceTag::PublicRevealed.bind(),
                    object_filter,
                    during_your_main_phase: false,
                },
                None if tail.is_empty() => TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Explore,
                    player: PlayerFilter::Any,
                    source_filter: Some(filter),
                    during_your_turn: false,
                },
                None => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported explore trigger tail in trigger clause (clause: '{}')",
                        words.join(" ")
                    )));
                }
            });
        }
    }

    if let Some(fight_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Fight)
    {
        let subject_tokens = &tokens[..fight_word_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)?
            && words[fight_word_idx + 1..].is_empty()
        {
            return Ok(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Fight,
                player: PlayerFilter::Any,
                source_filter: Some(filter),
                during_your_turn: false,
            });
        }
    }

    if let Some(put_token_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Put) {
        let subject_words =
            ActivationRestrictionCompatWords::new(&tokens[..put_token_idx]).to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words)
            && let Some(sticker) =
                crate::grammar::effects::combat_damage_family_shapes::parse_put_sticker_shape(
                    &tokens[put_token_idx..],
                )
        {
            let source_filter =
                parse_object_filter_lexed(sticker.target_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported sticker recipient in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            return Ok(TriggerSpec::KeywordAction {
                action: sticker.action,
                player,
                source_filter: Some(source_filter),
                during_your_turn: false,
            });
        }
    }

    let becomes_tapped_words = if trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX) {
        &words[..words.len().saturating_sub(3)]
    } else {
        words.as_slice()
    };

    if trigger_pattern_accepts(becomes_tapped_words, BECOMES_TAPPED_TRIGGER_SUFFIX)
        && let Some(becomes_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Becomes)
    {
        let subject_tokens = &tokens[..becomes_idx];
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::PermanentBecomesTapped(filter),
            None => TriggerSpec::ThisBecomesTapped,
        });
    }

    if trigger_pattern_accepts(becomes_tapped_words, THIS_BECOMES_TAPPED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if trigger_pattern_accepts(&words, THIS_BECOMES_UNTAPPED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if trigger_pattern_accepts(&words, THIS_BECOMES_MONSTROUS_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }
    if words.len() == 5
        && trigger_word_at_accepts_pattern(&words, 0, THIS_WORD_PATTERN)
        && words[1].eq_ignore_ascii_case("class")
        && trigger_word_at_accepts_pattern(&words, 2, BECOMES_WORD_PATTERN)
        && words[3].eq_ignore_ascii_case("level")
        && parse_named_number(words[4]).is_some()
    {
        return Ok(TriggerSpec::CounterPutOn {
            filter: ObjectFilter::source(),
            counter_type: Some(CounterType::Level),
            source_controller: None,
            one_or_more: false,
            include_players: false,
        });
    }
    if trigger_pattern_accepts(&words, BECOMES_MONSTROUS_TRIGGER_SUFFIX)
        && words.len() > 2
        && source_reference_surface_for_words(&words[..words.len() - 2]).is_some()
    {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if trigger_pattern_accepts(&words, THIS_MUTATES_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisMutates);
    }
    if trigger_pattern_accepts(&words, MUTATES_TRIGGER_SUFFIX)
        && words.len() > 1
        && source_reference_surface_for_words(&words[..words.len() - 1]).is_some()
    {
        return Ok(TriggerSpec::ThisMutates);
    }

    if trigger_pattern_accepts(&words, THIS_TURNED_FACE_UP_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisTurnedFaceUp);
    }

    if trigger_pattern_accepts(&words, TURNED_FACE_UP_TRIGGER_SUFFIX) {
        let subject_tokens = trigger_word_token_start(tokens, words.len().saturating_sub(4))
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::TurnedFaceUp(filter),
            None => TriggerSpec::ThisTurnedFaceUp,
        });
    }

    if let Some(becomes_idx) = trigger_atom_word(&words, TriggerClauseAtom::Becomes)
        && trigger_pattern_accepts(&words[becomes_idx + 1..], BECOMES_TARGET_OF_PREFIX_PATTERN)
    {
        let subject_words = &words[..becomes_idx];
        let subject_tokens = trigger_word_token_start(tokens, becomes_idx)
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        let subject_filter = parse_trigger_subject_filter_lexed(subject_tokens)?;
        let subject_is_source =
            subject_words.is_empty() || is_source_reference_words(subject_words);
        if subject_is_source {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_controller_tail(tail_words) {
                let mut spell_filter = ObjectFilter::spell();
                spell_filter.controller = Some(source_controller);
                return Ok(TriggerSpec::ThisBecomesTargetedBySpell(spell_filter));
            }
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words) {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: ObjectFilter::source(),
                    source_controller,
                });
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_ABILITY_TARGET_TAIL_PATTERN) {
                return Ok(TriggerSpec::ThisBecomesTargeted);
            }
            if trigger_pattern_accepts(tail_words, ONLY_IT_ABILITY_TARGET_TAIL_PATTERN) {
                let mut ability_filter = ObjectFilter::ability();
                ability_filter.target_count = Some(crate::effect::ChoiceCount::exactly(1));
                ability_filter.targets_only_object = Some(Box::new(ObjectFilter::source()));
                return Ok(TriggerSpec::ThisBecomesTargetedByStackObject(
                    ability_filter,
                ));
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_SPELLS_SUFFIX_PATTERN) {
                let tail_token_start =
                    trigger_word_token_start(tokens, tail_word_start).unwrap_or(tokens.len());
                let spell_filter_tokens = trim_commas(&tokens[tail_token_start..]);
                let spell_filter =
                    parse_object_filter_lexed(&spell_filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported spell filter in becomes-targeted trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                return Ok(TriggerSpec::ThisBecomesTargetedBySpell(spell_filter));
            }
        } else {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_controller_tail(tail_words)
                && let Some(target) = subject_filter.clone()
            {
                let mut spell_filter = ObjectFilter::spell();
                spell_filter.controller = Some(source_controller);
                return Ok(TriggerSpec::BecomesTargetedByStackObject {
                    target,
                    stack_object: spell_filter,
                });
            }
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words)
                && let Some(subject) =
                    trigger_grammar::parse_you_or_controlled_object_subject_words(subject_words)
            {
                return Ok(
                    TriggerSpec::PlayerOrObjectBecomesTargetedBySourceController {
                        player: subject.player,
                        object: subject.filter,
                        source_controller,
                    },
                );
            }
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words)
                && let Some(filter) = subject_filter.clone()
            {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: filter,
                    source_controller,
                });
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_ABILITY_TARGET_TAIL_PATTERN)
                && let Some(filter) = subject_filter
            {
                return Ok(TriggerSpec::BecomesTargeted(filter));
            }
            if trigger_pattern_accepts(tail_words, BACKUP_ABILITY_TARGET_TAIL_PATTERN)
                && let Some(filter) = subject_filter
            {
                let ability_filter = ObjectFilter::ability().with_ability_marker("backup");
                return Ok(TriggerSpec::BecomesTargetedByStackObject {
                    target: filter,
                    stack_object: ability_filter,
                });
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_SPELLS_SUFFIX_PATTERN)
                && let Some(filter) = subject_filter
            {
                let tail_token_start =
                    trigger_word_token_start(tokens, tail_word_start).unwrap_or(tokens.len());
                let spell_filter_tokens = trim_commas(&tokens[tail_token_start..]);
                let spell_filter =
                    parse_object_filter_lexed(&spell_filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported spell filter in becomes-targeted trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                return Ok(TriggerSpec::BecomesTargetedByStackObject {
                    target: filter,
                    stack_object: spell_filter,
                });
            }
        }
    }

    if let Some((recipient_end_word, source_start_word)) = passive_damage_by_word_span(&words) {
        let recipient_end_token =
            trigger_word_token_start(tokens, recipient_end_word).unwrap_or(tokens.len());
        let source_start_token =
            trigger_word_token_start(tokens, source_start_word).unwrap_or(tokens.len());
        let recipient_tokens = trim_edge_punctuation_tokens(&tokens[..recipient_end_token]);
        let source_tokens = trim_edge_punctuation_tokens(&tokens[source_start_token..]);
        if recipient_tokens.is_empty() || source_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "incomplete passive damage trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let source = parse_passive_damage_source_filter(source_tokens, &words)?;
        let recipient_words =
            ActivationRestrictionCompatWords::new(recipient_tokens).to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&recipient_words) {
            return Ok(TriggerSpec::DealsDamageToPlayer {
                source,
                player,
                source_surface: crate::triggers::DamageSourceSurface::PassiveBy,
            });
        }
        let target = parse_object_filter_lexed(recipient_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported passive damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::DealsDamageTo {
            source,
            target,
            source_surface: crate::triggers::DamageSourceSurface::PassiveBy,
        });
    }

    if let Some(is_word_idx) = dealt_excess_noncombat_damage_subject_word_idx(&words) {
        let is_token_idx = trigger_word_token_start(tokens, is_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..is_token_idx];
        if let Some(mut filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            if crate::word_primitives::parse_sequence_prefix(
                &words[..is_word_idx],
                &["one", "or", "more"],
            ) {
                filter.set_union_one_or_more(true);
            }
            return Ok(TriggerSpec::IsDealtExcessNoncombatDamage(filter));
        }
    }

    if let Some((is_word_idx, dealt_combat_damage)) = dealt_damage_suffix_subject_word_idx(&words)
        && !trigger_pattern_accepts(&words, SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX)
    {
        let is_token_idx = trigger_word_token_start(tokens, is_word_idx).unwrap_or(tokens.len());
        if is_word_idx == 0
            && words.first().is_some_and(|word| {
                trigger_word_accepts_pattern(word, YOU_CONTRACTION_WORD_PATTERN)
            })
        {
            if dealt_combat_damage {
                return Ok(TriggerSpec::DealsCombatDamageToPlayer {
                    source: ObjectFilter::default(),
                    player: PlayerFilter::You,
                });
            }
            return Ok(TriggerSpec::DealsDamageToPlayer {
                source: ObjectFilter::default(),
                player: PlayerFilter::You,
                source_surface: crate::triggers::DamageSourceSurface::Filter,
            });
        }
        let subject_tokens = &tokens[..is_token_idx];
        if let Some(player) = trigger_subject_player_selector_lexed(subject_tokens) {
            if dealt_combat_damage {
                return Ok(TriggerSpec::DealsCombatDamageToPlayer {
                    source: ObjectFilter::default(),
                    player,
                });
            }
            return Ok(TriggerSpec::DealsDamageToPlayer {
                source: ObjectFilter::default(),
                player,
                source_surface: crate::triggers::DamageSourceSurface::Filter,
            });
        }
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            if dealt_combat_damage {
                return Ok(TriggerSpec::IsDealtCombatDamage(filter));
            }
            return Ok(TriggerSpec::IsDealtDamage(filter));
        }
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX) {
        if trigger_pattern_accepts(&words, SOURCE_DEALT_COMBAT_DAMAGE_TRIGGER_PREFIX) {
            return Ok(TriggerSpec::ThisIsDealtCombatDamage);
        }
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if crate::word_primitives::parse_sequence_suffix(
        &words,
        &["causes", "you", "to", "gain", "life"],
    ) {
        let cause_word_idx = words.len().saturating_sub(5);
        let cause_token_idx =
            trigger_word_token_start(tokens, cause_word_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing causal source boundary in life-gain trigger (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let source_tokens = trim_commas(&tokens[..cause_token_idx]);
        let source = parse_object_filter_lexed(&source_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported causal source filter in life-gain trigger (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::YouGainLifeCausedBy(source));
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALS_TRIGGER_PREFIX)
        && let Some(deals_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Deal)
        && let Some(damage_idx_rel) =
            trigger_atom_token(&tokens[deals_idx + 1..], TriggerClauseAtom::Damage)
    {
        let damage_idx = deals_idx + 1 + damage_idx_rel;
        if let Some(to_idx_rel) =
            trigger_atom_token(&tokens[damage_idx + 1..], TriggerClauseAtom::To)
        {
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_tokens = trim_commas(&tokens[deals_idx + 1..damage_idx]);
            if !amount_tokens
                .first()
                .is_some_and(|token| token_matches_clause_shape(token, COMBAT_WORD_PATTERN))
            {
                let amount_view = ActivationRestrictionCompatWords::new(&amount_tokens);
                let amount_words = amount_view.to_word_refs();
                if let Some((amount, _)) =
                    parse_filter_comparison_tokens("damage amount", &amount_words, &words)?
                {
                    let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
                    let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
                    let target_words = target_view.to_word_refs();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                            player,
                            amount: Some(amount),
                        });
                    }
                }
            }
        }
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALS_DAMAGE_TO_TRIGGER_PREFIX)
        && let Some(to_idx) = trigger_atom_token(tokens, TriggerClauseAtom::To)
    {
        let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
        let target_one_or_more = has_leading_one_or_more(&target_tokens);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
        let target_words = target_view.to_word_refs();
        if let Some((player, target_filter, player_first)) =
            parse_player_or_object_damage_recipient(&target_tokens)
        {
            let player_trigger = TriggerSpec::ThisDealsDamageToPlayer {
                player,
                amount: None,
            };
            let object_trigger = TriggerSpec::ThisDealsDamageTo(target_filter);
            return Ok(if player_first {
                TriggerSpec::Either(Box::new(player_trigger), Box::new(object_trigger))
            } else {
                TriggerSpec::Either(Box::new(object_trigger), Box::new(player_trigger))
            });
        }
        if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
            return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                player,
                amount: None,
            });
        }
        let mut target_filter =
            parse_object_filter_lexed(strip_leading_one_or_more_lexed(&target_tokens), false)
                .map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported damage recipient filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
        target_filter.set_union_one_or_more(target_one_or_more);
        return Ok(TriggerSpec::ThisDealsDamageTo(target_filter));
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALS_DAMAGE_TRIGGER_PREFIX) {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    let has_deal = trigger_atom_word(&words, TriggerClauseAtom::Deal).is_some();
    if has_deal
        && trigger_pattern_accepts(&words, DAMAGE_WORD_PATTERN)
        && let Some(deals_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Deal)
    {
        let subject_tokens = &tokens[..deals_idx];
        if let Some(damage_idx_rel) =
            trigger_atom_token(&tokens[deals_idx + 1..], TriggerClauseAtom::Damage)
            && let Some(to_idx_rel) = trigger_atom_token(
                &tokens[deals_idx + 1 + damage_idx_rel + 1..],
                TriggerClauseAtom::To,
            )
        {
            let damage_idx = deals_idx + 1 + damage_idx_rel;
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_words =
                ActivationRestrictionCompatWords::new(&tokens[deals_idx + 1..damage_idx])
                    .to_word_refs();
            let exact_amount = parse_leading_exactly_quantifier(&tokens[deals_idx + 1..damage_idx])
                .and_then(|(amount, remainder)| remainder.is_empty().then_some(amount));
            let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
            let (target_tokens_without_turn, during_turn) = if let Some(without_turn) =
                crate::grammar::primitives::strip_lexed_suffix_phrase(
                    &target_tokens,
                    &["during", "your", "turn"],
                ) {
                (without_turn, Some(PlayerFilter::You))
            } else {
                (target_tokens.as_slice(), None)
            };
            let target_one_or_more = has_leading_one_or_more(target_tokens_without_turn);
            let target_view = ActivationRestrictionCompatWords::new(target_tokens_without_turn);
            let target_words = target_view.to_word_refs();
            if trigger_pattern_accepts(&amount_words, NONCOMBAT_DAMAGE_AMOUNT_PATTERN)
                && let Some(player) = parse_trigger_subject_player_filter(&target_words)
                && let Some((source, source_surface)) =
                    parse_damage_source_trigger_filter_lexed(subject_tokens)?
            {
                return Ok(TriggerSpec::DealsNoncombatDamageToPlayer {
                    source,
                    player,
                    source_surface,
                    damaged_player_one_or_more: target_one_or_more,
                    during_turn,
                });
            }
            if let Some((player, target, player_first)) =
                parse_player_or_object_damage_recipient(&target_tokens)
                && let Some((source, source_surface)) =
                    parse_damage_source_trigger_filter_lexed(subject_tokens)?
            {
                if let Some(amount) = exact_amount {
                    return Ok(TriggerSpec::DealsExactDamageToObjectOrPlayer {
                        source,
                        object: target,
                        player,
                        player_first,
                        amount,
                        source_surface,
                    });
                }
                let player_trigger = TriggerSpec::DealsDamageToPlayer {
                    source: source.clone(),
                    player,
                    source_surface,
                };
                let object_trigger = TriggerSpec::DealsDamageTo {
                    source,
                    target,
                    source_surface,
                };
                return Ok(if player_first {
                    TriggerSpec::Either(Box::new(player_trigger), Box::new(object_trigger))
                } else {
                    TriggerSpec::Either(Box::new(object_trigger), Box::new(player_trigger))
                });
            }
            if let Some(player) = parse_trigger_subject_player_filter(&target_words)
                && let Some((source, source_surface)) =
                    parse_damage_source_trigger_filter_lexed(subject_tokens)?
            {
                return Ok(TriggerSpec::DealsDamageToPlayer {
                    source,
                    player,
                    source_surface,
                });
            }
            // A self-name subject ("Lu Xun deals damage to an opponent")
            // yields no source filter; keep the recipient instead of
            // falling through to the recipient-less ThisDealsDamage.
            if let Some(player) = parse_trigger_subject_player_filter(&target_words)
                && parse_damage_source_trigger_filter_lexed(subject_tokens)?.is_none()
            {
                return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                    player,
                    amount: None,
                });
            }
            if let Ok(mut target) =
                parse_object_filter_lexed(strip_leading_one_or_more_lexed(&target_tokens), false)
                && let Some((source, source_surface)) =
                    parse_damage_source_trigger_filter_lexed(subject_tokens)?
            {
                target.set_union_one_or_more(target_one_or_more);
                return Ok(TriggerSpec::DealsDamageTo {
                    source,
                    target,
                    source_surface,
                });
            }
            if let Ok(mut target) =
                parse_object_filter_lexed(strip_leading_one_or_more_lexed(&target_tokens), false)
                && parse_damage_source_trigger_filter_lexed(subject_tokens)?.is_none()
            {
                target.set_union_one_or_more(target_one_or_more);
                return Ok(TriggerSpec::ThisDealsDamageTo(target));
            }
        }
        return Ok(
            match parse_damage_source_trigger_filter_lexed(subject_tokens)? {
                Some((source, source_surface)) => TriggerSpec::DealsDamage {
                    source,
                    source_surface,
                },
                None => TriggerSpec::ThisDealsDamage,
            },
        );
    }

    if trigger_pattern_accepts(&words, YOU_GAIN_LIFE_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.len() >= 6
        && trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX)
        && trigger_pattern_accepts(&words[..words.len() - 3], YOU_GAIN_LIFE_PREFIX_PATTERN)
    {
        return Ok(TriggerSpec::YouGainLifeDuringTurn(PlayerFilter::You));
    }

    if let Some(amount) = trigger_grammar::parse_opponents_each_lose_exact_life_words(&words) {
        return Ok(TriggerSpec::OpponentsEachLoseExactLife { amount });
    }

    if let Some(clause) = trigger_grammar::parse_players_lose_life_one_or_more_clause(tokens) {
        return Ok(TriggerSpec::PlayersLoseLifeOneOrMore(clause.player));
    }

    if trigger_pattern_accepts(&words, LOSE_LIFE_TRIGGER_SUFFIX) {
        let subject = &words[..words.len().saturating_sub(2)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLife(player));
        }
    }

    if trigger_pattern_accepts(&words, LOSE_GAME_TRIGGER_SUFFIX) {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesGame(player));
        }
    }

    if words.len() >= 5
        && trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX)
        && trigger_pattern_accepts(&words[..words.len() - 3], LOSE_LIFE_TRIGGER_SUFFIX)
    {
        let subject = &words[..words.len() - 5];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLifeDuringTurn {
                player,
                during_turn: PlayerFilter::You,
            });
        }
    }

    if let Some(draw_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Draw) {
        let subject = &words[..draw_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[draw_word_idx + 1..];
            if let Some(during_turn) =
                trigger_grammar::parse_not_during_turn_draw_suffix_words(tail)
            {
                return Ok(TriggerSpec::PlayerDrawsCardNotDuringTurn {
                    player,
                    during_turn,
                });
            }
            if has_draw_except_first_in_draw_step_pattern(tail) {
                return Ok(TriggerSpec::PlayerDrawsCardExceptFirstInDrawStep(player));
            }
            let card_numbers = parse_draw_numbers_each_turn(tail);
            match card_numbers.as_slice() {
                [card_number] => {
                    return Ok(TriggerSpec::PlayerDrawsNthCardEachTurn {
                        player,
                        card_number: *card_number,
                    });
                }
                [_, _, ..] => {
                    return Ok(TriggerSpec::PlayerDrawsNumberedCardsEachTurn {
                        player,
                        card_numbers,
                    });
                }
                _ => {}
            }
        }
    }

    if trigger_pattern_accepts(&words, DRAW_A_CARD_TRIGGER_SUFFIX) {
        let subject = &words[..words.len().saturating_sub(3)];
        if trigger_pattern_accepts(subject, YOU_DRAW_CARD_TRIGGER_SUBJECT_PATTERN) {
            return Ok(TriggerSpec::YouDrawCard);
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerDrawsCard(player));
        }
    }

    if trigger_pattern_accepts(&words, OPPONENT_EFFECT_DISCARDS_THIS_CARD_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::PlayerDiscardsCard {
            player: PlayerFilter::You,
            filter: Some(ObjectFilter::source()),
            cause_controller: Some(PlayerFilter::Opponent),
            effect_like_only: true,
            one_or_more: false,
        });
    }

    if let Some(discard_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Discard)
        && let Some(discard_token_idx) = trigger_word_token_start(tokens, discard_word_idx)
    {
        let subject_words = &words[..discard_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words)
            && let Ok(filter) =
                parse_discard_trigger_card_filter(&tokens[discard_token_idx + 1..], &words)
        {
            let tail_words = ActivationRestrictionCompatWords::new(
                tokens.get(discard_token_idx + 1..).unwrap_or_default(),
            )
            .to_word_refs();
            let one_or_more = trigger_grammar::find_trigger_surface_window(
                &tail_words,
                3,
                ONE_OR_MORE_QUANTIFIER_PATTERN,
            )
            .is_some();
            return Ok(TriggerSpec::PlayerDiscardsCard {
                player,
                filter,
                cause_controller: None,
                effect_like_only: false,
                one_or_more,
            });
        }
    }

    if let Some(reveal_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Reveal)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..reveal_word_idx])
    {
        let mut tail_tokens = trim_commas(
            &tokens
                [trigger_word_token_start(tokens, reveal_word_idx + 1).unwrap_or(tokens.len())..],
        );
        let tail_view = ActivationRestrictionCompatWords::new(&tail_tokens);
        let tail_words = tail_view.to_word_refs();
        let from_source = trigger_pattern_accepts(&tail_words, THIS_WAY_REVEAL_TAIL_PATTERN);
        if from_source {
            let cutoff = trigger_word_token_start(&tail_tokens, tail_words.len().saturating_sub(2))
                .unwrap_or(tail_tokens.len());
            tail_tokens = trim_commas(&tail_tokens[..cutoff]);
        }
        if !tail_tokens.is_empty()
            && let Ok(mut filter) = parse_object_filter_lexed(&tail_tokens, false)
        {
            filter.zone = None;
            return Ok(TriggerSpec::PlayerRevealsCard {
                player,
                filter,
                from_source,
            });
        }
    }

    if let Some(sacrifice_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Sacrifice)
        && let Some(sacrifice_token_idx) = trigger_word_token_start(tokens, sacrifice_word_idx)
    {
        let subject_words = &words[..sacrifice_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let mut filter_tokens = &tokens[sacrifice_token_idx + 1..];
            let filter_word_view = ActivationRestrictionCompatWords::new(filter_tokens);
            let filter_words = filter_word_view.to_word_refs();
            let one_or_more = trigger_grammar::find_trigger_surface_window(
                &filter_words,
                3,
                ONE_OR_MORE_QUANTIFIER_PATTERN,
            )
            .is_some();
            let mut other = false;
            if filter_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                filter_tokens = &filter_tokens[1..];
            }

            let filter = if filter_tokens.is_empty() {
                let mut filter = ObjectFilter::permanent();
                if other {
                    filter.other = true;
                }
                filter
            } else if filter_tokens
                .first()
                .is_some_and(|token| token_matches_clause_shape(token, THIS_OR_IT_PATTERN))
            {
                let filter_word_view = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_word_view.to_word_refs();
                let mut filter = ObjectFilter::source();
                let is_artifact =
                    trigger_pattern_accepts(&filter_words, SOURCE_ARTIFACT_WORD_PATTERN);
                let is_creature =
                    trigger_pattern_accepts(&filter_words, SOURCE_CREATURE_WORD_PATTERN);
                let is_enchantment =
                    trigger_pattern_accepts(&filter_words, SOURCE_ENCHANTMENT_WORD_PATTERN);
                let is_land = trigger_pattern_accepts(&filter_words, SOURCE_LAND_WORD_PATTERN);
                let is_planeswalker =
                    trigger_pattern_accepts(&filter_words, SOURCE_PLANESWALKER_WORD_PATTERN);
                if is_artifact {
                    filter = filter.with_type(CardType::Artifact);
                } else if is_creature {
                    filter = filter.with_type(CardType::Creature);
                } else if is_enchantment {
                    filter = filter.with_type(CardType::Enchantment);
                } else if is_land {
                    filter = filter.with_type(CardType::Land);
                } else if is_planeswalker {
                    filter = filter.with_type(CardType::Planeswalker);
                }
                filter
            } else {
                parse_object_filter_lexed(filter_tokens, other).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported sacrifice trigger filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::PlayerSacrifices {
                player,
                filter,
                one_or_more,
            });
        }
    }

    if let Some(roll_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Roll) {
        let subject_words = &words[..roll_word_idx];
        let result_words = &words[roll_word_idx + 1..];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            use crate::grammar::trigger_clauses::RollResultShape;
            match trigger_grammar::parse_roll_result_words(result_words) {
                Some(RollResultShape::HighestNatural) => {
                    return Ok(TriggerSpec::PlayerRollsHighestNaturalResult { player });
                }
                Some(RollResultShape::Fixed(result)) => {
                    return Ok(TriggerSpec::PlayerRollsResult { player, result });
                }
                Some(RollResultShape::UnspecifiedDie) => {
                    return Ok(TriggerSpec::PlayerRollsDie {
                        player,
                        one_or_more: false,
                    });
                }
                Some(RollResultShape::OneOrMoreDice) => {
                    return Ok(TriggerSpec::PlayerRollsDie {
                        player,
                        one_or_more: true,
                    });
                }
                None => {}
            }
        }
    }

    if let Some(result_idx) = crate::slice_primitives::select_position(&words, |word| {
        word.eq_ignore_ascii_case("win")
            || word.eq_ignore_ascii_case("wins")
            || word.eq_ignore_ascii_case("lose")
            || word.eq_ignore_ascii_case("loses")
    }) && crate::word_primitives::parse_sequence_complete(
        &words[result_idx + 1..],
        &["a", "coin", "flip"],
    ) && let Some(player) = parse_trigger_subject_player_filter(&words[..result_idx])
    {
        let won = words[result_idx].eq_ignore_ascii_case("win")
            || words[result_idx].eq_ignore_ascii_case("wins");
        return Ok(TriggerSpec::PlayerCoinFlipResult { player, won });
    }

    if trigger_pattern_accepts(
        &words,
        clause_shape!(exact & ["this", "creature", "enlists", "a", "creature"]),
    ) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Enlist,
            player: PlayerFilter::You,
        });
    }

    // "Manifest dread" is one indivisible keyword action. Keep this exact
    // two-word shape ahead of the generic final-word dispatch so an ordinary
    // manifest action can never satisfy a manifest-dread observer.
    if trigger_pattern_accepts(&words, YOU_MANIFEST_DREAD_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ManifestDread,
            player: PlayerFilter::You,
            source_filter: None,
            during_your_turn: false,
        });
    }

    // Winning is part of the observed clash result, not a second independent
    // action. Route this surface through the same winner-aware matcher as
    // "you win a clash" so losing the clash can never satisfy the trigger.
    if trigger_pattern_accepts(&words, YOU_CLASH_AND_WIN_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::WinsClash {
            player: PlayerFilter::You,
            surface: ironsmith_core::ClashWinTriggerSurface::ClashAndWin,
        });
    }

    if let Some(last_word) = words.last().copied()
        && let Some(action) = crate::events::KeywordActionKind::from_trigger_word(last_word)
    {
        let subject = &words[..words.len().saturating_sub(1)];
        if is_source_reference_words(subject) {
            return Ok(TriggerSpec::KeywordActionFromSource {
                action,
                player: PlayerFilter::You,
            });
        }
        if subject.len() > 2 && is_source_reference_words(&subject[..2]) {
            let trailing_ok = subject[2..].iter().all(|word| {
                trigger_word_accepts_pattern(word, SOURCE_KEYWORD_ACTION_TRAILING_WORD_PATTERN)
            });
            if trailing_ok {
                return Ok(TriggerSpec::KeywordActionFromSource {
                    action,
                    player: PlayerFilter::You,
                });
            }
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::KeywordAction {
                action,
                player,
                source_filter: None,
                during_your_turn: false,
            });
        }
    }

    if trigger_pattern_accepts(&words, YOU_OPEN_ATTRACTION_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::OpenAttraction,
            player: PlayerFilter::You,
            source_filter: None,
            during_your_turn: false,
        });
    }

    if trigger_pattern_accepts(&words, YOU_CLAIM_ATTRACTION_PRIZE_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ClaimAttractionPrize,
            player: PlayerFilter::You,
            source_filter: None,
            during_your_turn: false,
        });
    }

    if let Some(exploit_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Exploit)
    {
        let subject_words = &words[..exploit_word_idx];
        let tail_words = &words[exploit_word_idx + 1..];
        if !is_source_reference_words(subject_words) {
            let subject_end = word_view
                .token_index_after_words(exploit_word_idx)
                .unwrap_or(exploit_word_idx);
            let tail_start = word_view
                .token_index_after_words(exploit_word_idx + 1)
                .unwrap_or(tokens.len());
            let tail_tokens = tokens.get(tail_start..).unwrap_or_default();
            let object_filter = if tail_words.is_empty()
                || trigger_pattern_accepts(tail_words, EXPLOIT_CREATURE_TAIL_PATTERN)
            {
                None
            } else {
                Some(parse_object_filter_lexed(tail_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported exploit object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?)
            };
            let subject_tokens = &tokens[..subject_end];
            if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
                return Ok(match object_filter {
                    Some(object_filter) => TriggerSpec::KeywordActionTaggedObject {
                        action: crate::events::KeywordActionKind::Exploit,
                        player: PlayerFilter::Any,
                        source_filter: filter,
                        object_tag: crate::tag::CompilerReferenceTag::Exploited.bind(),
                        object_filter,
                        during_your_main_phase: false,
                    },
                    None => TriggerSpec::KeywordAction {
                        action: crate::events::KeywordActionKind::Exploit,
                        player: PlayerFilter::Any,
                        source_filter: Some(filter),
                        during_your_turn: false,
                    },
                });
            }
        }
    }

    if trigger_pattern_accepts(&words, THIS_EXPLOITS_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Exploit,
            player: PlayerFilter::You,
        });
    }

    if trigger_pattern_accepts(&words, YOU_COMPLETE_DUNGEON_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CompleteDungeon,
            player: PlayerFilter::You,
            source_filter: None,
            during_your_turn: false,
        });
    }

    if trigger_pattern_accepts(&words, WINS_CLASH_TRIGGER_SUFFIX_PATTERN) {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::WinsClash {
                player,
                surface: ironsmith_core::ClashWinTriggerSurface::WinAClash,
            });
        }
    }

    if let Some(counter_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Counter)
        && trigger_pattern_accepts(&words[counter_word_idx..], PASSIVE_COUNTER_PUT_TAIL_PATTERN)
    {
        let one_or_more = trigger_pattern_accepts(&words, ONE_OR_MORE_PREFIX_PATTERN);
        let descriptor_token_end =
            trigger_word_token_start(tokens, counter_word_idx).unwrap_or(tokens.len());
        let counter_descriptor_tokens = &tokens[..(descriptor_token_end + 1)];
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 4;
        let object_tokens = trigger_counter_recipient_tokens(tokens, object_word_start, &words)?;
        let (object_tokens, include_players) = split_counter_recipient_or_player(&object_tokens);
        let filter = parse_object_filter_lexed(object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        let counter_number = words[..counter_word_idx]
            .iter()
            .find_map(|word| ironsmith_core::parse_ordinal_word(word));
        if !include_players
            && let (Some(counter_number), Some(counter_type)) = (counter_number, counter_type)
        {
            return Ok(TriggerSpec::NthCounterPutOn {
                filter,
                counter_type,
                counter_number,
            });
        }

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: None,
            one_or_more,
            include_players,
        });
    }

    if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
        let tail_words = &words[attacks_word_idx + 1..];
        if tail_words.first() == Some(&"while")
            && matches!(words.first(), Some(&"this") | Some(&"it"))
        {
            let predicate_word_idx = attacks_word_idx + 2;
            let predicate_token_idx =
                trigger_word_token_start(tokens, predicate_word_idx).unwrap_or(tokens.len());
            let predicate_tokens = trim_edge_punctuation(&tokens[predicate_token_idx..]);
            if let PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            } = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
                &predicate_tokens,
            )? {
                return Ok(TriggerSpec::ThisAttacksWhileYouControl(filter));
            }
        }
        if trigger_pattern_accepts(tail_words, ATTACKS_AND_IS_NOT_BLOCKED_TAIL_PATTERN) {
            let attacks_token_idx =
                trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            return Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(filter) => TriggerSpec::AttacksAndIsntBlocked(filter),
                    None => TriggerSpec::ThisAttacksAndIsntBlocked,
                },
            );
        }
    }

    // Per-block-pair power comparisons use CreatureBlockedEvent rather than
    // the aggregate "becomes blocked" event. Preserve both object filters so
    // runtime matching can compare the two event snapshots without treating
    // either creature as the ability source.
    if crate::word_primitives::parse_sequence_suffix(&words, &["with", "lesser", "power"]) {
        let relative_tail_word = words.len().saturating_sub(3);
        let relative_tail_token =
            trigger_word_token_start(tokens, relative_tail_word).unwrap_or(tokens.len());

        if let Some(becomes_word) =
            crate::word_primitives::parse_sequence_start(&words, &["becomes", "blocked", "by"])
        {
            let subject_end =
                trigger_word_token_start(tokens, becomes_word).unwrap_or(tokens.len());
            let blocker_start =
                trigger_word_token_start(tokens, becomes_word + 3).unwrap_or(tokens.len());
            if subject_end > 0
                && blocker_start < relative_tail_token
                && relative_tail_token <= tokens.len()
            {
                let blocked = parse_attack_trigger_subject_filter_lexed(&tokens[..subject_end])?
                    .unwrap_or_else(ObjectFilter::source);
                let blocker_tokens =
                    trim_edge_punctuation(&tokens[blocker_start..relative_tail_token]);
                let blocker = parse_object_filter_lexed(&blocker_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported lesser-power blocker filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
                return Ok(TriggerSpec::BecomesBlockedByObjectWithLesserPower { blocked, blocker });
            }
        }

        if let Some(blocks_word) = trigger_atom_word(&words, TriggerClauseAtom::Block) {
            let subject_end = trigger_word_token_start(tokens, blocks_word).unwrap_or(tokens.len());
            let blocked_start =
                trigger_word_token_start(tokens, blocks_word + 1).unwrap_or(tokens.len());
            if subject_end > 0
                && blocked_start < relative_tail_token
                && relative_tail_token <= tokens.len()
            {
                let blocker = parse_attack_trigger_subject_filter_lexed(&tokens[..subject_end])?
                    .unwrap_or_else(ObjectFilter::source);
                let blocked_tokens =
                    trim_edge_punctuation(&tokens[blocked_start..relative_tail_token]);
                let blocked =
                    parse_object_filter_lexed(&blocked_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported lesser-power blocked-object filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                return Ok(TriggerSpec::BlocksObjectWithLesserPower { blocker, blocked });
            }
        }
    }

    if trigger_pattern_accepts(&words, THIS_BLOCKS_OR_BECOMES_BLOCKED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::ThisBlocks),
            Box::new(TriggerSpec::ThisBecomesBlocked),
        ));
    }

    if trigger_pattern_accepts(&words, THIS_BECOMES_BLOCKED_BY_TRIGGER_PREFIX)
        && let Some(by_idx) = trigger_atom_token(tokens, TriggerClauseAtom::By)
    {
        let raw_blocker_tokens = trim_commas(&tokens[by_idx + 1..]);
        let one_or_more = has_leading_one_or_more(&raw_blocker_tokens);
        let blocker_tokens = strip_leading_one_or_more_lexed(&raw_blocker_tokens);
        if !blocker_tokens.is_empty() {
            let mut blocker_filter =
                parse_object_filter_lexed(blocker_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported blocking-object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            preserve_trigger_filter_union_surface(&mut blocker_filter, blocker_tokens);
            blocker_filter.set_union_one_or_more(one_or_more);
            return Ok(TriggerSpec::ThisBecomesBlockedByObject(blocker_filter));
        }
    }

    if trigger_pattern_accepts(&words, THIS_BLOCKS_OR_BECOMES_BLOCKED_BY_TRIGGER_PREFIX)
        && let Some(by_idx) = trigger_atom_token(tokens, TriggerClauseAtom::By)
    {
        let raw_blocker_tokens = trim_commas(&tokens[by_idx + 1..]);
        let one_or_more = has_leading_one_or_more(&raw_blocker_tokens);
        let blocker_tokens = strip_leading_one_or_more_lexed(&raw_blocker_tokens);
        if !blocker_tokens.is_empty() {
            let mut blocker_filter =
                parse_object_filter_lexed(blocker_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported blocking-object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            preserve_trigger_filter_union_surface(&mut blocker_filter, blocker_tokens);
            blocker_filter.set_union_one_or_more(one_or_more);
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::ThisBlocksObject {
                    filter: blocker_filter.clone(),
                    min_blocked_objects: one_or_more.then_some(1),
                }),
                Box::new(TriggerSpec::ThisBecomesBlockedByObject(blocker_filter)),
            ));
        }
    }

    if trigger_pattern_accepts(&words, THIS_BLOCKS_PREFIX_PATTERN)
        && let Some(blocks_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Block)
    {
        let raw_tail_tokens = trim_commas(&tokens[blocks_idx + 1..]);
        let (min_blocked_objects, tail_tokens) = parse_leading_or_more_quantifier(&raw_tail_tokens)
            .map(|(count, stripped)| (Some(count), stripped))
            .unwrap_or((None, raw_tail_tokens.as_slice()));
        if !tail_tokens.is_empty() && !token_slice_at_is(tail_tokens, 0, "or") {
            let mut blocked_filter =
                parse_object_filter_lexed(tail_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported blocked-object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            blocked_filter.set_union_one_or_more(min_blocked_objects.is_some());
            return Ok(TriggerSpec::ThisBlocksObject {
                filter: blocked_filter,
                min_blocked_objects,
            });
        }
    }

    if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
        let subject_words = &words[..attacks_word_idx];
        let tail = &words[attacks_word_idx + 1..];
        if let Some(target) = parse_planeswalker_attacked_with_one_or_more_creatures_target(tail) {
            let attacks_token_idx =
                trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            if let Some(attacker) = trigger_subject_player_selector_lexed(subject_tokens) {
                return Ok(TriggerSpec::PlayerAttacksTargetWithOneOrMore { attacker, target });
            }
        }
        if let Some(target) = parse_one_or_more_planeswalker_attack_target(tail) {
            let attacks_token_idx =
                trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            if let Some(attacker) = trigger_subject_player_selector_lexed(subject_tokens) {
                return Ok(TriggerSpec::PlayerAttacksOneOrMore { attacker, target });
            }
        }
        let life_advantage_tail = non_article_word_refs(tail);
        if is_source_reference_words(subject_words)
            && let Some(attacked_player) =
                crate::grammar::shared_util::reference_shapes::parse_life_advantage_player(
                    &life_advantage_tail,
                )
        {
            let attacks_token_idx =
                trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let mut source = source_reference_surface_for_trigger_subject(&trim_edge_punctuation(
                &tokens[..attacks_token_idx],
            ))
            .map(ObjectFilter::source_with_surface)
            .unwrap_or_else(ObjectFilter::source);
            source.attacking_player_or_planeswalker_controlled_by = Some(attacked_player.clone());
            // This wording names the attacked player itself, not a
            // planeswalker or battle that player protects.
            source.targets_only_player = Some(attacked_player);
            return Ok(TriggerSpec::Attacks(source));
        }
        if matches!(subject_words.first(), Some(&"this") | Some(&"it"))
            && let Some((count, filter)) = parse_attacks_player_who_controls_at_least_tail(tail)
        {
            return Ok(TriggerSpec::ThisAttacksPlayerWhoControlsAtLeast { count, filter });
        }
    }

    let (words, attacked_player_filter, attacked_target_must_be_player) =
        if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
            let tail = &words[attacks_word_idx + 1..];
            if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok() {
                eprintln!("attacks-tail: {tail:?}");
            }
            if trigger_pattern_accepts(tail, ATTACKS_A_PLAYER_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::Any), true)
            } else if trigger_pattern_accepts(tail, ATTACKS_YOU_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::You), true)
            } else if trigger_pattern_accepts(tail, ATTACKS_OPPONENT_TAIL_PATTERN) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::Opponent),
                    true,
                )
            } else if crate::word_primitives::parse_sequence_complete(
                tail,
                &["enchanted", "player"],
            ) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::TaggedPlayer(
                        crate::tag::CompilerReferenceTag::Enchanted.bind(),
                    )),
                    true,
                )
            } else if crate::word_primitives::parse_sequence_complete(
                tail,
                &["the", "player", "who", "has", "the", "initiative"],
            ) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::TaggedPlayer(
                        crate::tag::CompilerReferenceTag::InitiativeHolder.bind(),
                    )),
                    true,
                )
            } else if trigger_pattern_accepts(tail, ATTACKS_DEFENDING_PLAYER_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::Any), true)
            } else if trigger_pattern_accepts(tail, ATTACKS_OPPONENT_OR_PLANESWALKER_TAIL_PATTERN) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::Opponent),
                    false,
                )
            } else if trigger_pattern_accepts(
                tail,
                ATTACKS_ENCHANTED_PLAYER_OR_PLANESWALKER_TAIL_PATTERN,
            ) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::TaggedPlayer(
                        crate::tag::CompilerReferenceTag::Enchanted.bind(),
                    )),
                    false,
                )
            } else if trigger_pattern_accepts(tail, ATTACKS_PLANESWALKER_OR_BATTLE_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], None, false)
            } else {
                (&words[..], None, false)
            }
        } else {
            (&words[..], None, false)
        };

    let last = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    if let Some(attacked) = crate::grammar::trigger_clauses::parse_players_attacked_clause(tokens) {
        let attacked_player_words = crate::lexer::token_word_refs(&tokens[attacked.player]);
        if let Some(player_filter) = parse_trigger_subject_player_filter(&attacked_player_words) {
            return Ok(TriggerSpec::PlayersAttackedOneOrMore(player_filter));
        }
    }

    if last == "blocked" && words.len() >= 2 && words[words.len().saturating_sub(2)] == "becomes" {
        let becomes_word_idx = words.len().saturating_sub(2);
        let becomes_token_idx =
            trigger_word_token_start(tokens, becomes_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..becomes_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::BecomesBlocked(filter),
                None => TriggerSpec::ThisBecomesBlocked,
            },
        );
    }

    match last {
        "attack" | "attacks" => {
            let attack_word_idx = words.len().saturating_sub(1);
            let attack_token_idx =
                trigger_word_token_start(tokens, attack_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attack_token_idx];
            let (minimum_attackers, subject_tokens) =
                parse_leading_or_more_quantifier(subject_tokens)
                    .map(|(count, stripped)| (Some(count), stripped))
                    .unwrap_or((None, subject_tokens));
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let one_or_more = minimum_attackers.is_some() || player_subject;
            Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(mut filter) => {
                        if let Some(player_filter) = attacked_player_filter.clone() {
                            filter.attacking_player_or_planeswalker_controlled_by =
                                Some(player_filter.clone());
                            if attacked_target_must_be_player {
                                filter.targets_only_player = Some(player_filter);
                            }
                        }
                        if let Some(min_total_attackers) =
                            minimum_attackers.filter(|count| *count > 1)
                        {
                            TriggerSpec::AttacksOneOrMoreWithMinTotal {
                                filter,
                                min_total_attackers,
                            }
                        } else if one_or_more {
                            TriggerSpec::AttacksOneOrMore(filter)
                        } else {
                            TriggerSpec::Attacks(filter)
                        }
                    }
                    None => source_reference_surface_for_trigger_subject(subject_tokens)
                        .filter(|surface| {
                            matches!(
                                surface,
                                crate::target::SourceReferenceSurface::ShortName(_)
                                    | crate::target::SourceReferenceSurface::FullName(_)
                            )
                        })
                        .map(|surface| {
                            TriggerSpec::Attacks(ObjectFilter::source_with_surface(surface))
                        })
                        .unwrap_or(TriggerSpec::ThisAttacks),
                },
            )
        }
        "block" | "blocks" => {
            let block_word_idx = words.len().saturating_sub(1);
            let block_token_idx =
                trigger_word_token_start(tokens, block_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..block_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) if one_or_more => TriggerSpec::BlocksOneOrMore(filter),
                Some(filter) => TriggerSpec::Blocks(filter),
                None => TriggerSpec::ThisBlocks,
            })
        }
        "dies" | "die" => {
            let dies_word_idx = words.len().saturating_sub(1);
            let dies_token_idx =
                trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, THIS_DESTINATION_TRIGGER_NAME_PATTERN)
            }) {
                let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
                let subject_words = subject_word_view.to_word_refs();
                if let Some(or_word_idx) =
                    crate::word_primitives::parse_sequence_start(&subject_words, OR_ANOTHER_WORDS)
                {
                    let rhs_word_idx = or_word_idx + 2;
                    let rhs_token_idx = trigger_word_token_start(subject_tokens, rhs_word_idx)
                        .unwrap_or(subject_tokens.len());
                    if rhs_token_idx < subject_tokens.len() {
                        let rhs_tokens = trim_edge_punctuation(&subject_tokens[rhs_token_idx..]);
                        if !rhs_tokens.is_empty()
                            && let Ok(filter) = parse_object_filter_lexed(&rhs_tokens, true)
                        {
                            return Ok(TriggerSpec::Either(
                                Box::new(TriggerSpec::ThisDies),
                                Box::new(TriggerSpec::Dies(filter)),
                            ));
                        }
                    }
                }
                if is_source_reference_words(&subject_words) {
                    return Ok(TriggerSpec::ThisDies);
                }
                return Err(CardTextError::ParseError(format!(
                    "unsupported this-prefixed dies trigger subject (clause: '{}')",
                    words.join(" ")
                )));
            }

            // Builder-aware preprocessing may restore an authored source name
            // after the CST boundary has been found. Keep that named source
            // and the "another" subject as distinct matcher branches: folding
            // both into one ObjectFilter makes `source` an AND constraint and
            // silently stops the trigger from seeing every other object.
            if let Some((_source_filter, other_filter)) =
                parse_source_or_another_trigger_subject_filters(subject_tokens)
            {
                return Ok(TriggerSpec::Either(
                    Box::new(TriggerSpec::ThisDies),
                    Box::new(TriggerSpec::Dies(other_filter)),
                ));
            }

            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if trigger_pattern_accepts(&subject_words, THE_CREATURE_HAUNTS_PATTERN) {
                return Ok(TriggerSpec::HauntedCreatureDies);
            }

            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }

            if let Some(damaged_by_trigger) =
                parse_damage_by_dies_trigger_lexed(subject_tokens, other, words)?
            {
                return Ok(damaged_by_trigger);
            }

            if let Ok(mut filter) = parse_object_filter_lexed(subject_tokens, other) {
                preserve_trigger_filter_union_surface(&mut filter, subject_tokens);
                return Ok(if one_or_more {
                    TriggerSpec::DiesOneOrMore(filter)
                } else {
                    TriggerSpec::Dies(filter)
                });
            }
            let mut normalized_subject_tokens = Vec::with_capacity(subject_tokens.len());
            let mut idx = 0usize;
            while idx < subject_tokens.len() {
                if token_matches_clause_shape(&subject_tokens[idx], AND_WORD_PATTERN)
                    && subject_tokens
                        .get(idx + 1)
                        .is_some_and(|token| token_matches_clause_shape(token, OR_WORD_PATTERN))
                {
                    idx += 1;
                    continue;
                }
                normalized_subject_tokens.push(subject_tokens[idx].clone());
                idx += 1;
            }
            if normalized_subject_tokens.len() != subject_tokens.len()
                && let Ok(mut filter) = parse_object_filter_lexed(&normalized_subject_tokens, other)
            {
                preserve_trigger_filter_union_surface(&mut filter, subject_tokens);
                return Ok(if one_or_more {
                    TriggerSpec::DiesOneOrMore(filter)
                } else {
                    TriggerSpec::Dies(filter)
                });
            }

            Err(CardTextError::ParseError(format!(
                "unsupported dies trigger subject filter (clause: '{}')",
                words.join(" ")
            )))
        }
        "turn" if words.len() >= 3 && trigger_pattern_accepts(words, DIES_THIS_TURN_SUFFIX) => {
            let dies_word_idx = words.len().saturating_sub(3);
            let dies_token_idx =
                trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies-this-turn trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }
            let mut filter =
                parse_trigger_subject_filter_lexed(subject_tokens)?.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported dies-this-turn trigger subject filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            if other {
                filter.other = true;
            }
            Ok(if one_or_more {
                TriggerSpec::DiesOneOrMore(filter)
            } else {
                TriggerSpec::Dies(filter)
            })
        }
        "turn"
            if words.len() >= 4 && trigger_pattern_accepts(words, DIES_DURING_YOUR_TURN_SUFFIX) =>
        {
            let dies_word_idx = words.len().saturating_sub(4);
            let dies_token_idx =
                trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies-during-turn trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }
            let filter = parse_object_filter_lexed(subject_tokens, other).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported dies-during-turn trigger subject filter (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            Ok(TriggerSpec::DiesDuringTurn {
                filter,
                one_or_more,
                during_turn: PlayerFilter::You,
            })
        }
        "combat"
            if crate::word_primitives::parse_sequence_suffix(
                words,
                &["dies", "during", "combat"],
            ) =>
        {
            let dies_word_idx = words.len() - 3;
            let dies_token_idx =
                trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            let subject_words =
                ActivationRestrictionCompatWords::new(subject_tokens).to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::DiesDuringCombat {
                    filter: None,
                    one_or_more,
                });
            }
            let filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported dies-during-combat trigger subject filter (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            Ok(TriggerSpec::DiesDuringCombat {
                filter: Some(filter),
                one_or_more,
            })
        }
        // The public triggered-line splitter owns the leading trigger intro
        // (`At`) and passes only the remaining clause to this parser. Keep
        // that representation event-qualified too; otherwise the broad
        // end-step grammar silently turns the monarch's end step into every
        // player's end step.
        _ if crate::word_primitives::parse_any_sequence_complete(
            words,
            &[
                &[
                    "at",
                    "the",
                    "beginning",
                    "of",
                    "the",
                    "monarchs",
                    "end",
                    "step",
                ],
                &[
                    "at",
                    "the",
                    "beginning",
                    "of",
                    "the",
                    "monarch",
                    "end",
                    "step",
                ],
                &["at", "beginning", "of", "monarch", "end", "step"],
                &["the", "beginning", "of", "the", "monarchs", "end", "step"],
                &["the", "beginning", "of", "the", "monarch", "end", "step"],
                &["beginning", "of", "monarch", "end", "step"],
            ],
        ) =>
        {
            Ok(TriggerSpec::BeginningOfMonarchEndStep)
        }
        _ if trigger_pattern_accepts(words, BEGINNING_END_STEP_TRIGGER_PATTERN)
            && !trigger_pattern_accepts(words, NEXT_END_STEP_TRIGGER_PATTERN) =>
        {
            let player = parse_possessive_clause_player_filter(words);
            let definite_surface = player == PlayerFilter::Any
                && crate::word_primitives::sequence_occurs(words, &["the", "end", "step"]);
            Ok(if definite_surface {
                TriggerSpec::BeginningOfTheEndStep
            } else {
                TriggerSpec::BeginningOfEndStep(player)
            })
        }
        _ if trigger_pattern_accepts(words, BEGINNING_UPKEEP_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfUpkeep(parse_possessive_clause_player_filter(words)),
        ),
        _ if trigger_pattern_accepts(words, BEGINNING_DRAW_STEP_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfDrawStep(parse_possessive_clause_player_filter(words)),
        ),
        _ if trigger_pattern_accepts(words, BEGINNING_FIRST_MAIN_PHASE_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfPrecombatMain(parse_possessive_clause_player_filter(words)),
        ),
        _ if trigger_pattern_accepts(words, BEGINNING_SECOND_MAIN_PHASE_TRIGGER_PATTERN) => {
            Ok(TriggerSpec::BeginningOfPostcombatMain {
                player: parse_possessive_clause_player_filter(words),
                surface: ironsmith_core::trigger_model::PostcombatMainPhaseSurface::SecondMain,
            })
        }
        _ if trigger_pattern_accepts(words, BEGINNING_PRECOMBAT_MAIN_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfPrecombatMain(parse_possessive_clause_player_filter(words)),
        ),
        _ if trigger_pattern_accepts(words, BEGINNING_POSTCOMBAT_MAIN_TRIGGER_PATTERN) => {
            let each_of = crate::word_primitives::sequence_occurs(
                words,
                &["each", "of", "your", "postcombat"],
            );
            Ok(TriggerSpec::BeginningOfPostcombatMain {
                player: parse_possessive_clause_player_filter(words),
                surface: if each_of {
                    ironsmith_core::trigger_model::PostcombatMainPhaseSurface::EachOfPostcombatMains
                } else {
                    ironsmith_core::trigger_model::PostcombatMainPhaseSurface::PostcombatMain
                },
            })
        }
        _ if trigger_pattern_accepts(words, BEGINNING_MAIN_PHASE_TRIGGER_PATTERN) => {
            let each_of = crate::word_primitives::sequence_occurs(
                words,
                &["each", "of", "your", "main", "phases"],
            );
            Ok(TriggerSpec::BeginningOfMainPhase {
                player: parse_possessive_clause_player_filter(words),
                surface: if each_of {
                    ironsmith_core::trigger_model::MainPhaseSurface::EachOfMainPhases
                } else {
                    ironsmith_core::trigger_model::MainPhaseSurface::MainPhase
                },
            })
        }
        _ if trigger_pattern_accepts(words, BEGINNING_COMBAT_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfCombat(parse_possessive_clause_player_filter(words)),
        ),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported trigger clause (clause: '{}')",
            words.join(" ")
        ))),
    }
}

pub(super) fn parse_named_ability_trigger_tail_lexed(
    tail_tokens: &[OwnedLexToken],
) -> Option<String> {
    trigger_grammar::parse_named_ability_tail(tail_tokens).map(|tail| tail.marker)
}

pub(super) fn parse_possessive_ability_trigger_tail_lexed(
    tail_tokens: &[OwnedLexToken],
    tail_words: &[&str],
) -> Result<Option<(ObjectFilter, Option<String>)>, CardTextError> {
    let Some(tail) = trigger_grammar::parse_possessive_ability_tail(tail_tokens) else {
        return Ok(None);
    };
    let owner_subject_tokens = &tail_tokens[tail.owner];
    let owner_filter = parse_object_filter_lexed(owner_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;

    Ok(Some((owner_filter, tail.marker)))
}

pub(super) fn parse_ability_of_object_trigger_tail_lexed(
    tail_tokens: &[OwnedLexToken],
    tail_words: &[&str],
) -> Result<Option<(ObjectFilter, bool)>, CardTextError> {
    let Some(tail) = trigger_grammar::parse_ability_of_object_tail(tail_tokens) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(&tail_tokens[tail.filter], false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    if tail.chosen_type_reference {
        filter.chosen_creature_type = true;
    }
    Ok(Some((filter, tail.non_mana_only)))
}
