use crate::ability::ActivationTiming;
use crate::cards::builders::PredicateAst;
use crate::model::compiler_semantic::{CompilerActivatedAbilityCore, ParsedManaRestriction};

use super::activation_and_restrictions::combine_mana_activation_condition;

/// Applies typed mana restrictions while the ability is still compiler-owned.
/// Runtime materialization of nested on-spend effects happens at lowering.
pub fn apply_pending_mana_restrictions(
    parsed: &mut crate::cards::builders::ParsedAbility,
    restrictions: &[ParsedManaRestriction],
) -> Result<(), crate::cards::builders::CardTextError> {
    let crate::model::CompilerAbilityKindCore::Activated(ability) = parsed.kind_mut() else {
        return Err(crate::cards::builders::CardTextError::InvariantViolation(
            "activated restriction preparation expected activated ability kind".to_string(),
        ));
    };
    for restriction in restrictions {
        apply_pending_mana_restriction(ability, restriction);
    }
    Ok(())
}

fn apply_pending_mana_restriction(
    ability: &mut CompilerActivatedAbilityCore,
    restriction: &ParsedManaRestriction,
) {
    if let Some(restriction) = restriction.usage_restriction.clone() {
        ability.mana_usage_restrictions.push(restriction);
    }

    if restriction.condition.is_none() && restriction.timing == ActivationTiming::AnyTime {
        return;
    }

    let condition_with_timing = restriction
        .condition
        .clone()
        .map(|condition| combine_mana_activation_condition(Some(condition), restriction.timing))
        .unwrap_or_else(|| combine_mana_activation_condition(None, restriction.timing));

    ability.activation_condition =
        merge_conditions(ability.activation_condition.take(), condition_with_timing);
}

fn merge_conditions(
    existing: Option<PredicateAst>,
    additional: Option<PredicateAst>,
) -> Option<PredicateAst> {
    match (existing, additional) {
        (None, None) => None,
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (Some(left), Some(right)) => Some(PredicateAst::And(Box::new(left), Box::new(right))),
    }
}
