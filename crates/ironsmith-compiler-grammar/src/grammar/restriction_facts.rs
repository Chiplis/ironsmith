use crate::ability::ActivationTiming;
use crate::cards::builders::PredicateAst;
use crate::lexer::{OwnedLexToken, render_token_slice};
use crate::model::compiler_semantic::{
    ActivationRestrictionNormalizationFact, ParsedActivationRestriction, ParsedManaRestriction,
    ParsedTriggerRestriction,
};

use super::abilities;
use super::restriction_normalization::{
    ActivationRestrictionNormalization, TextOnlyActivationRestriction,
    parse_once_per_turn_activation_restriction_tokens,
    parse_text_only_activation_restriction_tokens,
};

pub fn parse_activation_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ParsedActivationRestriction> {
    abilities::is_activate_only_restriction_sentence_lexed(tokens)
        .then(|| parse_activation_restriction_surface_tokens(tokens))
}

pub fn parse_activation_restriction_surface_tokens(
    tokens: &[OwnedLexToken],
) -> ParsedActivationRestriction {
    let timing = abilities::parse_activate_only_timing_lexed(tokens);
    let normalization = if timing == Some(ActivationTiming::OncePerTurn) {
        match parse_once_per_turn_activation_restriction_tokens(tokens) {
            ActivationRestrictionNormalization::Redundant => {
                ActivationRestrictionNormalizationFact::Redundant
            }
            ActivationRestrictionNormalization::Residual(text) => {
                ActivationRestrictionNormalizationFact::Residual(text)
            }
        }
    } else {
        ActivationRestrictionNormalizationFact::Preserve
    };
    let condition = abilities::parse_activation_condition_lexed(tokens)
        .and_then(|condition| strip_redundant_once_per_turn_condition(condition, timing.as_ref()));
    ParsedActivationRestriction {
        presentation_text: normalized_surface(tokens),
        timing,
        condition,
        text_only_condition: parse_text_only_activation_restriction_tokens(tokens)
            .map(text_only_condition),
        normalization,
        mana_usage_restriction: abilities::parse_mana_usage_restriction_sentence_lexed(tokens)
            .or_else(|| abilities::parse_mana_spend_bonus_sentence_lexed(tokens)),
        once_per_turn_after_other_restrictions: timing == Some(ActivationTiming::OncePerTurn)
            && crate::slice_primitives::find_window_by(tokens, 3, |window| {
                window[0].is_word("and") && window[1].is_word("only") && window[2].is_word("once")
            })
            .is_some(),
    }
}

fn strip_redundant_once_per_turn_condition(
    condition: PredicateAst,
    timing: Option<&ActivationTiming>,
) -> Option<PredicateAst> {
    if timing != Some(&ActivationTiming::OncePerTurn) {
        return Some(condition);
    }

    match condition {
        PredicateAst::MaxActivationsPerTurn(1) => None,
        PredicateAst::And(left, right) => {
            let left = strip_redundant_once_per_turn_condition(*left, timing);
            let right = strip_redundant_once_per_turn_condition(*right, timing);
            match (left, right) {
                (Some(left), Some(right)) => {
                    Some(PredicateAst::And(Box::new(left), Box::new(right)))
                }
                (Some(condition), None) | (None, Some(condition)) => Some(condition),
                (None, None) => None,
            }
        }
        condition => Some(condition),
    }
}

pub fn parse_trigger_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ParsedTriggerRestriction> {
    abilities::is_trigger_only_restriction_sentence_lexed(tokens).then(|| {
        ParsedTriggerRestriction {
            presentation_text: normalized_surface(tokens),
            max_times_each_turn: abilities::parse_triggered_times_each_turn_lexed(tokens),
        }
    })
}

pub fn parse_mana_restriction_tokens(tokens: &[OwnedLexToken]) -> Option<ParsedManaRestriction> {
    let usage_restriction = abilities::parse_mana_usage_restriction_sentence_lexed(tokens)
        .or_else(|| abilities::parse_mana_spend_bonus_sentence_lexed(tokens));
    let recognized = usage_restriction.is_some()
        || abilities::is_spend_mana_restriction_sentence_lexed(tokens)
        || abilities::is_mana_spend_bonus_sentence_lexed(tokens);
    recognized.then(|| parse_mana_restriction_surface_tokens(tokens))
}

pub fn parse_mana_restriction_surface_tokens(tokens: &[OwnedLexToken]) -> ParsedManaRestriction {
    ParsedManaRestriction {
        presentation_text: normalized_surface(tokens),
        timing: abilities::parse_activate_only_timing_lexed(tokens).unwrap_or_default(),
        condition: abilities::parse_activation_condition_lexed(tokens),
        usage_restriction: abilities::parse_mana_usage_restriction_sentence_lexed(tokens)
            .or_else(|| abilities::parse_mana_spend_bonus_sentence_lexed(tokens)),
    }
}

fn text_only_condition(parsed: TextOnlyActivationRestriction) -> PredicateAst {
    match parsed {
        TextOnlyActivationRestriction::SourceDidNotAttackThisTurn => {
            PredicateAst::Not(Box::new(PredicateAst::SourceAttackedThisTurn))
        }
        TextOnlyActivationRestriction::SourceAttackedThisTurn => {
            PredicateAst::SourceAttackedThisTurn
        }
    }
}

fn normalized_surface(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens)
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_string()
}

#[cfg(test)]
#[path = "restriction_facts_inline_tests.rs"]
mod tests;
