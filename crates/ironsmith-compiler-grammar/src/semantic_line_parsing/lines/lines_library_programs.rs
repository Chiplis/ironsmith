use super::*;

pub(super) fn starts_with_exact_graveyard_card_copy_cast_sequence(
    effect_parse_tokens: &[OwnedLexToken],
) -> bool {
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let Ok(Some(matched)) = crate::effect_sentences::try_parse_document_program(&sentences, 0)
    else {
        return false;
    };
    matched.feature_tag == Some("graveyard-card-copy-cast")
        && matched.consumed_sentences <= sentences.len()
}

pub fn exact_graveyard_card_copy_cast_sequence(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    let Ok(Some(matched)) = crate::effect_sentences::try_parse_document_program(&sentences, 0)
    else {
        return None;
    };
    if !matches!(
        matched.feature_tag,
        Some("graveyard-card-copy-cast" | "conditional-graveyard-card-copy-cast")
    ) {
        return None;
    }
    let trailing = &sentences[matched.consumed_sentences..];
    let has_standard_copy_cast_reminder = matches!(
        trailing,
        [reminder]
            if matches!(
                crate::lexer::parser_token_word_refs(reminder.lowered()).as_slice(),
                [
                    "you", "still", "pay", "its", "costs", "a", "copy", "of", "a",
                    "permanent", "spell", "becomes", "a", "token"
                ]
            )
    ) || matches!(
        trailing,
        [costs, permanent_copy]
            if matches!(
                crate::lexer::parser_token_word_refs(costs.lowered()).as_slice(),
                ["you", "still", "pay", "its", "costs"]
            ) && matches!(
                crate::lexer::parser_token_word_refs(permanent_copy.lowered()).as_slice(),
                ["a", "copy", "of", "a", "permanent", "spell", "becomes", "a", "token"]
            )
    );
    let trailing_cast_result = match trailing {
        [] => None,
        [_] if has_standard_copy_cast_reminder => None,
        [sentence] => {
            let Ok(effects) =
                crate::effect_sentences::parse_effect_sentence_lexed(sentence.lowered())
            else {
                return None;
            };
            let [
                effect @ EffectAst::IfResult {
                    predicate: crate::cards::builders::IfResultPredicate::Did,
                    effects: result_effects,
                },
            ] = effects.as_slice()
            else {
                return None;
            };
            if result_effects.is_empty() {
                return None;
            }
            Some(effect.clone())
        }
        [_, _] if has_standard_copy_cast_reminder => None,
        _ => return None,
    };

    let mut effects = matched.effects;
    if has_standard_copy_cast_reminder {
        fn mark_cast_copy(effects: &mut [EffectAst]) {
            for effect in effects {
                if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CastTagged {
                            as_copy: true,
                            copy_cast_reminder_surface,
                            ..
                        },
                    ..
                }) = effect
                {
                    *copy_cast_reminder_surface = true;
                }
                crate::model::visit::for_each_nested_effects_mut(effect, true, mark_cast_copy);
            }
        }
        mark_cast_copy(&mut effects);
    }
    if let Some(trailing_cast_result) = trailing_cast_result {
        effects.push(trailing_cast_result);
    }
    Some(effects)
}

pub fn exact_looked_hand_optional_cast_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 2 {
        return None;
    }
    let effects =
        crate::effect_sentences::parse_look_at_players_hand_then_may_cast_from_those_cards(
            &sentences, 0,
        )
        .ok()??;
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtHand {
                    target:
                        TargetAst::Player(
                            PlayerFilter::DamagedPlayer
                            | PlayerFilter::IteratedPlayer
                            | PlayerFilter::Target(_)
                            | PlayerFilter::AliasedTarget(_),
                            _,
                        ),
                },
            ..
        }),
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player: PlayerAst::You,
            zone_owner: PlayerAst::That,
            filter,
            zone: Zone::Hand,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        },
    ] = effects.as_slice()
    else {
        return None;
    };
    if filter != &ObjectFilter::nonland().in_zone(Zone::Hand) {
        return None;
    }
    Some(effects)
}

