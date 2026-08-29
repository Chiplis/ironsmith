use super::*;

pub(super) fn bind_sacrificed_snapshot_controller(effect: &mut EffectAst) {
    match effect {
        EffectAst::MayByPlayer { player, .. } if *player == PlayerAst::ItsController => {
            *player = PlayerAst::That;
        }
        EffectAst::SubjectVerb(subject_verb) => {
            if subject_verb.subject.player == PlayerAst::ItsController {
                subject_verb.subject.player = PlayerAst::That;
            }
            if let SubjectVerbActionAst::SearchLibrary { player, .. } = &mut subject_verb.action
                && *player == PlayerAst::ItsController
            {
                *player = PlayerAst::That;
            }
        }
        _ => {}
    }

    crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            bind_sacrificed_snapshot_controller(nested_effect);
        }
    });
}

/// Parse a typed iterator over the actual objects sacrificed by the preceding
/// instruction.  The last-known predicate deliberately stays inside the
/// object loop: it preserves both the exact sacrificed subset and the
/// controller each object had before leaving the battlefield.
pub fn parse_for_each_sacrificed_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::Sacrificed {
        return Ok(None);
    }
    let filter_tokens = shape.iterated_filter_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing object type in sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty object type in sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
    // Sacrifice result predicates use the permanent's battlefield snapshot,
    // not its current graveyard object.
    filter.zone = None;

    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... sacrificed this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let mut effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after sacrificed-this-way iterator (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    for effect in &mut effects {
        bind_sacrificed_snapshot_controller(effect);
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(filter),
            if_true: effects,
            if_false: Vec::new(),
        }],
    }]))
}
