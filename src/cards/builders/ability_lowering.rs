use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::cards::ParseAnnotations;
use crate::cards::builders::{
    AdditionalCostChoiceOptionAst, CardDefinitionBuilder, CardTextError, EffectAst, LineInfo,
    LoweredEffects, NormalizedAdditionalCostChoiceOptionAst, NormalizedParsedAbility,
    NormalizedPreparedAbility, ParsedAbility, PreparedEffectsForLowering, ReferenceExports,
    ReferenceImports, StaticAbilityAst, TriggerSpec, collect_tag_spans_from_effects_with_context,
    compile_trigger_spec, materialize_prepared_effects_with_trigger_context,
    materialize_prepared_statement_effects, materialize_prepared_triggered_effects,
    prepare_effects_for_lowering, prepare_effects_with_trigger_context_for_lowering,
    prepare_triggered_effects_for_lowering,
};
use crate::effect::{Effect, EffectMode};
use crate::static_abilities::StaticAbility;
use crate::zone::Zone;

pub(crate) fn lower_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    materialize_prepared_statement_effects(prepared)
}

pub(crate) fn lower_prepared_effects_with_trigger_context(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    materialize_prepared_effects_with_trigger_context(prepared)
}

pub(crate) fn lower_statement_effects_with_imports(
    effects_ast: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared = prepare_effects_for_lowering(effects_ast, imports.clone())?;
    lower_prepared_statement_effects(&prepared)
}

pub(crate) fn lower_statement_effects(
    effects_ast: &[EffectAst],
) -> Result<Vec<Effect>, CardTextError> {
    Ok(lower_statement_effects_with_imports(effects_ast, &ReferenceImports::default())?.effects)
}

pub(crate) fn lower_effects_with_trigger_context_and_imports(
    trigger: Option<&TriggerSpec>,
    effects_ast: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared =
        prepare_effects_with_trigger_context_for_lowering(trigger, effects_ast, imports.clone())?;
    lower_prepared_effects_with_trigger_context(&prepared)
}

pub(crate) fn lower_activated_ability_effects_with_imports(
    effects_ast: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    lower_effects_with_trigger_context_and_imports(None, effects_ast, imports)
}

pub(crate) fn lower_additional_cost_choice_modes_with_exports(
    options: &[AdditionalCostChoiceOptionAst],
) -> Result<(Vec<EffectMode>, ReferenceExports), CardTextError> {
    let mut prepared_options = Vec::with_capacity(options.len());
    for option in options {
        prepared_options.push(NormalizedAdditionalCostChoiceOptionAst {
            description: option.description.clone(),
            effects_ast: option.effects.clone(),
            prepared: prepare_effects_for_lowering(&option.effects, ReferenceImports::default())?,
        });
    }
    lower_prepared_additional_cost_choice_modes_with_exports(&prepared_options)
}

pub(crate) fn lower_prepared_additional_cost_choice_modes_with_exports(
    options: &[NormalizedAdditionalCostChoiceOptionAst],
) -> Result<(Vec<EffectMode>, ReferenceExports), CardTextError> {
    let mut exports = ReferenceExports::default();
    let mut first = true;
    let mut modes = Vec::with_capacity(options.len());
    for option in options {
        let lowered = lower_prepared_statement_effects(&option.prepared)?;
        if first {
            exports = lowered.exports.clone();
            first = false;
        } else {
            exports = ReferenceExports::join(&exports, &lowered.exports);
        }
        modes.push(EffectMode {
            description: option.description.trim().to_string(),
            effects: lowered.effects,
        });
    }
    Ok((modes, exports))
}

fn prepare_parsed_ability_payload(
    parsed: &ParsedAbility,
) -> Result<Option<NormalizedPreparedAbility>, CardTextError> {
    let Some(effects_ast) = parsed.effects_ast.as_ref() else {
        return Ok(None);
    };

    if let AbilityKind::Activated(activated) = &parsed.ability.kind
        && (!activated.effects.is_empty() || !activated.choices.is_empty())
    {
        return Ok(None);
    }
    if let AbilityKind::Triggered(triggered) = &parsed.ability.kind
        && (!triggered.effects.is_empty() || !triggered.choices.is_empty())
    {
        return Ok(None);
    }

    Ok(match (&parsed.ability.kind, parsed.trigger_spec.as_ref()) {
        (AbilityKind::Triggered(_), Some(trigger)) => Some(NormalizedPreparedAbility::Triggered {
            trigger: trigger.clone(),
            prepared: prepare_triggered_effects_for_lowering(
                trigger,
                effects_ast,
                parsed.reference_imports.clone(),
            )?,
        }),
        (AbilityKind::Activated(_), _) => Some(NormalizedPreparedAbility::Activated(
            prepare_effects_with_trigger_context_for_lowering(
                None,
                effects_ast,
                parsed.reference_imports.clone(),
            )?,
        )),
        _ => None,
    })
}

fn merge_intervening_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: Option<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    match (existing, additional) {
        (Some(primary), Some(secondary)) => Some(crate::ConditionExpr::And(
            Box::new(primary),
            Box::new(secondary),
        )),
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    }
}