/// Preserve the authored optional collection cast long enough for the
/// two-sentence looked-hand rule to bind its zone owner. Generic pronoun
/// normalization would otherwise reduce the second sentence to `Cast it`.
pub fn is_authored_look_hand_optional_cast_bundle(effect_parse_tokens: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(effect_parse_tokens);
    let [look, cast] = sentences.as_slice() else {
        return false;
    };
    matches!(
        crate::lexer::parser_token_word_refs(look).as_slice(),
        ["look", "at", "that", "players", "hand"]
    ) && matches!(
        crate::lexer::parser_token_word_refs(cast).as_slice(),
        [
            "you", "may", "cast", "a", "spell", "from", "among", "those", "cards", "without",
            "paying", "its", "mana", "cost"
        ]
    )
}

pub fn parse_library_origin_source_pump_unblockable_triggered_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    fn exact_owned_card_filter(filter: &ObjectFilter) -> bool {
        filter.owner == Some(PlayerFilter::You)
            && filter.nontoken
            && filter.card_types.is_empty()
            && filter.zone.is_none()
            && filter.controller.is_none()
    }

    fn recover_exact_library_origin(trigger: &TriggerSpec) -> Option<TriggerSpec> {
        match trigger {
            TriggerSpec::WithIntro { intro, trigger } => Some(TriggerSpec::WithIntro {
                intro: *intro,
                trigger: Box::new(recover_exact_library_origin(trigger)?),
            }),
            TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Library,
                one_or_more: true,
            } if exact_owned_card_filter(filter) => Some(trigger.clone()),
            TriggerSpec::PutIntoGraveyardOneOrMore(filter) if exact_owned_card_filter(filter) => {
                Some(TriggerSpec::PutIntoGraveyardFromZone {
                    filter: filter.clone(),
                    from: Zone::Library,
                    one_or_more: true,
                })
            }
            _ => None,
        }
    }

    let Some(split) = semantic_grammar::parse_comma_split_tokens(tokens) else {
        return Ok(None);
    };
    if !crate::word_primitives::sequence_occurs(
        &crate::lexer::parser_token_word_refs(split.before),
        &["from", "your", "library"],
    ) {
        return Ok(None);
    }
    let authored_intro =
        super::super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(
            split.before,
        );
    let trigger_tokens = if split
        .before
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        &split.before[1..]
    } else {
        split.before
    };
    let trigger = match parse_trigger_clause_lexed(trigger_tokens) {
        Ok(trigger) => trigger,
        Err(_) => {
            let trigger_view = crate::lexer::TokenWordView::new(trigger_tokens);
            let Some(origin_word) = trigger_view.parse_phrase_start(&["from", "your", "library"])
            else {
                return Ok(None);
            };
            let Some(origin_idx) = trigger_view.map_word_to_token_start(origin_word) else {
                return Ok(None);
            };
            let Some(origin_end) = trigger_view.token_index_after_words(origin_word + 3) else {
                return Ok(None);
            };
            let without_origin = trigger_tokens[..origin_idx]
                .iter()
                .chain(trigger_tokens[origin_end..].iter())
                .cloned()
                .collect::<Vec<_>>();
            let Ok(trigger) = parse_trigger_clause_lexed(&without_origin) else {
                return Ok(None);
            };
            trigger
        }
    };
    let Some(mut trigger) = recover_exact_library_origin(&trigger) else {
        return Ok(None);
    };
    if let Some(intro) = authored_intro {
        trigger = TriggerSpec::WithIntro {
            intro,
            trigger: Box::new(trigger),
        };
    }
    // Parse the pump independently from the shared-subject restriction. The
    // broad `can't` sentence family otherwise claims the full conjunction and
    // mistakes the trigger's `creature card in your library` for its subject.
    let Some(and_idx) = split.after.iter().enumerate().find_map(|(idx, token)| {
        let tail = crate::lexer::parser_token_word_refs(&split.after[idx + 1..]);
        (token.is_word("and")
            && crate::word_primitives::parse_any_sequence_complete(
                &tail,
                &[
                    &["cant", "be", "blocked", "this", "turn"],
                    &["can't", "be", "blocked", "this", "turn"],
                    &["can", "t", "be", "blocked", "this", "turn"],
                ],
            ))
        .then_some(idx)
    }) else {
        return Ok(None);
    };
    let pump_words =
        crate::lexer::parser_token_word_refs(trim_lexed_commas(&split.after[..and_idx]));
    if !crate::word_primitives::parse_sequence_prefix(&pump_words, &["this", "creature", "gets"])
        || !crate::word_primitives::parse_sequence_suffix(
            &pump_words,
            &["until", "end", "of", "turn"],
        )
        || !(crate::word_primitives::contains_word(&pump_words, "+1/+1")
            || pump_words.iter().filter(|word| **word == "1").count() == 2)
    {
        return Ok(None);
    }
    let mut effects = vec![EffectAst::subject_verb_pump(
        Value::Fixed(1),
        Value::Fixed(1),
        TargetAst::Source(None),
        Until::EndOfTurn,
        None,
    )];
    effects.push(EffectAst::subject_verb_cant(
        crate::effect::Restriction::BeBlocked(ObjectFilter::source()),
        Until::EndOfTurn,
        None,
    ));
    Ok(Some(LineAst::Triggered {
        trigger,
        effects,
        max_triggers_per_turn: None,
    }))
}

