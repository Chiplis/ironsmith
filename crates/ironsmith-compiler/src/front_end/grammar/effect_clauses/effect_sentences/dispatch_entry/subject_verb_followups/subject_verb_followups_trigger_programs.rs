use super::*;

pub(super) fn bind_demonstrative_land_match_to_triggering_object(
    predicate: PredicateAst,
) -> PredicateAst {
    match predicate {
        PredicateAst::SourceMatches(filter)
        | PredicateAst::ItMatches(filter)
        | PredicateAst::TargetMatches(filter)
            if filter.demonstrative_antecedent_surface()
                == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land) =>
        {
            PredicateAst::TaggedMatches(crate::tag::CompilerReferenceTag::Triggering.key(), filter)
        }
        PredicateAst::Not(inner) => PredicateAst::Not(Box::new(
            bind_demonstrative_land_match_to_triggering_object(*inner),
        )),
        PredicateAst::And(left, right) => PredicateAst::And(
            Box::new(bind_demonstrative_land_match_to_triggering_object(*left)),
            Box::new(bind_demonstrative_land_match_to_triggering_object(*right)),
        ),
        PredicateAst::Or(left, right) => PredicateAst::Or(
            Box::new(bind_demonstrative_land_match_to_triggering_object(*left)),
            Box::new(bind_demonstrative_land_match_to_triggering_object(*right)),
        ),
        other => other,
    }
}

pub(super) fn replace_event_amount_with_value(value: &mut Value, replacement: &Value) {
    match value {
        Value::EventValue(crate::effect::EventValueSpec::Amount) => {
            *value = replacement.clone();
        }
        Value::EventValueOffset(crate::effect::EventValueSpec::Amount, offset) => {
            *value = Value::Add(
                Box::new(replacement.clone()),
                Box::new(Value::Fixed(*offset)),
            );
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            replace_event_amount_with_value(left, replacement);
            replace_event_amount_with_value(right, replacement);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner)
        | Value::SurfaceHinted { value: inner, .. } => {
            replace_event_amount_with_value(inner, replacement);
        }
        _ => {}
    }
}

/// Keep an object-dependent continuation inside the reflexive trigger that
/// establishes its antecedent. A `WhenResult` lowers to a new stack entry, so
/// leaving the continuation as an outer sibling would make it execute before
/// the tagged object exists.
pub(super) fn post_rule_reflexive_object_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let references_reflexive_object =
        crate::compile_support::effects_reference_it_tag(sentence_effects)
            || crate::compile_support::effects_reference_its_controller(sentence_effects);
    if sentence_effects.is_empty() || !references_reflexive_object {
        return Ok(None);
    }
    let Some(EffectAst::WhenResult {
        effects: reflexive_effects,
        ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };

    reflexive_effects.append(sentence_effects);
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

/// A later delayed trigger whose subject is "the targeted ..." watches the
/// exact object selected by the nearest earlier target declaration. Keeping
/// only the noun filter (for example, `creature`) makes every matching object
/// capable of firing the delayed trigger.
pub(super) fn post_rule_targeted_object_delayed_leave(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::lexer::token_word_refs(sentence_tokens);
    if !crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[
            &["when", "the", "targeted"],
            &["whenever", "the", "targeted"],
        ],
    ) {
        return Ok(None);
    }
    let Some(tag) = state.effects.iter().rev().find_map(|effect| match effect {
        EffectAst::ChooseObjects { tag, .. } => Some(tag.clone()),
        _ => None,
    }) else {
        return Ok(None);
    };
    for effect in sentence_effects {
        if let EffectAst::DelayedTriggerThisTurn { trigger, .. }
        | EffectAst::DelayedTriggerForDuration { trigger, .. } = effect
        {
            bind_targeted_leaves_filter(trigger, &tag);
        }
    }
    Ok(None)
}

pub(super) fn post_rule_delayed_trigger_result_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let [EffectAst::IfResult { .. } | EffectAst::WhenResult { .. }] = sentence_effects.as_slice()
    else {
        return Ok(None);
    };
    let Some(
        EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedTriggerForDuration { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextCleanupStep { effects, .. }
        | EffectAst::DelayedUntilNextUntapStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. }
        | EffectAst::DelayedUntilNextMainPhase { effects, .. }
        | EffectAst::DelayedUntilNextFirstMainPhase { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects },
    ) = state.effects.last_mut()
    else {
        return Ok(None);
    };
    effects.append(sentence_effects);
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

pub(super) fn trailing_delayed_trigger_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedTriggerForDuration { effects, .. } => Some(effects),
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::Coordinated { effects, .. } => effects
            .last_mut()
            .and_then(trailing_delayed_trigger_effects_mut),
        EffectAst::Conditional { if_true, .. } => if_true
            .last_mut()
            .and_then(trailing_delayed_trigger_effects_mut),
        _ => None,
    }
}

pub(super) fn append_copy_retarget_to_trailing_delayed_trigger(
    previous: &mut EffectAst,
    followups: &mut Vec<EffectAst>,
) -> bool {
    if !effects_are_copy_retarget_followup(followups) {
        return false;
    }
    let Some(delayed_effects) = trailing_delayed_trigger_effects_mut(previous) else {
        return false;
    };
    if !effects_copy_a_stack_object(delayed_effects) {
        return false;
    }
    delayed_effects.append(followups);
    true
}

/// Sequence dispatch can claim a complete optional retarget sentence before
/// the post-parse follow-up registry runs. Repair that exact adjacency at the
/// public family root as well: a retarget of the copied-stack result belongs
/// inside the immediately preceding delayed trigger that creates that result,
/// never on the outer resolution program where the copy does not exist yet.
pub(in super::super) fn transport_copy_retarget_into_trailing_delayed_trigger(
    effects: &mut Vec<EffectAst>,
) {
    let mut index = 1usize;
    while index < effects.len() {
        let mut followups = vec![effects[index].clone()];
        if append_copy_retarget_to_trailing_delayed_trigger(&mut effects[index - 1], &mut followups)
        {
            effects.remove(index);
        } else {
            index += 1;
        }
    }
}

pub(super) fn post_rule_delayed_trigger_copy_retarget_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    if !append_copy_retarget_to_trailing_delayed_trigger(previous, sentence_effects) {
        return Ok(None);
    }
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}
