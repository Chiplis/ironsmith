use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, TriggeredAbility};
use crate::runtime_backend::semantic::ParsedRestrictions;

use super::activation_and_restrictions::{
    combine_mana_activation_condition, parse_activate_only_timing_lexed,
    parse_activation_condition_lexed, parse_mana_spend_bonus_sentence_lexed,
    parse_mana_usage_restriction_sentence_lexed, parse_triggered_times_each_turn_lexed,
};
use super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};
use super::lexer::{LexedClause, OwnedLexToken, TokenWordView, lex_line, render_token_slice};

const ACTIVATE_ONLY_ONCE_EACH_TURN_PHRASE: &[&str] = &["activate", "only", "once", "each", "turn"];
const ACTIVATE_ONLY_ONCE_EACH_TURN_AND_PREFIX: &[&str] =
    &["activate", "only", "once", "each", "turn", "and"];
const AND_ONLY_ONCE_EACH_TURN_SUFFIX: &[&str] = &["and", "only", "once", "each", "turn"];
const DID_NOT_ATTACK_THIS_TURN_PHRASES: &[&[&str]] = &[
    &["didn't", "attack", "this", "turn"],
    &["didnt", "attack", "this", "turn"],
    &["did", "not", "attack", "this", "turn"],
    &["has", "not", "attacked", "this", "turn"],
    &["hasnt", "attacked", "this", "turn"],
    &["hasn't", "attacked", "this", "turn"],
];
const SOURCE_ATTACKED_THIS_TURN_SUBJECTS: &[&[&str]] =
    &[&["this", "creature"], &["it"], &["that", "creature"]];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextOnlyActivationRestriction {
    SourceDidNotAttackThisTurn,
    SourceAttackedThisTurn,
}

