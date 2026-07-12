use crate::cards::builders::{
    CardTextError, EffectAst, EffectLoweringContext, IT_TAG, PredicateAst, TagKey,
};
use crate::effect::{Condition, Effect, EffectPredicate};
use crate::target::{ChooseSpec, ObjectFilter};

use super::{
    EffectPreludeTag, LoweredEffects, PreparedEffectsForLowering, PreparedPredicateForLowering,
    PreparedTriggeredEffectsForLowering, ReferenceEnv, ReferenceExports, ReferenceImports,
    compile_annotated_effects_with_context, compile_condition_from_predicate_ast, push_choice,
    rewrite_prepare_effects_for_lowering,
};

#[cfg(test)]
pub(crate) fn compile_statement_effects(
    effects: &[EffectAst],
) -> Result<Vec<Effect>, CardTextError> {
    stacker::maybe_grow(8 * 1024 * 1024, 16 * 1024 * 1024, || {
        Ok(
            compile_statement_effects_with_imports(effects, &ReferenceImports::default())?
                .effects
                .to_vec(),
        )
    })
}

pub(crate) fn compile_statement_effects_with_imports(
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    stacker::maybe_grow(8 * 1024 * 1024, 16 * 1024 * 1024, || {
        let prepared = rewrite_prepare_effects_for_lowering(effects, imports.clone())?;
        materialize_prepared_statement_effects(&prepared)
    })
}

pub(crate) fn materialize_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    if let Some(lowered) = materialize_trailing_self_replacement(prepared)? {
        return Ok(lowered);
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, _) = compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = normalize_two_target_counter_then_fight(compiled);
    let compiled = normalize_random_destroy_across_target_groups(compiled);
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
    if let Some(lowered) = materialize_trailing_self_replacement(prepared)? {
        return Ok(lowered);
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    let (compiled, choices) =
        compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = normalize_two_target_counter_then_fight(compiled);
    let compiled = normalize_random_destroy_across_target_groups(compiled);
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

fn materialize_trailing_self_replacement(
    prepared: &PreparedEffectsForLowering,
) -> Result<Option<LoweredEffects>, CardTextError> {
    if let Some((
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        },
        prefix_effects,
    )) = prepared.effects.split_last()
        && prefix_effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::SelfReplacement { .. }))
    {
        let default_lowered = compile_statement_effects_with_imports(if_false, &prepared.imports)?;
        let replacement_lowered =
            compile_statement_effects_with_imports(if_true, &prepared.imports)?;
        let prefix_lowered =
            compile_statement_effects_with_imports(prefix_effects, &prepared.imports)?;
        let mut default_effects = prefix_lowered.effects.flattened_default_effects().to_vec();
        default_effects.extend(default_lowered.effects.flattened_default_effects().to_vec());
        let mut condition_env = if prefix_effects.is_empty() {
            prepared.initial_env.clone()
        } else {
            prepared.annotated.effects[prefix_effects.len() - 1]
                .out_env
                .clone()
        };
        let implicit_condition_tag = if condition_env.known_last_object_tag().is_none()
            && predicate_uses_implicit_object_reference(predicate)
        {
            last_tagged_default_target(&default_effects).map(|(tag, _)| tag)
        } else {
            None
        };
        if condition_env.known_last_object_tag().is_none()
            && implicit_condition_tag.is_none()
            && predicate_uses_implicit_object_reference(predicate)
        {
            condition_env.source_object_antecedent = true;
        }
        let condition_imports = ReferenceExports::from_env(&condition_env).to_imports();
        let condition_tag = implicit_condition_tag
            .as_ref()
            .or(condition_imports.last_object_tag.as_ref());
        let condition = compile_condition_from_predicate_ast_with_env(
            predicate,
            &condition_env,
            condition_tag,
        )?;
        let replacement_effects = replacement_lowered
            .effects
            .flattened_default_effects()
            .to_vec();
        let replacement_effects =
            strip_duplicate_self_replacement_prelude(&default_effects, replacement_effects);
        let replacement_effects = if let Some(antecedent) = default_effects
            .iter()
            .rev()
            .find(|effect| effect.target_spec().is_some())
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&replacement_effects, antecedent)
        {
            vec![Effect::new(crate::effects::LocalRewriteEffect::new(
                antecedent.clone(),
                zone_replacements,
            ))]
        } else {
            replacement_effects
        };

        let mut choices = prefix_lowered.choices;
        for choice in default_lowered
            .choices
            .into_iter()
            .chain(replacement_lowered.choices)
        {
            push_choice(&mut choices, choice);
        }
        return Ok(Some(LoweredEffects {
            effects: crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment {
                    default_effects,
                    self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                        condition,
                        replacement_effects,
                    )],
                },
            ]),
            choices,
            exports: prepared.exports.clone(),
        }));
    }
    Ok(None)
}

