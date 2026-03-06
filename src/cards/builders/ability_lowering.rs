use super::*;

pub(crate) fn lower_statement_effects_with_seed(
    effects_ast: &[EffectAst],
    seed_last_object_tag: Option<&str>,
) -> Result<Vec<Effect>, CardTextError> {
    compile_statement_effects_with_seed(effects_ast, seed_last_object_tag)
}

pub(crate) fn lower_statement_effects(
    effects_ast: &[EffectAst],
) -> Result<Vec<Effect>, CardTextError> {
    compile_statement_effects(effects_ast)
}

pub(crate) fn lower_effects_with_trigger_context_and_seed(
    trigger: Option<&TriggerSpec>,
    effects_ast: &[EffectAst],
    seed_last_object_tag: Option<&str>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    compile_trigger_effects_with_seed(trigger, effects_ast, seed_last_object_tag)
}

pub(crate) fn lower_activated_ability_effects_with_seed(
    effects_ast: &[EffectAst],
    seed_last_object_tag: Option<&str>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    lower_effects_with_trigger_context_and_seed(None, effects_ast, seed_last_object_tag)
}

pub(crate) fn lower_additional_cost_choice_modes(
    options: &[AdditionalCostChoiceOptionAst],
) -> Result<Vec<EffectMode>, CardTextError> {
    options
        .iter()
        .map(|option| {
            let effects = lower_statement_effects(&option.effects)?;
            Ok(EffectMode {
                description: option.description.trim().to_string(),
                effects,
            })
        })
        .collect()
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
    seed_last_object_tag: Option<String>,
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
        seed_last_object_tag,
        trigger_spec: Some(trigger),
    }
}

pub(crate) fn lower_parsed_ability(
    mut parsed: ParsedAbility,
) -> Result<ParsedAbility, CardTextError> {
    let Some(effects_ast) = parsed.effects_ast.as_ref() else {
        return Ok(parsed);
    };

    let AbilityKind::Activated(activated) = &mut parsed.ability.kind else {
        if let AbilityKind::Triggered(triggered) = &mut parsed.ability.kind {
            if !triggered.effects.is_empty() || !triggered.choices.is_empty() {
                return Ok(parsed);
            }
            let Some(trigger_spec) = parsed.trigger_spec.as_ref() else {
                return Ok(parsed);
            };
            let (effects, choices, parsed_intervening_if) =
                if let Some(seed_last_object_tag) = parsed.seed_last_object_tag.as_deref() {
                    compile_trigger_effects_with_intervening_if_seed(
                        Some(trigger_spec),
                        effects_ast,
                        Some(seed_last_object_tag),
                    )?
                } else {
                    compile_trigger_effects_with_intervening_if(Some(trigger_spec), effects_ast)?
                };
            triggered.trigger = compile_trigger_spec(trigger_spec.clone());
            triggered.effects = effects;
            triggered.choices = choices;
            triggered.intervening_if =
                match (parsed_intervening_if, triggered.intervening_if.take()) {
                    (Some(primary), Some(secondary)) => Some(crate::ConditionExpr::And(
                        Box::new(primary),
                        Box::new(secondary),
                    )),
                    (Some(condition), None) | (None, Some(condition)) => Some(condition),
                    (None, None) => None,
                };
            return Ok(parsed);
        }
        return Ok(parsed);
    };
    if !activated.effects.is_empty() || !activated.choices.is_empty() {
        return Ok(parsed);
    }

    let (effects, choices) = lower_activated_ability_effects_with_seed(
        effects_ast,
        parsed.seed_last_object_tag.as_deref(),
    )?;
    activated.effects = effects;
    activated.choices = choices;
    Ok(parsed)
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
