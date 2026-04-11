use crate::cards::builders::{
    CardTextError, EffectAst, EffectLoweringContext, IT_TAG, PredicateAst, TagKey,
};
use crate::effect::{Condition, Effect, EffectPredicate};
use crate::target::ChooseSpec;

use super::{
    EffectPreludeTag, LoweredEffects, PreparedEffectsForLowering, PreparedPredicateForLowering,
    PreparedTriggeredEffectsForLowering, ReferenceEnv, ReferenceExports, ReferenceImports,
    compile_annotated_effects_with_context, compile_condition_from_predicate_ast,
    rewrite_prepare_effects_for_lowering,
};

pub(crate) fn compile_statement_effects(
    effects: &[EffectAst],
) -> Result<Vec<Effect>, CardTextError> {
    Ok(
        compile_statement_effects_with_imports(effects, &ReferenceImports::default())?
            .effects
            .to_vec(),
    )
}

pub(crate) fn compile_statement_effects_with_imports(
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared = rewrite_prepare_effects_for_lowering(effects, imports.clone())?;
    materialize_prepared_statement_effects(&prepared)
}

pub(crate) fn materialize_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    if let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        },
    ] = prepared.effects.as_slice()
    {
        let default_effects =
            compile_statement_effects_with_imports(if_false, &prepared.imports)?.effects;
        let replacement_effects =
            compile_statement_effects_with_imports(if_true, &prepared.imports)?.effects;
        let condition = compile_condition_from_predicate_ast_with_env(
            predicate,
            &prepared.initial_env,
            prepared.imports.last_object_tag.as_ref(),
        )?;
        return Ok(LoweredEffects {
            effects: crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment {
                    default_effects: default_effects.flattened_default_effects().to_vec(),
                    self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                        condition,
                        replacement_effects.flattened_default_effects().to_vec(),
                    )],
                },
            ]),
            choices: Vec::new(),
            exports: prepared.exports.clone(),
        });
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, _) = compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = fold_local_zone_rewrite_self_replacements(compiled);
    let final_env = ctx.reference_env();
    Ok(LoweredEffects {
        effects: crate::resolution::ResolutionProgram::from_effects(prepend_effect_prelude(
            compiled,
            compile_effect_prelude_tags(&prepared.prelude),
        )),
        choices: Vec::new(),
        exports: ReferenceExports::from_env(&final_env),
    })
}

pub(crate) fn materialize_prepared_effects_with_trigger_context(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    if let Some((
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        },
        prefix_effects,
    )) = prepared.effects.split_last()
        && prefix_effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::SelfReplacement { .. }))
    {
        let prefix_lowered =
            compile_statement_effects_with_imports(prefix_effects, &prepared.imports)?;
        let default_lowered = compile_statement_effects_with_imports(if_false, &prepared.imports)?;
        let replacement_lowered =
            compile_statement_effects_with_imports(if_true, &prepared.imports)?;
        let condition = compile_condition_from_predicate_ast_with_env(
            predicate,
            &prepared.initial_env,
            prepared.imports.last_object_tag.as_ref(),
        )?;
        let mut default_effects = prefix_lowered.effects.flattened_default_effects().to_vec();
        default_effects.extend(default_lowered.effects.flattened_default_effects().to_vec());

        let mut choices = prefix_lowered.choices;
        choices.extend(default_lowered.choices);
        choices.extend(replacement_lowered.choices);
        return Ok(LoweredEffects {
            effects: crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment {
                    default_effects,
                    self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                        condition,
                        replacement_lowered
                            .effects
                            .flattened_default_effects()
                            .to_vec(),
                    )],
                },
            ]),
            choices,
            exports: prepared.exports.clone(),
        });
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, choices) =
        compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = fold_local_zone_rewrite_self_replacements(compiled);
    let final_env = ctx.reference_env();
    Ok(LoweredEffects {
        effects: crate::resolution::ResolutionProgram::from_effects(prepend_effect_prelude(
            compiled,
            compile_effect_prelude_tags(&prepared.prelude),
        )),
        choices,
        exports: ReferenceExports::from_env(&final_env),
    })
}

pub(crate) fn materialize_prepared_triggered_effects(
    prepared: &PreparedTriggeredEffectsForLowering,
) -> Result<(LoweredEffects, Option<Condition>), CardTextError> {
    let mut lowered = materialize_prepared_effects_with_trigger_context(&prepared.prepared)?;
    strip_erroneous_meld_player_exile_effect(&mut lowered);
    let intervening_if = prepared
        .intervening_if
        .as_ref()
        .map(compile_prepared_predicate_for_lowering)
        .transpose()?;
    Ok((lowered, intervening_if))
}

