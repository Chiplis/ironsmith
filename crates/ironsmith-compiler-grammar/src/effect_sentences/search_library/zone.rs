use super::*;

pub fn parse_for_each_put_into_graveyard_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::PutIntoGraveyard {
        return Ok(None);
    }
    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    let effects = if let Some(filter_tokens) = shape.iterated_filter_tokens {
        let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
        filter.zone = None;
        filter.set_put_into_graveyard_this_way_surface(true);
        vec![EffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(filter),
            if_true: effects,
            if_false: Vec::new(),
        }]
    } else {
        effects
    };

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: crate::tag::CompilerReferenceTag::It.as_str().into(),
        effects,
    }]))
}
