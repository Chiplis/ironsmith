use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::cards::builders::{
    CardTextError, EffectAst, NormalizedPreparedAbility, ParsedAbility, ReferenceImports,
    TriggerSpec, compile_trigger_spec,
    materialize_prepared_effects_with_trigger_context, materialize_prepared_triggered_effects,
    prepare_effects_with_trigger_context_for_lowering, prepare_triggered_effects_for_lowering,
    trigger_binds_player_reference_context,
    validate_iterated_player_bindings_in_lowered_effects,
};
use crate::zone::Zone;

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
            validate_iterated_player_bindings_in_lowered_effects(
                &lowered,
                trigger_binds_player_reference_context(&trigger),
                "triggered ability effects",
            )?;
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
    validate_iterated_player_bindings_in_lowered_effects(
        &lowered,
        false,
        "activated ability effects",
    )?;
    activated.effects = lowered.effects;
    activated.choices = lowered.choices;
    Ok(parsed)
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
                effects: crate::resolution::ResolutionProgram::default(),
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
