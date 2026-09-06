use crate::cards::builders::SourcePredicateAst;
use crate::cards::builders::ForEachEffectAst;
use super::*;

pub(super) fn pre_rule_if_no_one_does_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    if shape.kind != followup_shapes::ConditionalFollowupKind::IfNoOneDoes {
        return Ok(None);
    }
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::DidNot);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

pub(super) fn pre_rule_if_you_win_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    let predicate = match shape.kind {
        followup_shapes::ConditionalFollowupKind::IfYouWinClash => IfResultPredicate::WonClash,
        followup_shapes::ConditionalFollowupKind::IfYouWinFlip => IfResultPredicate::Did,
        followup_shapes::ConditionalFollowupKind::IfYouWin => {
            let preceded_by_clash = state.effects.last().is_some_and(|effect| {
                terminal_result_producer(effect) == Some(TerminalResultProducer::Clash)
            });
            if preceded_by_clash {
                IfResultPredicate::WonClash
            } else {
                IfResultPredicate::Did
            }
        }
        _ => return Ok(None),
    };
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(predicate);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

pub(super) fn take_self_replacement_condition(
    effect: EffectAst,
) -> Option<(PredicateAst, Vec<EffectAst>, Vec<EffectAst>)> {
    match effect {
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }) => Some((predicate, if_true, if_false)),
        // Damage parsing preserves authored trailing condition order with a
        // typed `TrailingIf`. Once an `instead` follow-up has been classified
        // as a self-replacement, both surfaces carry the same semantic branch
        // and must be normalized before ordinary object-reference lowering.
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects }) => Some((predicate, effects, Vec::new())),
        EffectAst::ControlFlow(control) => {
            let crate::model::control_flow::ControlFlowNodeAst::Condition {
                condition,
                consequence_program,
                alternative_program,
                ..
            } = &control.node
            else {
                return None;
            };
            let crate::model::control_flow::ControlPredicateAst::State(predicate) =
                &condition.predicate
            else {
                return None;
            };
            let if_true = control.program(*consequence_program)?.effects.clone();
            let if_false = alternative_program
                .and_then(|program| control.program(program))
                .map(|program| program.effects.clone())
                .unwrap_or_default();
            Some((predicate.clone(), if_true, if_false))
        }
        _ => None,
    }
}

fn explicit_self_replacement_result_tag(effect: &EffectAst) -> Option<TagKey> {
    match effect {
        EffectAst::TagAffected { tag, .. }
            if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            Some(tag.clone().into())
        }
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
            if effects.len() == 1 =>
        {
            explicit_self_replacement_result_tag(&effects[0])
        }
        _ => None,
    }
}

pub(super) fn predicate_explicitly_says_that_land(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::Source(SourcePredicateAst::SourceMatches(filter))
        | PredicateAst::ItMatches(filter)
        | PredicateAst::TargetMatches(filter) => {
            filter.demonstrative_antecedent_surface()
                == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land)
        }
        PredicateAst::Not(inner) => predicate_explicitly_says_that_land(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_explicitly_says_that_land(left) || predicate_explicitly_says_that_land(right)
        }
        _ => false,
    }
}

/// Effects authored after a self-replacement happen regardless of which arm
/// replaced the original event. Keep that common suffix in both arms so the
/// lowering boundary remains one executable self-replacement segment and
/// branch-local pronouns resolve against the object produced by that arm.
pub(super) fn post_rule_self_replacement_common_suffix(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    if sentence_effects.is_empty()
        || matches!(
            classify_instead_followup_tokens(sentence_tokens),
            InsteadSemantics::SelfReplacement
        )
    {
        return Ok(None);
    }
    let Some(EffectAst::SelfReplacement {
        if_true, if_false, ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };

    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    if crate::word_primitives::parse_sequence_prefix(
        &words,
        &["exile", "the", "chosen", "creature", "then"],
    ) && crate::word_primitives::sequence_occurs(
        &words,
        &["controller", "gains", "life", "equal", "to"],
    ) && crate::word_primitives::sequence_occurs(&words, &["mana", "value"])
        && !effects_contain_gain_life(sentence_effects)
    {
        let iterated = crate::tag::CompilerReferenceTag::It.bind();
        sentence_effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::ItsController,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                    amount: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(iterated.key.clone())))
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
                }),
            )],
        }));
    }

    if_true.extend(sentence_effects.iter().cloned());
    if_false.append(sentence_effects);
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

