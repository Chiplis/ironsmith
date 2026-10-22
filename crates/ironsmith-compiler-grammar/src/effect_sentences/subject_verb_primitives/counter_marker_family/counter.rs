use super::*;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::ZoneMoveActionAst;

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
    let apply_only_if_matching_object =
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ItMatches(shape.object_filter),
            if_true: vec![put_counter],
            if_false: Vec::new(),
        });

    Ok(Some(vec![EffectAst::Conditionals(
        ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![apply_only_if_matching_object],
        },
    )]))
}

pub fn parse_tagged_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "that creature enters with X additional +1/+1 counters on it, where X
    // is ..." (Runadi): the same external entry replacement with the count
    // left as X for the enclosing where-X binding to define.
    let Some((shape, x_count)) =
        counter_shapes::parse_tagged_enters_additional_tokens_with_x(clause.tokens())
    else {
        return Ok(None);
    };
    let count = if x_count {
        Value::X
    } else {
        Value::Fixed(shape.descriptor.count as i32)
    };

    Ok(Some(vec![EffectAst::subject_verb_put_counters(
        shape.descriptor.counter_type,
        count
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
            EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ItMatches(
                    ObjectFilter::default().with_type(arm.object_type),
                ),
                if_true: vec![put_counter],
                if_false: Vec::new(),
            })
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
                    action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                        zone: Zone::Battlefield,
                        ..
                    }) | SubjectVerbActionAst::ZoneMoves(
                        ZoneMoveActionAst::ReturnToBattlefield { .. }
                    ) | SubjectVerbActionAst::ZoneMoves(
                        ZoneMoveActionAst::ReturnAllToBattlefield { .. }
                    ),
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
    Ok(Some(vec![EffectAst::Conditionals(
        ConditionalEffectAst::Conditional {
            predicate: parse_predicate_lexed(shape.predicate_tokens)?,
            if_true: effects,
            if_false: Vec::new(),
        },
    )]))
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
                action: SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::ReturnToBattlefield { .. }
                ) | SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::ReturnAllToBattlefield { .. }
                ),
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

    Ok(Some(vec![EffectAst::ForEach(
        ForEachEffectAst::ForEachPlayer {
            effects: per_player_effects,
        },
    )]))
}

/// "The token enters with X +1/+1 counters on it" (Printlifter Ooze) after a
/// creation instruction: the counters are part of the created token's entry
/// event (CR 122.6). Lowering folds this inline-entry put into the preceding
/// token creation; a surrounding "where X is ..." binds the amount.
pub fn parse_created_token_enters_with_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(clause.tokens());
    // "The token enters with half that many +1/+1 counters on it, rounded
    // down" (Ochre Jelly): the only punctuation allowed is the comma before
    // the rounding tail.
    let tokens: Vec<crate::lexer::OwnedLexToken> = tokens
        .iter()
        .filter(|token| token.as_word().is_some() || token.kind != crate::lexer::TokenKind::Comma)
        .cloned()
        .collect();
    let tokens = tokens.as_slice();
    if tokens.iter().any(|token| token.as_word().is_none()) {
        return Ok(None);
    }
    let words = crate::lexer::token_word_refs(tokens);
    let mut lead = match words.as_slice() {
        ["the" | "that", "token", "enters", "with", ..] => 4usize,
        ["the" | "those", "tokens", "enter", "with", ..] => 4usize,
        _ => return Ok(None),
    };
    let half = words.get(lead) == Some(&"half");
    if half {
        lead += 1;
    }
    let Some((amount, used)) = crate::util::parse_value(&tokens[lead..]) else {
        return Ok(None);
    };
    let mut rest = &words[lead + used..];
    let amount = if half {
        let Some(stripped) = rest.strip_suffix(&["rounded", "down"]) else {
            return Ok(None);
        };
        rest = stripped;
        Value::HalfRoundedDown(Box::new(amount))
    } else {
        amount
    };
    let counter_word = match rest {
        [counter, "counter" | "counters", "on", "it" | "them"]
        | [counter, "counter" | "counters", "on", "each", "of", "them"] => *counter,
        _ => return Ok(None),
    };
    let Some(counter_type) =
        crate::grammar::filters::parse_counter_type_words(&[counter_word, "counter"])
    else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::subject_verb_put_counters(
        counter_type,
        amount
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupSeparateSentence),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span()),
        None,
        false,
    )]))
}
