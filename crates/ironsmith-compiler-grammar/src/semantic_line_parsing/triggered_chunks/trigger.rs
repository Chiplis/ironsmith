use crate::cards::builders::ConditionalEffectAst;
use super::*;

pub fn apply_explicit_intervening_if_to_triggered_chunk(
    chunk: LineAst,
    explicit_intervening_if: Option<PredicateAst>,
) -> Result<LineAst, CardTextError> {
    let Some(predicate) = explicit_intervening_if else {
        return Ok(chunk);
    };
    match chunk {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
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
                resolve_it_animations_to_source(&mut effects);
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
            if let Some(mut effects_ast) = parsed.effects_ast.take() {
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
                    resolve_it_animations_to_source(&mut effects_ast);
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
                            if_true, if_false, ..
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
