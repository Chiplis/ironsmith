use super::*;

pub fn parse_exile_until_match_put_counters_on_match(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                    stop_rule,
                    ..
                },
            ..
        })) if consult_stop_rule_is_single_match(stop_rule)
    ) {
        return Ok(None);
    }

    let Ok(counter_effects) = effect_sentences::parse_effect_sentence_lexed(second) else {
        return Ok(None);
    };
    let [EffectAst::SubjectVerb(counter_effect)] = counter_effects.as_slice() else {
        return Ok(None);
    };
    let mut counter_effect = counter_effect.clone();
    let SubjectVerbActionAst::PutCounters { target, .. } = &mut counter_effect.action else {
        return Ok(None);
    };
    if !target_references_it(target) {
        return Ok(None);
    }
    let reference_span = match &*target {
        TargetAst::Tagged(_, span) | TargetAst::Source(span) => *span,
        TargetAst::Object(_, _, span) => *span,
        _ => None,
    };
    *target = TargetAst::Tagged(parts.match_tag.clone(), reference_span);

    let mut effects = parts.effects;
    effects.push(EffectAst::SubjectVerb(counter_effect));
    Ok(Some(effects))
}
