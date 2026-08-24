use super::*;

pub fn parse_draw_then_connive_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_draw_then_connive_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.draw_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(shape.connive_tokens)? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

pub fn parse_sentence_draw_then_connive(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(clause)
}

pub(super) fn lower_sacrifice_then_put_additional(
    shape: counter_shapes::SacrificeThenPutAdditionalShape<'_>,
    span: Option<TextSpan>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut put_effects = lower_put_with_additional_counter(shape.put, span)?;
    if put_effects.is_empty() {
        return Ok(Vec::new());
    }
    let mut effects = if shape.plain_word_sacrifice {
        vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            ObjectFilter {
                source: true,
                ..Default::default()
            },
            1,
            None,
        )]
    } else {
        parse_effect_chain_inner(shape.sacrifice_tokens)?
    };
    if effects.is_empty() {
        return Ok(Vec::new());
    }
    effects.append(&mut put_effects);
    Ok(effects)
}