fn last_tagged_default_target(default_effects: &[Effect]) -> Option<(TagKey, ChooseSpec)> {
    default_effects
        .iter()
        .rev()
        .find_map(last_tagged_target_in_effect)
}

fn last_tagged_target_in_effect(effect: &Effect) -> Option<(TagKey, ChooseSpec)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if let Some(target) = tagged.effect.target_spec() {
            return Some((tagged.tag.clone(), target.clone()));
        }
        return last_tagged_target_in_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return last_tagged_target_in_effect(&with_id.effect);
    }
    if let Some(local_rewrite) = effect.downcast_ref::<crate::effects::LocalRewriteEffect>() {
        return last_tagged_target_in_effect(&local_rewrite.effect);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence
            .effects
            .iter()
            .rev()
            .find_map(last_tagged_target_in_effect);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<Effect>>() {
        return may
            .effects
            .iter()
            .rev()
            .find_map(last_tagged_target_in_effect);
    }
    None
}

fn predicate_uses_implicit_object_reference(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::ItIsLandCard
        | PredicateAst::ItIsSoulbondPaired
        | PredicateAst::ItMatches(_)
        | PredicateAst::TargetMatches(_) => true,
        PredicateAst::Not(inner) => predicate_uses_implicit_object_reference(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_uses_implicit_object_reference(left)
                || predicate_uses_implicit_object_reference(right)
        }
        _ => false,
    }
}

pub(crate) fn materialize_prepared_triggered_effects(
    prepared: &PreparedTriggeredEffectsForLowering,
) -> Result<(LoweredEffects, Option<Condition>), CardTextError> {
    let mut lowered = materialize_prepared_effects_with_trigger_context(&prepared.prepared)?;
    strip_erroneous_meld_player_exile_effect(&mut lowered);
    dedupe_adjacent_target_only_effects(&mut lowered);
    let intervening_if = prepared
        .intervening_if
        .as_ref()
        .map(compile_prepared_predicate_for_lowering)
        .transpose()?;
    if let Some(condition) = intervening_if.as_ref() {
        retarget_source_move_to_damaged_death_card(&mut lowered, condition);
    }
    Ok((lowered, intervening_if))
}

fn damaged_death_condition_target_filter(condition: &Condition) -> Option<ObjectFilter> {
    match condition {
        Condition::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        } if *count == 1 => {
            let mut filter = victim.clone();
            filter.zone = Some(crate::zone::Zone::Graveyard);
            filter.entered_graveyard_from_battlefield_this_turn = true;
            filter.dealt_damage_by_source_this_turn = Some(*damager);
            Some(filter)
        }
        Condition::And(left, right) => damaged_death_condition_target_filter(left)
            .or_else(|| damaged_death_condition_target_filter(right)),
        _ => None,
    }
}

fn retarget_source_move_to_damaged_death_card(lowered: &mut LoweredEffects, condition: &Condition) {
    let Some(filter) = damaged_death_condition_target_filter(condition) else {
        return;
    };
    let Some(segment) = lowered.effects.segments.first_mut() else {
        return;
    };
    let Some(effect) = segment.default_effects.first_mut() else {
        return;
    };
    let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(move_to_zone) = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return;
    };
    if !matches!(move_to_zone.target.base(), ChooseSpec::Source)
        || move_to_zone.zone != crate::zone::Zone::Battlefield
    {
        return;
    }

    let mut replacement = move_to_zone.clone();
    replacement.target =
        ChooseSpec::Object(filter).with_count(crate::effect::ChoiceCount::exactly(1));
    *effect = Effect::new(crate::effects::TaggedEffect::new(
        tagged.tag.clone(),
        Effect::new(replacement),
    ));
}

