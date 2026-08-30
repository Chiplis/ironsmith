use super::*;

pub(super) fn parse_passive_color_type_addition_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        effect_grammar::labeled_dispatch::parse_passive_color_type_addition_shape(tokens)
    else {
        return Ok(None);
    };

    let target = if shape.tagged_subject {
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(shape.subject_tokens)?
    };

    let mut effects = Vec::new();
    if !shape.colors.is_empty() {
        let color_effect = if shape.adds_colors {
            EffectAst::subject_verb_add_colors(target.clone(), shape.colors, Until::Forever)
        } else {
            EffectAst::subject_verb_set_colors(target.clone(), shape.colors, Until::Forever)
        };
        effects.push(color_effect);
    }
    if !shape.card_types.is_empty() {
        effects.push(EffectAst::subject_verb_add_card_types(
            target.clone(),
            shape.card_types,
            Until::Forever,
        ));
    }
    if !shape.subtypes.is_empty() {
        effects.push(EffectAst::subject_verb_add_subtypes(
            target,
            shape.subtypes,
            Until::Forever,
        ));
    }

    Ok((!effects.is_empty()).then_some(effects))
}
