use super::*;

pub(super) fn pre_rule_token_followups(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    // The generic dispatch path trims terminal punctuation before running the
    // followup registry. Quoted token rules need their closing quote and the
    // period inside that quote, so recognize and merge them from the original
    // sentence slice while leaving every other followup on the normalized
    // tokens supplied by the caller.
    let reminder_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lowered)
        .unwrap_or(sentence_tokens);
    let authored_reminder_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lexed)
        .unwrap_or(sentence_tokens);
    let reminder_facts = followup_shapes::token_reminder_followup_facts(reminder_tokens);
    if try_bind_conditional_token_entry_followup(state.effects, authored_reminder_tokens)? {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some(
                "subject-verb verb=Create subject=implicit recognizer=conditional-token-entry",
            ),
        }));
    }
    if let Some(followup) = parse_create_more_of_prior_tokens(sentence_tokens, state.effects) {
        if followup.instead {
            let Some(previous) = state.effects.pop() else {
                return Err(CardTextError::InvariantViolation(
                    "typed prior-token replacement lost its default effect".to_string(),
                ));
            };
            if !effect_creates_any_token(&previous) {
                state.effects.push(previous);
                return Err(CardTextError::ParseError(
                    "prior-token replacement does not immediately follow token creation"
                        .to_string(),
                ));
            }
            state.effects.push(EffectAst::SelfReplacement {
                predicate: followup.predicate,
                if_true: vec![followup.create],
                if_false: vec![previous],
                attach_to_previous_ability: false,
            });
        } else {
            state.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: followup.predicate,
                if_true: vec![followup.create],
                if_false: Vec::new(),
            }));
        }
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some("subject-verb verb=Create subject=implicit recognizer=prior-token-instead"),
        }));
    }
    if let Some((effects, predicate)) = parse_instead_replacement_sentence(sentence_tokens)?
        && !state.effects.is_empty()
    {
        let previous = state
            .effects
            .pop()
            .expect("non-empty effect list yields a previous effect");
        state.effects.push(EffectAst::SelfReplacement {
            predicate,
            if_true: effects,
            if_false: vec![previous],
            attach_to_previous_ability: false,
        });
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some("subject-verb verb=Create subject=implicit recognizer=instead-replacement"),
        }));
    }
    if is_spawn_scion_token_mana_reminder(sentence_tokens) {
        if state
            .effects
            .last()
            .is_some_and(effect_creates_eldrazi_spawn_or_scion)
        {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: None,
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone token mana reminder clause (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    }
    if let Some(effect) =
        parse_sentence_exile_that_token_when_source_leaves(sentence_tokens, state.effects)
    {
        state.effects.push(effect);
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    if let Some(effect) =
        parse_sentence_sacrifice_source_when_that_token_leaves(sentence_tokens, state.effects)
    {
        state.effects.push(effect);
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    // A copy-token lifecycle modifier is more specific than the broad token
    // reminder family. Apply it through nested source-sentence/loop wrappers
    // first; otherwise the generic reminder path sees that a token exists but
    // cannot reach the nested copy action and reports a false standalone
    // reminder error.
    let token_copy_followup = parse_token_copy_followup_sentence(sentence_tokens);
    if matches!(token_copy_followup, Some(TokenCopyFollowup::HasHaste(_)))
        && crate::grammar::token_definitions::token_ability_sentence_uses_gain_verb(reminder_tokens)
        && let Some(abilities) =
            parse_token_granted_ability_followup_sentence_lexed(reminder_tokens)?
        && try_apply_token_granted_ability_followup(
            state.effects,
            &abilities,
            ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain,
        )?
    {
        // Ordinary created tokens retain the authored `gain` presentation.
        // Copy-token programs do not expose a mutable token definition here;
        // when this probe returns false, the lifecycle fallback below folds
        // haste into that copy action instead.
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some(
                "subject-verb verb=Grant subject=implicit recognizer=created-token-ability-followup",
            ),
        }));
    }
    if let Some(followup) = token_copy_followup
        && try_apply_token_copy_followup(state.effects, followup)?
    {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some(
                "subject-verb verb=Exile subject=implicit recognizer=token-copy-delayed-followup",
            ),
        }));
    }
    // A duration-scoped grant is an effect on the created objects, not part of
    // their copiable token definition. Let the typed follow-up plan bind it to
    // the preceding creation instead of folding haste into the token forever.
    let is_temporary_token_grant = matches!(
        token_copy_followup,
        Some(TokenCopyFollowup::GainHasteUntilEndOfTurn(_))
    );
    if !is_temporary_token_grant
        && crate::effect_sentences::mixed_pronoun_token_rule_list(authored_reminder_tokens)
            .is_some()
        && state.effects.last().is_some_and(effect_creates_any_token)
        && crate::effect_sentences::attach_mixed_pronoun_token_rules_to_last_create(
            state.effects,
            authored_reminder_tokens,
        )
    {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some(
                "subject-verb verb=Grant subject=implicit recognizer=created-token-ability-followup",
            ),
        }));
    }
    if !is_temporary_token_grant
        && is_generic_token_reminder_sentence(reminder_tokens)
        && state.effects.last().is_some_and(effect_creates_any_token)
    {
        if append_token_reminder_to_last_create_effect(state.effects, reminder_tokens)? {
            let route = reminder_facts.lifecycle_head.then_some(
                "subject-verb verb=Exile subject=implicit recognizer=token-copy-delayed-followup",
            );
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route,
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone token reminder clause (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    }
    let parses_under_token_source_identity =
        crate::util::source_reference_surface_for_words(&["this", "token"]).is_some();
    if !is_temporary_token_grant
        && is_generic_token_reminder_sentence(reminder_tokens)
        && !parses_under_token_source_identity
        && !reminder_facts.delayed_pronoun_lifecycle
        && !reminder_facts.pronoun_trigger_prefix
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone token reminder clause (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    }
    // Target-declaration normalization may collapse repeated `target`
    // markers into one union filter.  The authored sentence is still the
    // authoritative declaration surface, so split its independent target
    // slots before the broad normalized target parser can merge them.
    if let Some(effects) = parse_choose_target_prelude_sentence(authored_reminder_tokens)? {
        state.effects.extend(effects);
        *state.carried_context = None;
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    if let Some(followup) = token_copy_followup {
        let mut plan = SentenceParsePlan::new(sentence_tokens.to_vec());
        plan.direct_effects = Some(apply_unapplied_token_copy_followup(
            sentences[sentence_idx].lowered(),
            sentence_tokens,
            followup,
            state.effects.is_empty(),
        )?);
        return Ok(Some(PreParseFollowupResult::Plan(plan)));
    }
    if let Some(abilities) = parse_token_granted_ability_followup_sentence_lexed(reminder_tokens)? {
        let presentation =
            if crate::grammar::token_definitions::token_ability_sentence_uses_gain_verb(
                reminder_tokens,
            ) {
                ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
            } else {
                ironsmith_core::TokenAbilityPresentation::SeparateSentence
            };
        if try_apply_token_granted_ability_followup(state.effects, &abilities, presentation)? {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: Some(
                    "subject-verb verb=Grant subject=implicit recognizer=created-token-ability-followup",
                ),
            }));
        }
    }
    Ok(None)
}

