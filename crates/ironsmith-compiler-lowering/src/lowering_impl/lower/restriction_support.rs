use crate::ability::{Ability, AbilityKind, ActivatedAbility, TriggeredAbility};
use crate::cards::builders::CardTextError;
use crate::model::compiler_semantic::{
    ActivationRestrictionNormalizationFact, ParsedActivationRestriction, ParsedRestrictions,
    ParsedTriggerRestriction,
};

pub fn apply_pending_restrictions_to_ability(
    ability: &mut Ability,
    pending: &mut ParsedRestrictions,
) -> Result<(), CardTextError> {
    let activation_restrictions = std::mem::take(&mut pending.activation);
    let trigger_restrictions = std::mem::take(&mut pending.trigger);

    match &mut ability.kind {
        AbilityKind::Activated(ability) => {
            for restriction in &activation_restrictions {
                apply_pending_activation_restriction(ability, restriction)?;
            }
        }
        AbilityKind::Triggered(ability) => {
            for restriction in &trigger_restrictions {
                apply_pending_trigger_restriction(ability, restriction);
            }
        }
        AbilityKind::Static(_) => {}
    }

    if !activation_restrictions.is_empty() {
        pending.activation.extend(activation_restrictions);
    }
    if !trigger_restrictions.is_empty() {
        pending.trigger.extend(trigger_restrictions);
    }
    Ok(())
}

pub fn is_restrictable_ability(ability: &Ability) -> bool {
    matches!(
        ability.kind,
        AbilityKind::Activated(_) | AbilityKind::Triggered(_)
    )
}

fn apply_pending_activation_restriction(
    ability: &mut ActivatedAbility,
    restriction: &ParsedActivationRestriction,
) -> Result<(), CardTextError> {
    fn push_restriction_condition(ability: &mut ActivatedAbility, condition: crate::ConditionExpr) {
        if !ability.activation_restrictions.contains(&condition) {
            ability.activation_restrictions.push(condition);
        }
    }

    // The restriction line recorded a predicate; the runtime ability holds the
    // bound form, so it is bound here.
    let restriction_condition = restriction
        .condition
        .as_ref()
        .map(crate::lowering_support::resolve_intervening_if_without_trigger)
        .transpose()?;
    if restriction_condition.is_some() {
        ability.activation_condition = merge_conditions(
            ability.activation_condition.take(),
            restriction_condition.clone(),
        );
    }

    let mut timing_applied = false;
    if let Some(parsed_timing) = restriction.timing {
        let merged_timing = merge_activation_timing(ability.timing, parsed_timing);
        timing_applied = merged_timing == parsed_timing;
        ability.timing = merged_timing;
        if !timing_applied {
            push_restriction_condition(
                ability,
                crate::ConditionExpr::ActivationTiming(parsed_timing),
            );
        }
    }

    if let Some(crate::ConditionExpr::MaxActivationsPerTurn(limit)) = restriction_condition {
        push_restriction_condition(ability, crate::ConditionExpr::MaxActivationsPerTurn(limit));
    }
    if let Some(condition) = restriction.text_only_condition.as_ref() {
        push_restriction_condition(
            ability,
            crate::lowering_support::resolve_intervening_if_without_trigger(condition)?,
        );
    }
    if let Some(usage) = restriction.mana_usage_restriction.clone() {
        let usage = usage.try_map_effects(&mut |effect| {
            super::super::lowering_support::lower_compiler_child_effect(effect)
        })?;
        ability.mana_usage_restrictions.push(usage);
    }

    let additional = if restriction.timing.is_some() && !timing_applied {
        Some(restriction.presentation_text.clone())
    } else {
        normalized_activation_restriction(restriction)
    };
    if let Some(additional) = additional {
        ability.additional_restrictions.push(additional);
    }
    if restriction.once_per_turn_after_other_restrictions
        && !ability
            .additional_restrictions
            .iter()
            .any(|value| value == "__ironsmith_once_per_turn_after_other_restrictions")
    {
        ability
            .additional_restrictions
            .push("__ironsmith_once_per_turn_after_other_restrictions".to_string());
    }
    Ok(())
}

fn apply_pending_trigger_restriction(
    ability: &mut TriggeredAbility,
    restriction: &ParsedTriggerRestriction,
) {
    if let Some(parsed_count) = restriction.max_times_each_turn {
        ability.intervening_if = Some(match ability.intervening_if.take() {
            Some(crate::ConditionExpr::MaxTimesEachTurn(existing)) => {
                crate::ConditionExpr::MaxTimesEachTurn(existing.min(parsed_count))
            }
            _ => crate::ConditionExpr::MaxTimesEachTurn(parsed_count),
        });
    }
}

fn merge_activation_timing(
    existing: crate::ability::ActivationTiming,
    next: crate::ability::ActivationTiming,
) -> crate::ability::ActivationTiming {
    match (existing, next) {
        (current, crate::ability::ActivationTiming::AnyTime) => current,
        (crate::ability::ActivationTiming::AnyTime, next) => next,
        (current, next) if current == next => current,
        (current, _) => current,
    }
}

fn normalized_activation_restriction(restriction: &ParsedActivationRestriction) -> Option<String> {
    match &restriction.normalization {
        ActivationRestrictionNormalizationFact::Preserve => {
            Some(restriction.presentation_text.clone())
        }
        ActivationRestrictionNormalizationFact::Redundant => None,
        ActivationRestrictionNormalizationFact::Residual(restriction) => Some(restriction.clone()),
    }
}

fn merge_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: Option<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    match (existing, additional) {
        (None, None) => None,
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (Some(left), Some(right)) => {
            Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
        }
    }
}