fn dedupe_adjacent_target_only_effects(lowered: &mut LoweredEffects) {
    let flattened = lowered.effects.flattened_default_effects();
    if flattened.len() < 2 {
        return;
    }

    let mut rewritten = Vec::with_capacity(flattened.len());
    for effect in flattened {
        let duplicate_target_only = rewritten.last().is_some_and(|previous: &Effect| {
            let Some(previous_target) = previous.downcast_ref::<crate::effects::TargetOnlyEffect>()
            else {
                return false;
            };
            let Some(current_target) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
            else {
                return false;
            };
            previous_target == current_target
        });
        if !duplicate_target_only {
            rewritten.push(effect.clone());
        }
    }

    if rewritten.len() != flattened.len() {
        lowered.effects = crate::resolution::ResolutionProgram::from_effects(rewritten);
    }
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
            && let Some(delayed) = with_id
                .effect
                .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            && let Some(if_effect) = effects[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition == with_id.id
            && let [inner_effect] = delayed.effects.as_slice()
        {
            let mut delayed = delayed.clone();
            delayed.effects = vec![
                Effect::with_id(with_id.id.0, inner_effect.clone()),
                effects[idx + 1].clone(),
            ];
            rewritten.push(Effect::new(delayed));
            idx += 2;
            continue;
        }
        if idx + 1 < effects.len()
            && let Some(with_id) = effects[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = effects[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && {
                #[cfg(not(feature = "serialization"))]
                {
                    if_effect.condition == with_id.id
                }
                #[cfg(feature = "serialization")]
                {
                    if_effect.condition == with_id.id
                }
            }
            && if_effect.predicate == EffectPredicate::Happened
            && if_effect.else_.is_empty()
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&if_effect.then, &with_id.effect)
        {
            rewritten.push(Effect::with_id(
                with_id.id.0,
                Effect::new(crate::effects::LocalRewriteEffect::new(
                    {
                        #[cfg(not(feature = "serialization"))]
                        {
                            (*with_id.effect).clone()
                        }
                        #[cfg(feature = "serialization")]
                        {
                            (*with_id.effect).clone()
                        }
                    },
                    zone_replacements,
                )),
            ));
            idx += 2;
            continue;
        }
        if idx + 1 < effects.len()
            && effects[idx].target_spec().is_some()
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&effects[idx + 1..idx + 2], &effects[idx])
            && zone_replacements.iter().all(|replacement| {
                replacement.from_zone == Some(crate::zone::Zone::Stack)
                    && replacement.to_zone == Some(crate::zone::Zone::Graveyard)
            })
        {
            rewritten.push(Effect::new(crate::effects::LocalRewriteEffect::new(
                effects[idx].clone(),
                zone_replacements,
            )));
            idx += 2;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }

    rewritten
}

fn strip_duplicate_self_replacement_prelude(
    default_effects: &[Effect],
    mut replacement_effects: Vec<Effect>,
) -> Vec<Effect> {
    let shared_prelude_len = default_effects
        .iter()
        .zip(replacement_effects.iter())
        .take_while(|(default, replacement)| same_resolution_prelude(default, replacement))
        .count();
    if shared_prelude_len > 0 {
        replacement_effects.drain(0..shared_prelude_len);
    }
    replacement_effects
}

fn same_resolution_prelude(left: &Effect, right: &Effect) -> bool {
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagAttachedToSourceEffect>(),
        right.downcast_ref::<crate::effects::TagAttachedToSourceEffect>(),
    ) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagTriggeringObjectEffect>(),
        right.downcast_ref::<crate::effects::TagTriggeringObjectEffect>(),
    ) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagTriggeringSourceEffect>(),
        right.downcast_ref::<crate::effects::TagTriggeringSourceEffect>(),
    ) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>(),
        right.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>(),
    ) {
        return left == right;
    }
    false
}

