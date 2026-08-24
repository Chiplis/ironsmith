use super::*;

/// A trailing reflexive result of a library consult must attach to the consult,
/// not to the intervening cleanup instruction. Keep the cleanup last in the
/// runtime sequence while preserving its explicit full revealed-set tag.
pub fn parse_consult_cleanup_then_typed_when_result(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(consult) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [
        consult @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
            ..
        }),
    ] = consult.as_slice()
    else {
        return Ok(None);
    };

    let Ok(cleanup) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let [
        cleanup @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
            ..
        }),
    ] = cleanup.as_slice()
    else {
        return Ok(None);
    };

    let Ok(followup) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };
    let [when_result @ EffectAst::WhenResult { .. }] = followup.as_slice() else {
        return Ok(None);
    };

    Ok(Some(vec![
        consult.clone(),
        when_result.clone(),
        cleanup.clone(),
    ]))
}