fn strip_erroneous_meld_player_exile_effect(lowered: &mut LoweredEffects) {
    let flattened = lowered.effects.flattened_default_effects();
    if flattened.len() < 2 {
        return;
    }

    let mut rewritten = Vec::with_capacity(flattened.len());
    let mut idx = 0usize;
    while idx < flattened.len() {
        let skip_erroneous_exile = idx + 1 < flattened.len()
            && flattened[idx]
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .is_some_and(|effect| {
                    effect.zone == crate::zone::Zone::Exile
                        && effect.target
                            == crate::target::ChooseSpec::Player(
                                crate::target::PlayerFilter::IteratedPlayer,
                            )
                })
            && flattened[idx + 1]
                .downcast_ref::<crate::effects::MeldEffect>()
                .is_some();
        if skip_erroneous_exile {
            idx += 1;
            continue;
        }

        rewritten.push(flattened[idx].clone());
        idx += 1;
    }

    if rewritten.len() != flattened.len() {
        lowered.effects = crate::resolution::ResolutionProgram::from_effects(rewritten);
    }
}

fn fold_local_zone_rewrite_self_replacements(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::new();
    let mut idx = 0usize;

    while idx < effects.len() {
        if idx + 1 < effects.len()
            && let Some(with_id) = effects[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = effects[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition == with_id.id
            && if_effect.predicate == EffectPredicate::Happened
            && if_effect.else_.is_empty()
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&if_effect.then, &with_id.effect)
        {
            rewritten.push(Effect::with_id(
                with_id.id.0,
                Effect::new(crate::effects::LocalRewriteEffect::new(
                    (*with_id.effect).clone(),
                    zone_replacements,
                )),
            ));
            idx += 2;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }

    rewritten
}

fn extract_local_zone_replacement_followups(
    effects: &[Effect],
    antecedent: &Effect,
) -> Option<Vec<crate::effects::RegisterZoneReplacementEffect>> {
    let mut replacements = Vec::new();
    let antecedent_target = antecedent.0.get_target_spec().cloned();
    for effect in effects {
        let mut register = effect
            .downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?
            .clone();
        if register.mode != crate::effects::ReplacementApplyMode::OneShot {
            return None;
        }
        if choose_spec_contains_it_tag(&register.target)
            && let Some(target_spec) = &antecedent_target
        {
            register.target = target_spec.clone();
        }
        replacements.push(register);
    }
    Some(replacements)
}

fn choose_spec_contains_it_tag(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(tag) => tag.as_str() == IT_TAG,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_contains_it_tag(inner)
        }
        _ => false,
    }
}

pub(crate) fn compile_effect_prelude_tags(prelude: &[EffectPreludeTag]) -> Vec<Effect> {
    prelude
        .iter()
        .map(|tag| match tag {
            EffectPreludeTag::AttachedSource(tag) => Effect::tag_attached_to_source(tag.as_str()),
            EffectPreludeTag::TriggeringObject(tag) => Effect::tag_triggering_object(tag.as_str()),
            EffectPreludeTag::TriggeringDamageTarget(tag) => {
                Effect::tag_triggering_damage_target(tag.as_str())
            }
        })
        .collect()
}

pub(crate) fn compile_condition_from_predicate_ast_with_env(
    predicate: &PredicateAst,
    refs: &ReferenceEnv,
    saved_last_object_tag: Option<&TagKey>,
) -> Result<Condition, CardTextError> {
    let mut ctx = EffectLoweringContext::new();
    let reference_env: crate::cards::builders::ReferenceEnv = refs.clone().into();
    ctx.apply_reference_env(&reference_env);
    let saved_last_tag = saved_last_object_tag.map(|tag| tag.as_str().to_string());
    compile_condition_from_predicate_ast(predicate, &mut ctx, &saved_last_tag)
}

pub(crate) fn compile_prepared_predicate_for_lowering(
    prepared: &PreparedPredicateForLowering,
) -> Result<Condition, CardTextError> {
    compile_condition_from_predicate_ast_with_env(
        &prepared.predicate,
        &prepared.reference_env,
        prepared.saved_last_object_tag.as_ref(),
    )
}

fn prepend_effect_prelude(mut compiled: Vec<Effect>, mut prelude: Vec<Effect>) -> Vec<Effect> {
    if prelude.is_empty() {
        return compiled;
    }
    prelude.append(&mut compiled);
    prelude
}
