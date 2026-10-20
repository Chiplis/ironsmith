use super::*;

fn follows_source_until_exile(state: &SentenceDispatchState<'_>) -> bool {
    matches!(
        state.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(
                ZoneMoveActionAst::ExileUntilSourceLeaves {
                    duration: ironsmith_core::ExileUntilDuration::SourceLeavesBattlefield,
                    leave_watcher: None,
                    ..
                }
            ),
            ..
        }))
    )
}

fn is_source_until_exile_return_header(tokens: &[OwnedLexToken]) -> bool {
    let Some(comma) = crate::slice_primitives::select_position(tokens, OwnedLexToken::is_comma)
    else {
        return false;
    };
    let words = crate::lexer::parser_token_word_refs(&tokens[..comma]);
    let Some((intro, subject)) = words.split_first() else {
        return false;
    };
    matches!(*intro, "when" | "whenever")
        && matches!(
            subject,
            ["an", "exiled", "card", "enters", "under", "your", "control", "this", "way"]
                | ["an", "exiled", "card", "enters", "the", "battlefield", "under", "your", "control", "this", "way"]
                | ["a", "card", "exiled", "this", "way", "enters", "under", "your", "control"]
                | ["a", "card", "exiled", "this", "way", "enters", "the", "battlefield", "under", "your", "control"]
        )
}

/// A card exiled until this source leaves returns by the original exile
/// duration, not by a new source ability. The ensuing trigger is installed by
/// that exile ability's resolution and must survive the source's departure.
pub(super) fn pre_rule_source_until_exile_return_enters(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !follows_source_until_exile(state) || !is_source_until_exile_return_header(sentence_tokens)
    {
        return Ok(None);
    }
    let comma = crate::slice_primitives::select_position(sentence_tokens, OwnedLexToken::is_comma)
        .expect("the recognized return header has a comma");
    let body = trim_commas(&sentence_tokens[comma + 1..]);
    if body.is_empty() {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan::new(body))))
}

pub(super) fn post_rule_source_until_exile_return_enters(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    if !follows_source_until_exile(state)
        || !is_source_until_exile_return_header(sentence_tokens)
        || sentence_effects.is_empty()
    {
        return Ok(None);
    }
    for effect in sentence_effects.iter_mut() {
        ironsmith_core::tag::TagKeyWalk::map_tag_keys(effect, &mut |key| {
            if key.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                || key.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
            {
                *key = crate::tag::CompilerReferenceTag::Triggering.bind().into();
            }
        });
    }
    let filter = crate::target::ObjectFilter::default()
        .controlled_by(crate::target::PlayerFilter::You)
        .match_tagged(
            ironsmith_core::SOURCE_EXILED_THIS_RESOLUTION_TAG,
            ironsmith_core::TaggedOpbjectRelation::IsTaggedObject,
        );
    let mut zone = ironsmith_core::trigger_model::ZoneChangeTrigger::new()
        .from(crate::zone::Zone::Exile)
        .to(crate::zone::Zone::Battlefield)
        .filter(filter);
    zone.duration_return_from_source = true;
    let effects = std::mem::take(sentence_effects);
    sentence_effects.push(EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
        trigger: crate::cards::builders::TriggerSpec::ZoneChange(zone),
        effects,
        one_shot: true,
        duration: Until::Forever,
        either_of_watched_objects: false,
        while_any_tagged_object_in_zone: None,
    }));
    Ok(Some(PostParseFollowupResult::Annotated))
}

pub(super) fn is_singular_explicit_return_to_battlefield(effect: &EffectAst) -> bool {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return false;
    };
    let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { target, .. }) =
        &subject_verb.action
    else {
        return false;
    };
    if !target_is_explicitly_chosen(target) {
        return false;
    }
    match target {
        TargetAst::WithCount(_, count) | TargetAst::WithCountValue(_, count, _) => {
            count.is_single()
        }
        _ => true,
    }
}

pub(super) fn is_explicit_return_to_battlefield(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { target, .. }),
            ..
        }) if target_is_explicitly_chosen(target)
    )
}

pub(super) fn pre_rule_returned_permanent_enters(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(comma_idx) =
        crate::slice_primitives::select_position(sentence_tokens, OwnedLexToken::is_comma)
    else {
        return Ok(None);
    };
    let trigger_words = crate::lexer::parser_token_word_refs(&sentence_tokens[..comma_idx]);
    if !crate::word_primitives::parse_any_sequence_complete(
        &trigger_words,
        &[
            &["when", "that", "permanent", "enters"],
            &["when", "that", "card", "enters"],
        ],
    ) || !state
        .effects
        .last()
        .is_some_and(is_explicit_return_to_battlefield)
    {
        return Ok(None);
    }
    let body = trim_commas(&sentence_tokens[comma_idx + 1..]);
    if body.is_empty() {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan::new(
        body,
    ))))
}

/// A spell can create a one-shot delayed trigger tied to the exact permanent
/// returned by its preceding instruction. Keep the trigger typed instead of
/// allowing the sentence parser to flatten its payload into an immediate
/// follow-up effect.
pub(super) fn post_rule_returned_permanent_enters(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(comma_idx) =
        crate::slice_primitives::select_position(sentence_tokens, OwnedLexToken::is_comma)
    else {
        return Ok(None);
    };
    if !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(&sentence_tokens[..comma_idx]),
        &["when", "that", "permanent", "enters"],
    ) || !state
        .effects
        .last()
        .is_some_and(is_singular_explicit_return_to_battlefield)
        || sentence_effects.is_empty()
    {
        return Ok(None);
    }

    let effects = std::mem::take(sentence_effects);
    sentence_effects.push(EffectAst::Delayed(
        DelayedEffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface {
                surface: crate::target::SourceReferenceSurface::ThisPermanentType(
                    "that permanent".to_string(),
                ),
                subject_number: ironsmith_core::trigger_model::TriggerSubjectNumber::Singular,
                origin_condition: None,
            },
            effects,
            one_shot: true,
            duration: Until::Forever,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
        },
    ));
    Ok(Some(PostParseFollowupResult::Annotated))
}
