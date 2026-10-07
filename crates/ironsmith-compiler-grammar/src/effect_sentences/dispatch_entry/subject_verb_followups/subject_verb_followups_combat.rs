use super::*;
use crate::cards::builders::ForEachEffectAst;

/// Bind the omitted recipient before the independent conditional/clause
/// readers run. Copy the complete original action and change only its amount;
/// the existing self-replacement lowering owns the shared target declaration,
/// condition timing and mutually exclusive execution of the two arms.
pub(super) fn pre_rule_damage_amount_replacement(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_damage_amount_replacement(sentence_tokens) else {
        return Ok(None);
    };
    let Some(previous) = state.effects.last() else {
        return Ok(None);
    };
    let Some(source) = primary_damage_source_from_effect(previous) else {
        return Ok(None);
    };
    if !shape.repeats_source
        && !matches!(&source, TargetAst::Source(_))
        && !matches!(&source, TargetAst::Object(filter, None, _) if filter.source)
    {
        return Ok(None);
    }
    let Some((amount, used)) = crate::util::parse_value(shape.amount_tokens) else {
        return Ok(None);
    };
    if used != shape.amount_tokens.len() {
        return Ok(None);
    }
    // Event quantities ("twice that much") need their own exact event/LKI
    // owner. This family admits only a literal or the spell's X plus a literal.
    fn scalar_amount(value: &Value) -> bool {
        match value.unhinted() {
            Value::Fixed(_) | Value::X => true,
            Value::Add(left, right) => scalar_amount(left) && scalar_amount(right),
            _ => false,
        }
    }
    if !scalar_amount(&amount) {
        return Ok(None);
    }
    fn replace_amount(effect: &mut EffectAst, replacement: &Value) -> bool {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Damage(
                    DamageActionAst::DealDamage { amount, .. }
                    | DamageActionAst::DealDamageEqualToPower { amount, .. },
                ),
                ..
            }) => {
                *amount = replacement.clone();
                true
            }
            EffectAst::SourceSentence { effects, .. } | EffectAst::Sequence { effects }
                if effects.len() == 1 => replace_amount(&mut effects[0], replacement),
            _ => false,
        }
    }
    let mut replacement = previous.clone();
    if !replace_amount(&mut replacement, &amount) {
        return Ok(None);
    }
    let Some(predicate) = parse_trailing_if_predicate_lexed(shape.predicate_tokens) else {
        return Ok(None);
    };
    fn has_unbound_cast_time_control(predicate: &PredicateAst) -> bool {
        match predicate {
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControls {
                filter, ..
            }) => filter.has_as_you_cast_this_turn_surface(),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                has_unbound_cast_time_control(left) || has_unbound_cast_time_control(right)
            }
            PredicateAst::Not(inner) => has_unbound_cast_time_control(inner),
            _ => false,
        }
    }
    if has_unbound_cast_time_control(&predicate) {
        return Err(CardTextError::ParseError(
            "damage amount replacement requires retained cast-time control evidence".into(),
        ));
    }
    let predicate = bind_self_replacement_condition_to_previous_target(
        predicate,
        shape.predicate_tokens,
        primary_damage_target_from_effect(previous).as_ref(),
    );
    let previous = state.effects.pop().expect("the bound damage instruction exists");
    state.effects.push(EffectAst::SelfReplacement {
        predicate,
        if_true: vec![replacement],
        if_false: vec![previous],
        attach_to_previous_ability: false,
    });
    *state.carried_context = None;
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some("subject-verb verb=Deal subject=source recognizer=damage-amount-replacement"),
    }))
}

pub(super) fn primary_damage_source_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { .. }) => {
                Some(TargetAst::Source(None))
            }
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                source,
                ..
            })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                source, ..
            }) => Some(source.clone()),
            _ => None,
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_source_from_effect);
                }
            });
            found
        }
    }
}

pub(super) fn replace_anaphoric_damage_source_in_effects(
    effects: &mut [EffectAst],
    source: &TargetAst,
) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source: effect_source,
                    ..
                })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage {
                    source: effect_source,
                    ..
                }) if target_references_it(effect_source) => {
                    *effect_source = source.clone();
                }
                _ => {}
            },
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                replace_anaphoric_damage_source_in_effects(nested, source);
            }),
        }
    }
}

pub(super) fn sole_damage_payload(effects: &[EffectAst]) -> Option<(Value, bool)> {
    let [effect] = effects else {
        return None;
    };
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamage {
                    amount,
                    unpreventable,
                    ..
                })
                | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    amount,
                    unpreventable,
                    ..
                }),
            ..
        }) => Some((amount.clone(), *unpreventable)),
        EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })
        | EffectAst::Coordinated { effects, .. } => sole_damage_payload(effects),
        _ => None,
    }
}

/// Collapse an authored singular damage-source anaphor before reference
/// resolution can interpret it as the most recent object result. In a
/// self-replacement, both "It" and "that creature" repeat the source and
/// target of the default damage event; neither refers to an object used to pay
/// an earlier cost.
pub(super) fn normalize_anaphoric_damage_self_replacement(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
    source: &TargetAst,
    target: &TargetAst,
) -> bool {
    if !effect_grammar::followup_shapes::is_anaphoric_damage_self_replacement(tokens) {
        return false;
    }
    let Some((amount, unpreventable)) = sole_damage_payload(effects) else {
        return false;
    };
    *effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
            source: source.clone(),
            amount,
            target: target.clone(),
            unpreventable,
        }),
    )];
    true
}