fn lower_parsed_ability_internal(
    mut parsed: ParsedAbility,
    prepared: Option<NormalizedPreparedAbility>,
) -> Result<ParsedAbility, CardTextError> {
    let Some(_) = parsed.effects_ast.as_ref() else {
        return Ok(parsed);
    };

    let prepared = match prepared {
        Some(prepared) => Some(prepared),
        None => prepare_parsed_ability_payload(&parsed)?,
    };

    let AbilityKind::Activated(activated) = &mut parsed.ability.kind else {
        if let AbilityKind::Triggered(triggered) = &mut parsed.ability.kind {
            if !triggered.effects.is_empty() || !triggered.choices.is_empty() {
                return Ok(parsed);
            }
            let Some(NormalizedPreparedAbility::Triggered { trigger, prepared }) = prepared else {
                return Ok(parsed);
            };
            let (lowered, parsed_intervening_if) =
                materialize_prepared_triggered_effects(&prepared)?;
            triggered.trigger = compile_trigger_spec(trigger);
            triggered.effects = lowered.effects;
            triggered.choices = lowered.choices;
            triggered.intervening_if = merge_intervening_conditions(
                triggered.intervening_if.take(),
                parsed_intervening_if,
            );
            return Ok(parsed);
        }
        return Ok(parsed);
    };
    if !activated.effects.is_empty() || !activated.choices.is_empty() {
        return Ok(parsed);
    }

    let Some(NormalizedPreparedAbility::Activated(prepared)) = prepared else {
        return Ok(parsed);
    };
    let lowered = materialize_prepared_effects_with_trigger_context(&prepared)?;
    activated.effects = lowered.effects;
    activated.choices = lowered.choices;
    Ok(parsed)
}

pub(crate) fn lower_prepared_ability(
    normalized: NormalizedParsedAbility,
) -> Result<ParsedAbility, CardTextError> {
    lower_parsed_ability_internal(normalized.parsed, normalized.prepared)
}

pub(crate) fn materialize_static_abilities_ast(
    abilities: Vec<StaticAbilityAst>,
) -> Result<Vec<StaticAbility>, CardTextError> {
    lower_static_abilities_ast(abilities)
}

pub(crate) fn apply_instead_followup_statement_to_last_ability(
    builder: &mut CardDefinitionBuilder,
    last_restrictable_ability: Option<usize>,
    effects: &[EffectAst],
    info: &LineInfo,
    annotations: &mut ParseAnnotations,
) -> Result<bool, CardTextError> {
    let Some(index) = last_restrictable_ability else {
        return Ok(false);
    };
    if index >= builder.abilities.len() {
        return Ok(false);
    }

    let normalized = info.normalized.normalized.as_str().to_ascii_lowercase();
    if !normalized.starts_with("if ") || !normalized.contains(" instead") {
        return Ok(false);
    }

    let compiled = lower_statement_effects(effects)?;
    if compiled.len() != 1 {
        return Ok(false);
    }

    let Some(replacement) = compiled[0].downcast_ref::<crate::effects::ConditionalEffect>() else {
        return Ok(false);
    };
    if !replacement.if_false.is_empty() {
        return Ok(false);
    }

    collect_tag_spans_from_effects_with_context(effects, annotations, &info.normalized);

    let conditional = replacement.clone();
    match &mut builder.abilities[index].kind {
        AbilityKind::Triggered(ability) => {
            let original = std::mem::take(&mut ability.effects);
            if original.is_empty() {
                return Ok(false);
            }
            ability.effects = vec![Effect::new(crate::effects::ConditionalEffect::new(
                conditional.condition,
                conditional.if_true,
                original,
            ))];
        }
        AbilityKind::Activated(ability) => {
            let original = std::mem::take(&mut ability.effects);
            if original.is_empty() {
                return Ok(false);
            }
            ability.effects = vec![Effect::new(crate::effects::ConditionalEffect::new(
                conditional.condition,
                conditional.if_true,
                original,
            ))];
        }
        _ => return Ok(false),
    }

    Ok(true)
}

pub(crate) fn parsed_triggered_ability(
    trigger: TriggerSpec,
    effects_ast: Vec<EffectAst>,
    functional_zones: Vec<Zone>,
    text: Option<String>,
    intervening_if: Option<crate::ConditionExpr>,
    reference_imports: ReferenceImports,
) -> ParsedAbility {
    ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: compile_trigger_spec(trigger.clone()),
                effects: vec![],
                choices: vec![],
                intervening_if,
            }),
            functional_zones,
            text,
        },
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: Some(trigger),
    }
}

pub(crate) fn lower_parsed_ability(parsed: ParsedAbility) -> Result<ParsedAbility, CardTextError> {
    lower_parsed_ability_internal(parsed, None)
}

pub(crate) fn lower_static_ability_ast(
    ability: StaticAbilityAst,
) -> Result<StaticAbility, CardTextError> {
    match ability {
        StaticAbilityAst::Static(ability) => Ok(ability),
        StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition,
        } => {
            let mut lowered = lower_parsed_ability(ability)?.ability;
            if lowered.text.is_none() {
                lowered.text = Some(display.clone());
            }
            let mut grant =
                crate::static_abilities::GrantObjectAbilityForFilter::new(filter, lowered, display);
            if let Some(condition) = condition {
                grant = grant.with_condition(condition);
            }
            Ok(StaticAbility::new(grant))
        }
        StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition,
        } => {
            let mut lowered = lower_parsed_ability(ability)?.ability;
            if lowered.text.is_none() {
                lowered.text = Some(display.clone());
            }
            let mut grant = crate::static_abilities::AttachedAbilityGrant::new(lowered, display);
            if let Some(condition) = condition {
                grant = grant.with_condition(condition);
            }
            Ok(StaticAbility::new(grant))
        }
        StaticAbilityAst::SoulbondSharedObjectAbility { ability, display } => {
            let mut lowered = lower_parsed_ability(ability)?.ability;
            if lowered.text.is_none() {
                lowered.text = Some(display);
            }
            Ok(StaticAbility::soulbond_shared_object_ability(lowered))
        }
    }
}

pub(crate) fn lower_static_abilities_ast(
    abilities: Vec<StaticAbilityAst>,
) -> Result<Vec<StaticAbility>, CardTextError> {
    abilities
        .into_iter()
        .map(lower_static_ability_ast)
        .collect()
}
