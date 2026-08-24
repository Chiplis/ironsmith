use super::*;

pub(super) fn is_singular_explicit_return_to_battlefield(effect: &EffectAst) -> bool {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return false;
    };
    let SubjectVerbActionAst::ReturnToBattlefield { target, .. } = &subject_verb.action else {
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
            action: SubjectVerbActionAst::ReturnToBattlefield { target, .. },
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
    sentence_effects.push(EffectAst::DelayedTriggerForDuration {
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
    });
    Ok(None)
}