/// Parse `<action> instead[ if <condition>]` as a replacement of the action
/// the preceding sentence performed.
///
/// An authored ability-word ladder restates the whole action ("Morbid — Create
/// three 2/2 green Wolf creature tokens instead if a creature died this turn")
/// rather than referring back to the tokens it replaces, so the sentence is a
/// self-replacement over the previous effect.
fn parse_instead_replacement_sentence(
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<(Vec<EffectAst>, PredicateAst)>, CardTextError> {
    let tokens = crate::grammar::effects::split_labeled_effect_prefix_lexed(sentence_tokens)
        .unwrap_or(sentence_tokens);
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    // A damage replacement repeats the earlier event's source and target
    // ("It deals 4 damage to that creature instead ...", or the same sentence
    // written with the card's name) rather than restating the action. Its own
    // family binds those references and keeps the authored trailing condition
    // surface.
    let words = crate::lexer::parser_token_word_refs(tokens);
    if effect_grammar::followup_shapes::is_anaphoric_damage_self_replacement(tokens)
        || crate::word_primitives::sequence_occurs(&words, &["damage", "to", "that", "creature"])
    {
        return Ok(None);
    }
    let Some(instead_idx) = tokens.iter().rposition(|token| token.is_word("instead")) else {
        return Ok(None);
    };
    let action_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[..instead_idx]);
    if action_tokens.is_empty() {
        return Ok(None);
    }
    let condition_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[instead_idx + 1..]);
    let Some(predicate) = parse_trailing_if_predicate_lexed(condition_tokens) else {
        return Ok(None);
    };
    let Ok(effects) = crate::effect_sentences::parse_effect_sentences_lexed(action_tokens) else {
        return Ok(None);
    };
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some((effects, predicate)))
}