pub(in super::super) fn post_rule_future_zone_and_self_replacement(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    lowered_sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let sentence_effects_baseline = sentence_effects.clone();
    let sentence_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lexed)
        .unwrap_or(lowered_sentence_tokens);
    maybe_rewrite_future_zone_replacement_sentence(sentence_effects, sentence_tokens);
    if matches!(
        classify_instead_followup_tokens(sentence_tokens),
        InsteadSemantics::SelfReplacement
    ) && sentence_effects.len() == 1
        && !state.effects.is_empty()
        && sentence_effects.first().is_some_and(|effect| {
            matches!(
                effect,
                EffectAst::Conditionals(ConditionalEffectAst::Conditional { .. }) | EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { .. })
            ) || matches!(
                effect,
                EffectAst::ControlFlow(control)
                    if matches!(
                        &control.node,
                        crate::model::control_flow::ControlFlowNodeAst::Condition { .. }
                    )
            )
        })
        && let Some((predicate, mut if_true, mut if_false)) = sentence_effects
            .pop()
            .and_then(take_self_replacement_condition)
    {
        if let Some(replacement) = materialize_search_count_self_replacement(
            state.effects,
            predicate.clone(),
            &if_true,
            sentence_tokens,
        ) {
            state.effects.push(replacement);
            return Ok(Some(PostParseFollowupResult::Handled {
                consumed_sentences: 1,
            }));
        }
        let Some(mut previous) = state.effects.pop() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous effect for 'instead' conditional rewrite".to_string(),
            ));
        };
        let previous_target = primary_target_from_effect(&previous);
        let mut previous_result_tag = explicit_self_replacement_result_tag(&previous);
        let replacement_qualifies_antecedent = if_true
            .iter()
            .find_map(primary_target_from_effect)
            .as_ref()
            .is_some_and(target_has_authored_it_qualification);
        if previous_result_tag.is_none()
            && previous_target.is_some()
            && replacement_qualifies_antecedent
        {
            let tag =
                crate::util::helper_tag_for_tokens(sentence_tokens, "self_replacement_antecedent");
            previous = EffectAst::TagAffected {
                effect: Box::new(previous),
                tag: crate::tag::TagRef::of(tag.clone()),
            };
            previous_result_tag = Some(tag.key.clone());
        }
        let previous_damage_target = primary_damage_target_from_effect(&previous);
        let previous_damage_source = primary_damage_source_from_effect(&previous);
        let predicate = bind_self_replacement_condition_to_previous_target(
            predicate,
            sentence_tokens,
            previous_target.as_ref(),
        );
        bind_nested_self_replacement_condition_to_previous_target(
            &mut if_true,
            sentence_tokens,
            previous_target.as_ref(),
        );
        if has_trailing_unpreventable_damage_rider(sentence_tokens)
            && !mark_last_deal_damage_unpreventable(&mut if_true)
        {
            return Err(CardTextError::ParseError(format!(
                "unpreventable-damage replacement rider has no damage effect (clause: '{}')",
                LexedClause::new(sentence_tokens).text(),
            )));
        }
        let (mut default_effects, carried_player) =
            default_effects_for_self_replacement(state.effects, previous);
        if let Some(mill_count) = default_effects
            .iter()
            .rev()
            .find_map(mill_count_from_effect)
        {
            replace_mill_event_amounts_with_value(&mut if_true, &mill_count);
        }
        if let Some(player) = carried_player {
            bind_that_player_subjects_in_effects(&mut if_true, player);
        }
        preserve_search_owner_anaphor_in_self_replacement(&mut default_effects);
        preserve_search_owner_anaphor_in_self_replacement(&mut if_true);
        if let Some(owner) = first_search_library_owner(&default_effects) {
            bind_self_replacement_search_owner(&mut if_true, &owner);
        }
        if let Some(target) = previous_result_tag
            .as_ref()
            .map(|tag| TargetAst::Tagged(crate::tag::TagRef::of(tag.clone()), None))
            .as_ref()
            .or(previous_target.as_ref())
        {
            replace_it_target_in_effects(&mut if_true, target);
        }
        if let Some(target) = previous_damage_target.as_ref() {
            replace_it_damage_target_in_effects(&mut if_true, target);
            replace_placeholder_damage_target_in_effects(&mut if_true, target);
        }
        if let Some(source) = previous_damage_source.as_ref()
            && !previous_damage_target.as_ref().is_some_and(|target| {
                normalize_anaphoric_damage_self_replacement(
                    &mut if_true,
                    sentence_tokens,
                    source,
                    target,
                )
            })
        {
            // In an authored damage self-replacement, a leading source
            // pronoun ("It deals ... instead") repeats the source of the
            // default damage event. It must not bind to the most recent
            // object antecedent, which may come from an additional cost.
            replace_anaphoric_damage_source_in_effects(&mut if_true, source);
        }
        for effect in default_effects.into_iter().rev() {
            if_false.insert(0, effect);
        }
        state.effects.push(EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability: false,
        });
        return Ok(Some(PostParseFollowupResult::Handled {
            consumed_sentences: 1,
        }));
    }
    if *sentence_effects == sentence_effects_baseline {
        Ok(None)
    } else {
        Ok(Some(PostParseFollowupResult::Annotated))
    }
}

pub(super) fn default_effects_for_self_replacement(
    prior_effects: &mut Vec<EffectAst>,
    previous: EffectAst,
) -> (Vec<EffectAst>, Option<PlayerAst>) {
    let mut default_effects = vec![previous];
    let mut carried_player = default_effects
        .iter()
        .rev()
        .find_map(carried_player_from_effect);

    let anchor_idx =
        if carried_player.is_none() && default_effects.iter().any(effect_has_that_player_subject) {
            let mut idx = prior_effects.len();
            let mut found = None;
            while idx > 0 {
                idx -= 1;
                if carried_player_from_effect(&prior_effects[idx]).is_some() {
                    found = Some(idx);
                    break;
                }
            }
            found
        } else {
            None
        };
    if let Some(anchor_idx) = anchor_idx {
        carried_player = carried_player_from_effect(&prior_effects[anchor_idx]);
        let mut anchored_default_effects = prior_effects.split_off(anchor_idx);
        anchored_default_effects.append(&mut default_effects);
        default_effects = anchored_default_effects;
    }

    if let Some(player) = carried_player {
        bind_that_player_subjects_in_effects(&mut default_effects, player);
    }

    (default_effects, carried_player)
}