fn text_only_activation_restriction_shape(
    tokens: &[OwnedLexToken],
) -> Option<TextOnlyActivationRestriction> {
    const ACTIVATE_ONLY_IF_PREFIX: &[&str] = &["activate", "only", "if"];
    const ONLY_IF_PREFIX: &[&str] = &["only", "if"];
    const DID_NOT_ATTACK_THIS_TURN_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::action(
            "attack_condition",
            LexCaptureKind::OneOfPhrase(DID_NOT_ATTACK_THIS_TURN_PHRASES),
        )]);
    const SOURCE_ATTACKED_THIS_TURN_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "source",
            LexCaptureKind::OneOfPhrase(SOURCE_ATTACKED_THIS_TURN_SUBJECTS),
        ),
        LexPattern::action(
            "attack_condition",
            LexCaptureKind::OneOfPhrase(&[&["attacked", "this", "turn"]]),
        ),
    ]);

    fn strip_prefix_words_ci<'a>(
        tokens: &'a [OwnedLexToken],
        phrase: &[&str],
    ) -> Option<&'a [OwnedLexToken]> {
        let words = TokenWordView::new(tokens);
        let word_refs = words.word_refs();
        if word_refs.len() < phrase.len()
            || !word_refs
                .iter()
                .zip(phrase.iter())
                .all(|(word, expected)| word.eq_ignore_ascii_case(expected))
        {
            return None;
        }
        let token_idx = words.token_index_after_words(phrase.len())?;
        Some(&tokens[token_idx..])
    }

    fn strip_suffix_words_ci<'a>(
        tokens: &'a [OwnedLexToken],
        phrase: &[&str],
    ) -> Option<&'a [OwnedLexToken]> {
        let words = TokenWordView::new(tokens);
        let word_refs = words.word_refs();
        if word_refs.len() < phrase.len() {
            return None;
        }
        let start = word_refs.len() - phrase.len();
        if !word_refs[start..]
            .iter()
            .zip(phrase.iter())
            .all(|(word, expected)| word.eq_ignore_ascii_case(expected))
        {
            return None;
        }
        let range = words.token_range_for_word_range(start, word_refs.len())?;
        Some(&tokens[..range.start])
    }

    let mut tokens = restriction_tokens_without_terminal_period(tokens);
    if let Some(stripped) = strip_prefix_words_ci(tokens, ACTIVATE_ONLY_ONCE_EACH_TURN_AND_PREFIX) {
        tokens = stripped;
    }
    if let Some(stripped) = strip_prefix_words_ci(tokens, ACTIVATE_ONLY_IF_PREFIX) {
        tokens = stripped;
    } else if let Some(stripped) = strip_prefix_words_ci(tokens, ONLY_IF_PREFIX) {
        tokens = stripped;
    }
    if let Some(stripped) = strip_suffix_words_ci(tokens, AND_ONLY_ONCE_EACH_TURN_SUFFIX) {
        tokens = stripped;
    }

    let clause = LexedClause::new(tokens);
    if let Some(matched) = DID_NOT_ATTACK_THIS_TURN_PATTERN.match_clause(clause)
        && matched
            .capture_clause_by_role(LexCaptureRole::Action, clause)
            .is_some()
    {
        return Some(TextOnlyActivationRestriction::SourceDidNotAttackThisTurn);
    }
    for subject in SOURCE_ATTACKED_THIS_TURN_SUBJECTS {
        let Some(tail) = strip_prefix_words_ci(tokens, subject) else {
            continue;
        };
        let clause = LexedClause::new(tail);
        if let Some(matched) = DID_NOT_ATTACK_THIS_TURN_PATTERN.match_clause(clause)
            && matched
                .capture_clause_by_role(LexCaptureRole::Action, clause)
                .is_some()
        {
            return Some(TextOnlyActivationRestriction::SourceDidNotAttackThisTurn);
        }
    }
    if let Some(matched) = SOURCE_ATTACKED_THIS_TURN_PATTERN.match_clause(clause)
        && matched
            .capture_clause_by_role(LexCaptureRole::Subject, clause)
            .is_some()
        && matched
            .capture_clause_by_role(LexCaptureRole::Action, clause)
            .is_some()
    {
        return Some(TextOnlyActivationRestriction::SourceAttackedThisTurn);
    }
    None
}

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

    fn parse_text_only_activation_restriction_condition_tokens(
        tokens: &[OwnedLexToken],
    ) -> Option<crate::ConditionExpr> {
        match text_only_activation_restriction_shape(tokens)? {
            TextOnlyActivationRestriction::SourceDidNotAttackThisTurn => Some(
                crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::SourceAttackedThisTurn)),
            ),
            TextOnlyActivationRestriction::SourceAttackedThisTurn => {
                Some(crate::ConditionExpr::SourceAttackedThisTurn)
            }
        }
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

    if let Some(text_condition) = parse_text_only_activation_restriction_condition_tokens(&tokens) {
        push_restriction_condition(ability, text_condition);
    }

    let restriction = if parsed_timing.is_some() && !timing_applied {
        Some(normalize_restriction_text(restriction))
    } else {
        normalize_activation_restriction(restriction, &tokens, parsed_timing.as_ref())
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
    let parsed_usage_restriction = parse_mana_usage_restriction_sentence_lexed(&tokens)
        .or_else(|| parse_mana_spend_bonus_sentence_lexed(&tokens));
    let has_usage_restriction = parsed_usage_restriction.is_some();
    let parsed_condition = parse_activation_condition_lexed(&tokens);

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
    tokens: &[OwnedLexToken],
    timing: Option<&ActivationTiming>,
) -> Option<String> {
    if timing != Some(&ActivationTiming::OncePerTurn) {
        return Some(restriction.to_string());
    }
    let tokens = restriction_tokens_without_terminal_period(tokens);
    let words = TokenWordView::new(tokens);
    if words.word_refs() == ACTIVATE_ONLY_ONCE_EACH_TURN_PHRASE {
        return None;
    }
    let mut start = 0usize;
    let mut end = tokens.len();

    if words.starts_with(ACTIVATE_ONLY_ONCE_EACH_TURN_AND_PREFIX)
        && let Some(token_idx) =
            words.token_index_after_words(ACTIVATE_ONLY_ONCE_EACH_TURN_AND_PREFIX.len())
    {
        start = token_idx;
    }

    if words.ends_with(AND_ONLY_ONCE_EACH_TURN_SUFFIX)
        && let Some(range) = words.token_range_for_word_range(
            words.len() - AND_ONLY_ONCE_EACH_TURN_SUFFIX.len(),
            words.len(),
        )
    {
        end = range.start;
    }

    let normalized = lowercase_first_ascii(render_token_slice(&tokens[start..end]).trim());
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn lowercase_first_ascii(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut lowered = first.to_ascii_lowercase().to_string();
    lowered.push_str(chars.as_str());
    lowered
}

fn restriction_tokens_without_terminal_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens.last().is_some_and(|token| token.parser_text == ".") {
        &tokens[..tokens.len().saturating_sub(1)]
    } else {
        tokens
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
