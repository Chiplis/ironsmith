use super::*;

pub fn parse_choose_then_do_same_for_filter_then_return_to_battlefield(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = effect_sentences::parse_sentence_choose_then_do_same_for_filter(
        effect_sentences::SubjectVerbPrimitiveClause::new(sentences[sentence_idx].lowered()),
    )?
    else {
        return Ok(None);
    };

    let Some(return_shape) = effect_grammar::parse_return_tagged_battlefield_shape(
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    effects.push(EffectAst::subject_verb_return_to_battlefield(
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            effect_sentences::span_from_tokens(sentences[sentence_idx + 1].lowered()),
        ),
        return_shape.tapped,
        false,
        false,
        ReturnControllerAst::Preserve,
        None,
    ));
    Ok(Some(effects))
}