#[cfg(test)]
#[test]
pub(super) fn looked_hand_optional_cast_authored_guard_keeps_possessive_and_may() {
    let tokens = lex_line(
        "look at that player's hand. You may cast a spell from among those cards without paying its mana cost.",
        0,
    )
    .expect("looked-hand optional cast should lex");
    assert!(is_authored_look_hand_optional_cast_bundle(&tokens));
    assert!(exact_looked_hand_optional_cast_bundle(&tokens).is_some());

    let mandatory = lex_line(
        "look at that player's hand. Cast a spell from among those cards without paying its mana cost.",
        0,
    )
    .expect("mandatory near miss should lex");
    assert!(!is_authored_look_hand_optional_cast_bundle(&mandatory));
}

#[cfg(test)]
#[test]
pub(super) fn library_origin_source_pump_unblockable_preemption_is_exact() {
    fn is_library_origin(trigger: &TriggerSpec) -> bool {
        match trigger {
            TriggerSpec::WithIntro { trigger, .. } => is_library_origin(trigger),
            TriggerSpec::PutIntoGraveyardFromZone {
                from: Zone::Library,
                one_or_more: true,
                ..
            } => true,
            _ => false,
        }
    }

    let full = "Whenever one or more cards are put into your graveyard from your library, this creature gets +1/+1 until end of turn and can't be blocked this turn.";
    let full_tokens = lex_line(full, 0).expect("exact library-origin line should lex");
    let parsed = parse_library_origin_source_pump_unblockable_triggered_line(&full_tokens)
        .expect("exact library-origin preemption should parse")
        .expect("exact library-origin preemption should claim the line");
    let LineAst::Triggered {
        trigger, effects, ..
    } = parsed
    else {
        panic!("expected one triggered line: {parsed:#?}");
    };
    assert!(is_library_origin(&trigger), "{trigger:#?}");
    assert_eq!(effects.len(), 2, "{effects:#?}");

    let hand_origin = lex_line(
        "Whenever one or more cards are put into your graveyard from your hand, this creature gets +1/+1 until end of turn and can't be blocked this turn.",
        0,
    )
    .expect("lex nonlibrary near miss");
    assert!(
        parse_library_origin_source_pump_unblockable_triggered_line(&hand_origin)
            .expect("near miss should remain parseable")
            .is_none()
    );
}