pub(super) fn parse_create_more_of_prior_tokens(
    sentence_tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<PriorTokenCreateFollowup> {
    let shape = followup_shapes::parse_create_more_prior_tokens(sentence_tokens)?;
    let predicate_tokens =
        crate::grammar::effects::split_labeled_effect_prefix_lexed(shape.predicate_tokens)
            .unwrap_or(shape.predicate_tokens);
    let predicate = parse_trailing_if_predicate_lexed(predicate_tokens)?;
    let mut create = prior_effects.last()?.clone();
    let EffectAst::SubjectVerb(subject_verb) = &mut create else {
        return None;
    };
    let (count, previous_target) = match &mut subject_verb.action {
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. })
        | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { count, .. }) => (count, None),
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { source, count, .. }) => {
            (count, Some(source.clone()))
        }
        _ => return None,
    };
    *count = Value::Fixed(shape.count as i32);
    let predicate = bind_self_replacement_condition_to_previous_target(
        predicate,
        shape.predicate_tokens,
        previous_target.as_ref(),
    );

    Some(PriorTokenCreateFollowup {
        predicate,
        create,
        instead: shape.instead,
    })
}

pub(super) fn post_rule_token_copy_and_extra_turn(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let sentence_effects_baseline = sentence_effects.clone();
    collapse_token_copy_next_end_step_exile_followup(sentence_effects, sentence_tokens);
    collapse_token_copy_end_of_combat_exile_followup(sentence_effects, sentence_tokens);
    if is_that_turn_end_step_sentence(sentence_tokens)
        && let Some(extra_turn_player) = most_recent_extra_turn_player(state.effects)
        && !sentence_effects.is_empty()
    {
        // The leading delayed-schedule grammar already recognizes
        // "that turn's end step". Rebind its anaphoric player to the extra
        // turn we just parsed instead of wrapping the schedule a second time.
        // A second wrapper would register a delayed trigger whose payload is
        // another identical delayed trigger.
        if let [EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn { player, .. })] =
            sentence_effects.as_mut_slice()
        {
            *player = extra_turn_player;
        } else {
            // Older/narrower sentence routes can still surface this wording as
            // a plain next-end-step wrapper. Preserve only its payload when
            // specializing it to the preceding extra turn.
            let delayed_effects = if matches!(
                sentence_effects.as_slice(),
                [EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep { .. })]
            ) {
                match sentence_effects.pop().expect("matched one delayed effect") {
                    EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep { effects, .. }) => effects,
                    _ => unreachable!("matched delayed-next-end-step effect"),
                }
            } else {
                std::mem::take(sentence_effects)
            };
            sentence_effects.push(EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn {
                player: extra_turn_player,
                effects: delayed_effects,
            }));
        }
    }
    if *sentence_effects == sentence_effects_baseline {
        Ok(None)
    } else {
        Ok(Some(PostParseFollowupResult::Annotated))
    }
}

pub(super) fn trailing_optional_copy_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    let is_optional_copy = match &*effect {
        EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: PlayerAst::You | PlayerAst::Implicit,
            effects,
        }) => effects_copy_a_stack_object(effects),
        _ => false,
    };
    if is_optional_copy {
        return match effect {
            EffectAst::Permissions(PermissionEffectAst::May { effects })
            | EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: PlayerAst::You | PlayerAst::Implicit,
                effects,
            }) => Some(effects),
            _ => None,
        };
    }
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::Coordinated { effects, .. } => effects
            .last_mut()
            .and_then(trailing_optional_copy_effects_mut),
        _ => None,
    }
}
