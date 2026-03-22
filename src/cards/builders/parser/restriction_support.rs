use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, TriggeredAbility};
use crate::cards::builders::ParsedRestrictions;

use super::activation_and_restrictions::{
    combine_mana_activation_condition, parse_activate_only_timing_lexed,
    parse_activation_condition_lexed, parse_mana_usage_restriction_sentence_lexed,
    parse_triggered_times_each_turn_lexed,
};
use super::lexer::lex_line;

pub(crate) fn apply_pending_restrictions_to_ability(
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
                    apply_pending_mana_restriction(ability, restriction);
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

pub(crate) fn is_restrictable_ability(ability: &Ability) -> bool {
    matches!(
        ability.kind,
        AbilityKind::Activated(_) | AbilityKind::Triggered(_)
    )
}

pub(crate) fn apply_pending_activation_restriction(
    ability: &mut ActivatedAbility,
    restriction: &str,
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

    fn parse_text_only_activation_restriction_condition(
        restriction: &str,
    ) -> Option<crate::ConditionExpr> {
        let lower = restriction
            .trim()
            .to_ascii_lowercase()
            .trim_end_matches('.')
            .to_string();

        if lower.contains("didn't attack this turn")
            || lower.contains("did not attack this turn")
            || lower.contains("has not attacked this turn")
        {
            return Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::SourceAttackedThisTurn,
            )));
        }

        if lower.contains("this creature attacked this turn")
            || lower.contains("it attacked this turn")
            || lower.contains("that creature attacked this turn")
        {
            return Some(crate::ConditionExpr::SourceAttackedThisTurn);
        }

        None
    }

    let tokens = lex_line(restriction, 0).unwrap_or_default();
    let parsed_timing = parse_activate_only_timing_lexed(&tokens);
    let parsed_condition = parse_activation_condition_lexed(&tokens);
    if parsed_condition.is_some() {
        let existing = ability.activation_condition.take();
        ability.activation_condition =
            merge_mana_activation_conditions(existing, parsed_condition.clone());
    }

    let mut timing_applied = false;
    if let Some(parsed_timing) = parsed_timing.as_ref() {
        let merged_timing = merge_activation_timing(&ability.timing, parsed_timing.clone());
        timing_applied = &merged_timing == parsed_timing;
        ability.timing = merged_timing;
        if !timing_applied {
            push_restriction_condition(
                ability,
                crate::ConditionExpr::ActivationTiming(parsed_timing.clone()),
            );
        }
    }

    if let Some(crate::ConditionExpr::MaxActivationsPerTurn(limit)) = parsed_condition {
        push_restriction_condition(ability, crate::ConditionExpr::MaxActivationsPerTurn(limit));
    }

    if let Some(text_condition) = parse_text_only_activation_restriction_condition(restriction) {
        push_restriction_condition(ability, text_condition);
    }

    let restriction = if parsed_timing.is_some() && !timing_applied {
        Some(normalize_restriction_text(restriction))
    } else {
        normalize_activation_restriction(restriction, parsed_timing.as_ref())
    };
    if let Some(restriction) = restriction {
        ability.additional_restrictions.push(restriction);
    }
}

fn apply_pending_trigger_restriction(ability: &mut TriggeredAbility, restriction: &str) {
    let tokens = lex_line(restriction, 0).unwrap_or_default();
    let count = parse_triggered_times_each_turn_lexed(&tokens);
    if let Some(parsed_count) = count {
        ability.intervening_if = Some(match ability.intervening_if.take() {
            Some(crate::ConditionExpr::MaxTimesEachTurn(existing)) => {
                crate::ConditionExpr::MaxTimesEachTurn(existing.min(parsed_count))
            }
            _ => crate::ConditionExpr::MaxTimesEachTurn(parsed_count),
        });
    }
}

pub(crate) fn apply_pending_mana_restriction(ability: &mut ActivatedAbility, restriction: &str) {
    let normalized_restriction = normalize_restriction_text(restriction);
    if normalized_restriction.is_empty() {
        return;
    }
    let tokens = lex_line(&normalized_restriction, 0).unwrap_or_default();
    let parsed_timing = parse_activate_only_timing_lexed(&tokens).unwrap_or_default();
    let parsed_usage_restriction = parse_mana_usage_restriction_sentence_lexed(&tokens);
    let has_usage_restriction = parsed_usage_restriction.is_some();
    let parsed_condition = parse_activation_condition_lexed(&tokens).or_else(|| {
        if parsed_timing == ActivationTiming::AnyTime && !has_usage_restriction {
            Some(crate::ConditionExpr::Unmodeled(
                normalized_restriction.clone(),
            ))
        } else {
            None
        }
    });

    if let Some(restriction) = parsed_usage_restriction {
        ability.mana_usage_restrictions.push(restriction);
    }

    if parsed_condition.is_none()
        && parsed_timing == ActivationTiming::AnyTime
        && !has_usage_restriction
    {
        return;
    }

    if parsed_condition.is_none() && parsed_timing == ActivationTiming::AnyTime {
        return;
    }

    let condition_with_timing = parsed_condition
        .map(|condition| combine_mana_activation_condition(Some(condition), parsed_timing.clone()))
        .unwrap_or_else(|| combine_mana_activation_condition(None, parsed_timing));

    let existing = ability.activation_condition.take();
    ability.activation_condition =
        merge_mana_activation_conditions(existing, condition_with_timing);
}

fn merge_activation_timing(
    existing: &ActivationTiming,
    next: ActivationTiming,
) -> ActivationTiming {
    match (existing, &next) {
        (current, ActivationTiming::AnyTime) => current.clone(),
        (ActivationTiming::AnyTime, _) => next,
        (current, next_timing) if current == next_timing => current.clone(),
        (current, _) => current.clone(),
    }
}

fn normalize_restriction_text(text: &str) -> String {
    text.trim().trim_end_matches('.').trim().to_string()
}

fn normalize_activation_restriction(
    restriction: &str,
    timing: Option<&ActivationTiming>,
) -> Option<String> {
    if timing != Some(&ActivationTiming::OncePerTurn) {
        return Some(restriction.to_string());
    }
    let mut normalized = restriction.to_ascii_lowercase();
    if normalized == "activate only once each turn" {
        return None;
    }
    let prefix = "activate only once each turn and ";
    if normalized.starts_with(prefix) {
        normalized = normalized[prefix.len()..].trim_start().to_string();
    }
    let suffix = " and only once each turn";
    if normalized.ends_with(suffix) {
        normalized = normalized[..normalized.len() - suffix.len()]
            .trim_end()
            .to_string();
    }
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
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
