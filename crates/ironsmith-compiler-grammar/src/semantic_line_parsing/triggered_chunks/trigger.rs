use super::*;
use crate::cards::builders::ConditionalEffectAst;

/// "Whenever a creature enters, if there are two or more other creatures on
/// the battlefield" (Portcullis): when the trigger's subject is not the source,
/// "other" in the condition excludes the entering object.
fn exclude_entering_object_from_other_counts(
    trigger: &TriggerSpec,
    predicate: PredicateAst,
) -> PredicateAst {
    let (TriggerSpec::EntersBattlefield { filter, .. }
    | TriggerSpec::EntersBattlefieldOneOrMore { filter, .. }) = trigger
    else {
        return predicate;
    };
    if filter.other || filter.source {
        return predicate;
    }
    fn exclude(filter: &mut crate::ObjectFilter) {
        if !filter.other {
            return;
        }
        let tag: crate::tag::TagKey = crate::tag::CompilerReferenceTag::It.bind().into();
        if !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == tag
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsNotTaggedObject
        }) {
            filter
                .tagged_constraints
                .push(crate::target::TaggedObjectConstraint {
                    tag,
                    relation: crate::target::TaggedOpbjectRelation::IsNotTaggedObject,
                });
        }
    }
    fn rewrite(predicate: PredicateAst) -> PredicateAst {
        match predicate {
            PredicateAst::ValueComparison {
                left: crate::effect::Value::Count(mut filter),
                operator,
                right,
            } => {
                exclude(&mut filter);
                PredicateAst::ValueComparison {
                    left: crate::effect::Value::Count(filter),
                    operator,
                    right,
                }
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasAtLeast {
                player,
                mut filter,
                count,
            }) => {
                exclude(&mut filter);
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasAtLeast {
                    player,
                    filter,
                    count,
                })
            }
            PredicateAst::And(left, right) => {
                PredicateAst::And(Box::new(rewrite(*left)), Box::new(rewrite(*right)))
            }
            PredicateAst::Or(left, right) => {
                PredicateAst::Or(Box::new(rewrite(*left)), Box::new(rewrite(*right)))
            }
            PredicateAst::Not(inner) => PredicateAst::Not(Box::new(rewrite(*inner))),
            other => other,
        }
    }
    rewrite(predicate)
}

/// "Whenever Alex Wilder or another creature you control enters, if you cast
/// it from anywhere other than your hand, ...": when the entering object can
/// be something other than the source, the cast-origin clause names the
/// entering (triggering) object, not the source spell. Rewriting it also
/// keeps the clause from establishing the source as the antecedent of the
/// effect's later `it`.
fn bind_cast_origin_predicate_to_entering_object(
    trigger: &TriggerSpec,
    predicate: PredicateAst,
) -> PredicateAst {
    fn has_non_source_entering_subject(trigger: &TriggerSpec) -> bool {
        match trigger {
            TriggerSpec::WithIntro { trigger, .. } => has_non_source_entering_subject(trigger),
            TriggerSpec::AnyOf(triggers) => triggers.iter().any(has_non_source_entering_subject),
            TriggerSpec::Either(left, right) => {
                has_non_source_entering_subject(left) || has_non_source_entering_subject(right)
            }
            TriggerSpec::EntersBattlefield { filter, .. }
            | TriggerSpec::EntersBattlefieldOneOrMore { filter, .. } => !filter.source,
            _ => false,
        }
    }
    fn rewrite(predicate: PredicateAst) -> PredicateAst {
        use crate::cards::builders::TurnHistoryPredicateAst;
        match predicate {
            PredicateAst::ThisSpellWasCastFromNonHand => PredicateAst::And(
                Box::new(PredicateAst::TurnHistory(
                    TurnHistoryPredicateAst::TriggeringObjectWasCast,
                )),
                Box::new(PredicateAst::Not(Box::new(PredicateAst::TurnHistory(
                    TurnHistoryPredicateAst::TriggeringObjectWasCastFromZone(
                        crate::zone::Zone::Hand,
                    ),
                )))),
            ),
            PredicateAst::ThisSpellWasCastFromZone(zone) => PredicateAst::TurnHistory(
                TurnHistoryPredicateAst::TriggeringObjectWasCastFromZone(zone),
            ),
            PredicateAst::And(left, right) => {
                PredicateAst::And(Box::new(rewrite(*left)), Box::new(rewrite(*right)))
            }
            PredicateAst::Or(left, right) => {
                PredicateAst::Or(Box::new(rewrite(*left)), Box::new(rewrite(*right)))
            }
            PredicateAst::Not(inner) => PredicateAst::Not(Box::new(rewrite(*inner))),
            other => other,
        }
    }
    if !has_non_source_entering_subject(trigger) {
        return predicate;
    }
    rewrite(predicate)
}

