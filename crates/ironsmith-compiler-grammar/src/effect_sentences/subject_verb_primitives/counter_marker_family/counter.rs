use super::*;

pub fn parse_if_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_if_enters_additional_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let put_counter = EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span()),
        None,
        false,
    );
    let apply_only_if_creature = EffectAst::Conditional {
        predicate: PredicateAst::ItMatches(ObjectFilter::creature()),
        if_true: vec![put_counter],
        if_false: Vec::new(),
    };

    Ok(Some(vec![EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![apply_only_if_creature],
    }]))
}

pub fn parse_tagged_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_tagged_enters_additional_tokens(clause.tokens()) else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span()),
        None,
        false,
    )]))
}

pub fn parse_tagged_conditional_entry_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        counter_shapes::parse_tagged_conditional_entry_counters_tokens(clause.tokens())
    else {
        return Ok(None);
    };

    let effects = shape
        .arms
        .into_iter()
        .map(|arm| {
            let put_counter = EffectAst::subject_verb_put_counters(
                arm.descriptor.counter_type,
                Value::Fixed(arm.descriptor.count as i32)
                    .with_surface_hint(
                        ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter,
                    )
                    .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
                    .with_surface_hint(
                        ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence,
                    ),
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span()),
                None,
                false,
            );
            EffectAst::Conditional {
                predicate: PredicateAst::ItMatches(
                    ObjectFilter::default().with_type(arm.object_type),
                ),
                if_true: vec![put_counter],
                if_false: Vec::new(),
            }
        })
        .collect();

    Ok(Some(effects))
}

pub fn parse_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_put_with_additional_tokens(clause.tokens()) else {
        return Ok(None);
    };
    lower_put_with_additional_counter(shape, clause.span()).map(Some)
}

pub(super) fn lower_put_with_additional_counter(
    shape: counter_shapes::PutWithAdditionalCounterShape<'_>,
    span: Option<TextSpan>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = parse_effect_chain_inner(shape.move_tokens)?;
    if effects.is_empty()
        || !effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::MoveToZone {
                        zone: Zone::Battlefield,
                        ..
                    } | SubjectVerbActionAst::ReturnToBattlefield { .. }
                        | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                    ..
                })
            )
        })
    {
        return Ok(Vec::new());
    }

    effects.push(EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), span),
        None,
        false,
    ));

    Ok(effects)
}

pub fn parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_sacrifice_then_put_additional_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    lower_sacrifice_then_put_additional(shape, clause.span()).map(Some)
}

pub fn parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        counter_shapes::parse_if_sacrifice_then_put_additional_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    let effects = lower_sacrifice_then_put_additional(shape.effect, clause.span())?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::Conditional {
        predicate: parse_predicate_lexed(shape.predicate_tokens)?,
        if_true: effects,
        if_false: Vec::new(),
    }]))
}

pub fn parse_each_player_return_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_each_player_return_additional_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    let mut per_player_effects = parse_effect_chain_inner(shape.return_tokens)?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        Value::Fixed(shape.descriptor.count as i32),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span()),
        None,
        false,
    ));

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}