fn extract_local_zone_replacement_followups(
    effects: &[Effect],
    antecedent: &Effect,
) -> Option<Vec<crate::effects::RegisterZoneReplacementEffect>> {
    let mut replacements = Vec::new();
    let antecedent_target = antecedent.target_spec().cloned();
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

fn normalize_two_target_counter_then_fight(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::with_capacity(effects.len());
    let mut idx = 0;
    while idx < effects.len() {
        if idx + 3 < effects.len()
            && let Some((first_tag, first_target)) = tagged_target_only(&effects[idx])
            && let Some((second_tag, _second_target)) = tagged_target_only(&effects[idx + 1])
            && let Some((counter_tag, condition, counters)) =
                single_conditional_tagged_put_counters(&effects[idx + 2])
            && counter_target_matches_choice(&counters.target, first_target)
            && fight_references_counter_tag(&effects[idx + 3], counter_tag.as_str())
        {
            let mut fixed_counters = counters.clone();
            fixed_counters.target = ChooseSpec::Tagged(first_tag.clone());
            let fixed_counter_effect = Effect::new(fixed_counters).tag(counter_tag.as_str());
            rewritten.push(effects[idx].clone());
            rewritten.push(effects[idx + 1].clone());
            rewritten.push(Effect::conditional(
                condition.clone(),
                vec![fixed_counter_effect],
                Vec::new(),
            ));
            rewritten.push(Effect::fight(
                ChooseSpec::Tagged(first_tag.clone()),
                ChooseSpec::Tagged(second_tag.clone()),
            ));
            idx += 4;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }
    rewritten
}

fn tagged_target_only(effect: &Effect) -> Option<(&TagKey, &ChooseSpec)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    Some((&tagged.tag, &target_only.target))
}

fn random_single_tagged_destroy_tag(destroy: &crate::effects::DestroyEffect) -> Option<&TagKey> {
    let ChooseSpec::WithCount(inner, count) = &destroy.spec else {
        return None;
    };
    if !count.is_single() || !count.is_random() {
        return None;
    }
    match inner.as_ref() {
        ChooseSpec::Tagged(tag) => Some(tag),
        _ => None,
    }
}

fn any_of_tagged_objects(tags: &[&TagKey]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.any_of = tags
        .iter()
        .map(|tag| ObjectFilter::tagged((*tag).clone()))
        .collect();
    filter
}

fn normalize_random_destroy_across_target_groups(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::with_capacity(effects.len());
    let mut idx = 0usize;
    while idx < effects.len() {
        if idx + 2 < effects.len()
            && let Some((first_tag, _)) = tagged_target_only(&effects[idx])
            && let Some((second_tag, _)) = tagged_target_only(&effects[idx + 1])
            && first_tag != second_tag
            && let Some(destroy) = effects[idx + 2].downcast_ref::<crate::effects::DestroyEffect>()
            && random_single_tagged_destroy_tag(destroy) == Some(second_tag)
            && let ChooseSpec::WithCount(_, count) = &destroy.spec
        {
            let target = ChooseSpec::WithCount(
                Box::new(ChooseSpec::Object(any_of_tagged_objects(&[
                    first_tag, second_tag,
                ]))),
                *count,
            );
            rewritten.push(effects[idx].clone());
            rewritten.push(effects[idx + 1].clone());
            rewritten.push(Effect::new(crate::effects::DestroyEffect::with_spec(
                target,
            )));
            idx += 3;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }
    rewritten
}

fn single_conditional_tagged_put_counters(
    effect: &Effect,
) -> Option<(&TagKey, &Condition, &crate::effects::PutCountersEffect)> {
    let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let tagged = conditional.if_true[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let counters = tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    Some((&tagged.tag, &conditional.condition, counters))
}

fn counter_target_matches_choice(counter_target: &ChooseSpec, choice: &ChooseSpec) -> bool {
    match (counter_target, choice) {
        (ChooseSpec::Object(left), ChooseSpec::Target(inner)) => {
            matches!(inner.as_ref(), ChooseSpec::Object(right) if left == right)
        }
        (left, right) => left == right,
    }
}

fn fight_references_counter_tag(effect: &Effect, tag: &str) -> bool {
    let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() else {
        return false;
    };
    choose_spec_references_tag(&fight.creature1, tag)
        && choose_spec_references_tag(&fight.creature2, tag)
}

fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate.as_str() == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_tag(inner, tag)
        }
        _ => false,
    }
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
            EffectPreludeTag::TriggeringBlockers(tag, filter) => {
                Effect::tag_triggering_blockers(tag.as_str(), Some(filter.clone()))
            }
            EffectPreludeTag::TriggeringSource(tag) => Effect::tag_triggering_source(tag.as_str()),
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