/// "Whenever you cast an instant or sorcery spell, if Taigam attacked this
/// turn, that spell gains rebound": a source condition does not retarget an
/// `it`-bound grant to the source over the spell the trigger announced.
fn trigger_announces_cast_spell(trigger: &TriggerSpec) -> bool {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_announces_cast_spell(trigger),
        TriggerSpec::SpellCast { .. } | TriggerSpec::SpellCastSameNameCardInZone { .. } => true,
        _ => false,
    }
}

pub fn apply_explicit_intervening_if_to_triggered_chunk(
    chunk: LineAst,
    explicit_intervening_if: Option<PredicateAst>,
) -> Result<LineAst, CardTextError> {
    let Some(predicate) = explicit_intervening_if else {
        return Ok(chunk);
    };
    let predicate = match &chunk {
        LineAst::Triggered { trigger, .. } => {
            bind_cast_origin_predicate_to_entering_object(trigger, predicate)
        }
        LineAst::Ability(parsed) => match parsed.trigger_spec.as_deref() {
            Some(trigger) => bind_cast_origin_predicate_to_entering_object(trigger, predicate),
            None => predicate,
        },
        _ => predicate,
    };
    match chunk {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
            let predicate = exclude_entering_object_from_other_counts(&trigger, predicate);
            let random_count_antecedent_predicate = predicate.clone();
            let predicate = link_spell_cast_mana_spent_predicate(&trigger, predicate);
            let (trigger, predicate) = absorb_predicate_into_trigger(trigger, predicate);
            let (trigger, effects) =
                absorb_single_conditional_effect_into_trigger(trigger, effects);
            let mut effects = effects;
            bind_random_count_condition_antecedent_in_effects(
                &mut effects,
                &random_count_antecedent_predicate,
            );
            let Some(predicate) = predicate else {
                return Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                });
            };
            if let Some(antecedent) = predicate_object_filter_antecedent(&predicate) {
                bind_condition_antecedent_in_effects(
                    &mut effects,
                    &antecedent,
                    ConditionAntecedentBinding::TaggedItOnly,
                );
            }
            bind_random_count_condition_antecedent_in_effects(&mut effects, &predicate);
            if let Some(counter_type) = predicate_source_counter_antecedent(&predicate) {
                bind_condition_counter_antecedent_in_effects(&mut effects, counter_type);
            }
            if predicate.establishes_source_object_antecedent() {
                if trigger_announces_cast_spell(&trigger) {
                    ironsmith_compiler_semantic::condition_antecedent::resolve_it_counter_and_animation_targets_to_source(
                        &mut effects,
                    );
                    ironsmith_compiler_semantic::condition_antecedent::resolve_it_grant_targets_to_triggering_spell(
                        &mut effects,
                    );
                } else {
                    resolve_it_animations_to_source(&mut effects);
                }
            }
            if matches!(
                effects.as_slice(),
                [EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_false, .. })] if if_false.is_empty()
            ) {
                Ok(LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn,
                })
            } else {
                Ok(LineAst::Triggered {
                    trigger,
                    effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                        predicate,
                        if_true: effects,
                        if_false: Vec::new(),
                    })],
                    max_triggers_per_turn,
                })
            }
        }
        LineAst::Ability(mut parsed) => {
            let announces_cast_spell = parsed
                .trigger_spec
                .as_deref()
                .is_some_and(trigger_announces_cast_spell);
            if let Some(mut effects_ast) = parsed.effects_ast.take() {
                // A bare `it` continues the source the condition names; a
                // demonstrative ("that spell") names the announced spell.
                parsed.reference_imports.source_object_antecedent |=
                    predicate.establishes_source_object_antecedent();
                if let Some(antecedent) = predicate_object_filter_antecedent(&predicate) {
                    bind_condition_antecedent_in_effects(
                        &mut effects_ast,
                        &antecedent,
                        ConditionAntecedentBinding::TaggedItOnly,
                    );
                }
                bind_random_count_condition_antecedent_in_effects(&mut effects_ast, &predicate);
                if let Some(counter_type) = predicate_source_counter_antecedent(&predicate) {
                    bind_condition_counter_antecedent_in_effects(&mut effects_ast, counter_type);
                }
                if predicate.establishes_source_object_antecedent() {
                    if announces_cast_spell {
                        ironsmith_compiler_semantic::condition_antecedent::resolve_it_counter_and_animation_targets_to_source(
                            &mut effects_ast,
                        );
                        ironsmith_compiler_semantic::condition_antecedent::resolve_it_grant_targets_to_triggering_spell(
                            &mut effects_ast,
                        );
                    } else {
                        resolve_it_animations_to_source(&mut effects_ast);
                    }
                }
                parsed.effects_ast = Some(effects_ast);
            }
            if is_stack_object_targeting_predicate(&predicate) {
                if let Some(effects_ast) = parsed.effects_ast.take() {
                    if let [
                        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                            predicate,
                            if_true,
                            if_false,
                        }),
                    ] = effects_ast.as_slice()
                        && if_false.is_empty()
                        && is_stack_object_targeting_predicate(predicate)
                    {
                        parsed.effects_ast = Some(if_true.clone());
                    } else {
                        parsed.effects_ast = Some(effects_ast);
                    }
                }
                return Ok(LineAst::Ability(parsed));
            }
            let mut reference_imports = parsed.reference_imports.clone();
            let default_last_object_tag = reference_imports.last_object_tag.clone().or_else(|| {
                parsed.trigger_spec.as_deref().and_then(
                    ironsmith_compiler_semantic::trigger_references::default_trigger_last_object_tag,
                )
            });
            if reference_imports.last_object_tag.is_none() {
                reference_imports.last_object_tag = default_last_object_tag.clone();
            }
            // Whether this predicate can bind at all decides whether the line
            // takes this shape, so it is checked here — but the answer stored is
            // the predicate, not the binding. Lowering binds it again against
            // the same references the trigger exports.
            let binds = crate::reference_resolution_support::resolve_condition_from_predicate(
                &predicate,
                &ReferenceEnv::from_imports(&reference_imports, false, false, false, None),
                &default_last_object_tag,
            )
            .is_ok();
            if binds {
                if let AbilityKind::Triggered(triggered) = parsed.kind_mut() {
                    triggered.intervening_if = Some(match triggered.intervening_if.take() {
                        Some(existing) => {
                            PredicateAst::And(Box::new(existing), Box::new(predicate.clone()))
                        }
                        None => predicate.clone(),
                    });
                }
                if let Some(effects_ast) = parsed.effects_ast.take() {
                    if let [
                        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                            if_true,
                            if_false,
                            ..
                        }),
                    ] = effects_ast.as_slice()
                        && if_false.is_empty()
                    {
                        parsed.effects_ast = Some(if_true.clone());
                    } else {
                        parsed.effects_ast = Some(effects_ast);
                    }
                }
            } else if let Some(effects_ast) = parsed.effects_ast.take() {
                parsed.effects_ast = Some(effects_ast);
            }
            Ok(LineAst::Ability(parsed))
        }
        other => Ok(other),
    }
}
