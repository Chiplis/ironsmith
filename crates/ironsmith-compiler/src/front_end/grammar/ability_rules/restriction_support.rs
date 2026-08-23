use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, TriggeredAbility};
use crate::model::compiler_semantic::{
    ActivationRestrictionNormalizationFact, ParsedActivationRestriction, ParsedManaRestriction,
    ParsedRestrictions, ParsedTriggerRestriction,
};

use super::activation_and_restrictions::combine_mana_activation_condition;

pub fn apply_pending_restrictions_to_ability(
    ability: &mut Ability,
    pending: &mut ParsedRestrictions,
) {
    let activation_restrictions = std::mem::take(&mut pending.activation);
    let trigger_restrictions = std::mem::take(&mut pending.trigger);

    match &mut ability.kind {
        AbilityKind::Activated(ability) => {
            if activation_restrictions.is_empty() {
                return;
            }
            if ability.is_mana_ability() {
                for restriction in &activation_restrictions {
                    apply_pending_activation_restriction_to_mana_ability(ability, restriction);
                }
            } else {
                for restriction in &activation_restrictions {
                    apply_pending_activation_restriction(ability, restriction);
                }
            }
        }
        AbilityKind::Triggered(ability) => {
            if trigger_restrictions.is_empty() {
                return;
            }
            for restriction in &trigger_restrictions {
                apply_pending_trigger_restriction(ability, restriction);
            }
        }
        _ => {}
    }

    if !activation_restrictions.is_empty() {
        pending.activation.extend(activation_restrictions);
    }
    if !trigger_restrictions.is_empty() {
        pending.trigger.extend(trigger_restrictions);
    }
}

pub fn is_restrictable_ability(ability: &Ability) -> bool {
    matches!(
        ability.kind,
        AbilityKind::Activated(_) | AbilityKind::Triggered(_)
    )
}

pub fn apply_pending_activation_restriction(
    ability: &mut ActivatedAbility,
    restriction: &ParsedActivationRestriction,
) {
    fn push_restriction_condition(ability: &mut ActivatedAbility, condition: crate::ConditionExpr) {
        if !ability
            .activation_restrictions
            .iter()
            .any(|existing| existing == &condition)
        {
            ability.activation_restrictions.push(condition);
        }
    }

    if restriction.condition.is_some() {
        let existing = ability.activation_condition.take();
        ability.activation_condition =
            merge_mana_activation_conditions(existing, restriction.condition.clone());
    }

    let mut timing_applied = false;
    if let Some(parsed_timing) = restriction.timing.as_ref() {
        let merged_timing = merge_activation_timing(&ability.timing, *parsed_timing);
        timing_applied = &merged_timing == parsed_timing;
        ability.timing = merged_timing;
        if !timing_applied {
            push_restriction_condition(
                ability,
                crate::ConditionExpr::ActivationTiming(*parsed_timing),
            );
        }
    }

    if let Some(crate::ConditionExpr::MaxActivationsPerTurn(limit)) = &restriction.condition {
        push_restriction_condition(ability, crate::ConditionExpr::MaxActivationsPerTurn(*limit));
    }

    if let Some(text_condition) = restriction.text_only_condition.clone() {
        push_restriction_condition(ability, text_condition);
    }

    let additional_restriction = if restriction.timing.is_some() && !timing_applied {
        Some(restriction.presentation_text.clone())
    } else {
        normalized_activation_restriction(restriction)
    };
    if let Some(restriction) = additional_restriction {
        ability.additional_restrictions.push(restriction);
    }
    if restriction.once_per_turn_after_other_restrictions
        && ability
            .additional_restrictions
            .iter()
            .all(|existing| existing != "__ironsmith_once_per_turn_after_other_restrictions")
    {
        ability
            .additional_restrictions
            .push("__ironsmith_once_per_turn_after_other_restrictions".to_string());
    }
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

pub fn apply_pending_mana_restriction(
    ability: &mut ActivatedAbility,
    restriction: &ParsedManaRestriction,
) {
    apply_mana_restriction_facts(
        ability,
        restriction.timing,
        restriction.condition.clone(),
        restriction.usage_restriction.clone(),
    );
}

/// Applies the typed mana restrictions collected while parsing one activated
/// ability. This belongs beside restriction preparation rather than in runtime
/// lowering: no Oracle surface is inspected here.
pub fn apply_pending_mana_restrictions(
    parsed: &mut crate::cards::builders::ParsedAbility,
    restrictions: &[crate::model::compiler_semantic::ParsedManaRestriction],
) -> Result<(), crate::cards::builders::CardTextError> {
    let crate::ability::AbilityKind::Activated(ability) = parsed.kind_mut() else {
        return Err(crate::cards::builders::CardTextError::InvariantViolation(
            "activated restriction preparation expected activated ability kind".to_string(),
        ));
    };
    for restriction in restrictions {
        apply_pending_mana_restriction(ability, restriction);
    }
    Ok(())
}

fn apply_pending_activation_restriction_to_mana_ability(
    ability: &mut ActivatedAbility,
    restriction: &ParsedActivationRestriction,
) {
    apply_mana_restriction_facts(
        ability,
        restriction.timing.unwrap_or_default(),
        restriction.condition.clone(),
        restriction.mana_usage_restriction.clone(),
    );
}

fn apply_mana_restriction_facts(
    ability: &mut ActivatedAbility,
    timing: ActivationTiming,
    condition: Option<crate::ConditionExpr>,
    usage_restriction: Option<crate::ability::ManaUsageRestriction>,
) {
    if let Some(restriction) = usage_restriction {
        ability.mana_usage_restrictions.push(restriction);
    }

    if condition.is_none() && timing == ActivationTiming::AnyTime {
        return;
    }

    let condition_with_timing = condition
        .map(|condition| combine_mana_activation_condition(Some(condition), timing))
        .unwrap_or_else(|| combine_mana_activation_condition(None, timing));

    let existing = ability.activation_condition.take();
    ability.activation_condition =
        merge_mana_activation_conditions(existing, condition_with_timing);
}

fn merge_activation_timing(
    existing: &ActivationTiming,
    next: ActivationTiming,
) -> ActivationTiming {
    match (existing, &next) {
        (current, ActivationTiming::AnyTime) => *current,
        (ActivationTiming::AnyTime, _) => next,
        (current, next_timing) if current == next_timing => *current,
        (current, _) => *current,
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

fn merge_mana_activation_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: Option<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    match (existing, additional) {
        (None, None) => None,
        (Some(condition), None) => Some(condition),
        (None, Some(condition)) => Some(condition),
        (Some(left), Some(right)) => {
            Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
        }
    }
}
