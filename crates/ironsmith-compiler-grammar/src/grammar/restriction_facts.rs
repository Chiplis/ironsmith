use crate::ability::ActivationTiming;
use crate::cards::builders::PredicateAst;
use crate::cards::builders::SourcePredicateAst;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrailingOnceLimit {
    EachTurn,
    Lifetime,
}

/// Split a trailing "and only once [each turn]" off an "Activate only ..."
/// sentence ("Activate only as a sorcery and only once each turn.",
/// "Activate only if ... and only once."). A bare "Activate only once." is a
/// lifetime limit with nothing left over.
fn split_trailing_once_limit(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], TrailingOnceLimit)> {
    let word_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.as_word().is_some())
        .map(|(idx, _)| idx)
        .collect();
    let words: Vec<&str> = word_positions
        .iter()
        .map(|&idx| tokens[idx].parser_text())
        .collect();
    if words.as_slice() == ["activate", "only", "once"]
        || words.as_slice() == ["activate", "this", "ability", "only", "once"]
    {
        return Some((&[], TrailingOnceLimit::Lifetime));
    }
    let (tail_len, limit) = if words.ends_with(&["and", "only", "once", "each", "turn"]) {
        (5, TrailingOnceLimit::EachTurn)
    } else if words.ends_with(&["and", "only", "once"]) {
        (3, TrailingOnceLimit::Lifetime)
    } else {
        return None;
    };
    if words.len() <= tail_len + 2 {
        return None;
    }
    let and_idx = word_positions[words.len() - tail_len];
    Some((&tokens[..and_idx], limit))
}

pub fn parse_activation_restriction_surface_tokens(
    tokens: &[OwnedLexToken],
) -> ParsedActivationRestriction {
    let mut timing = abilities::parse_activate_only_timing_lexed(tokens);
    let mut condition = abilities::parse_activation_condition_lexed(tokens)
        .and_then(|condition| strip_redundant_once_per_turn_condition(condition, timing.as_ref()));
    // CR 602.5b: every stated restriction applies. A single-timing parse keeps
    // only the first timing of "as a sorcery and only once each turn" or
    // "during your upkeep and only once each turn", and "only once" has no
    // timing at all, so re-read the leading part and add the limit
    // explicitly. The combined "during your turn and only once each turn"
    // shape is already fully read as OncePerTurn plus a timing condition.
    if let Some((leading, limit)) = split_trailing_once_limit(tokens)
        && !(limit == TrailingOnceLimit::EachTurn
            && timing == Some(ActivationTiming::OncePerTurn)
            && condition.is_some())
    {
        let (leading_timing, leading_condition) = if leading.is_empty() {
            (None, None)
        } else {
            (
                abilities::parse_activate_only_timing_lexed(leading),
                abilities::parse_activation_condition_lexed(leading),
            )
        };
        if leading.is_empty() || leading_timing.is_some() || leading_condition.is_some() {
            let limit = match limit {
                TrailingOnceLimit::EachTurn => PredicateAst::MaxActivationsPerTurn(1),
                TrailingOnceLimit::Lifetime => PredicateAst::MaxActivationsPerObject(1),
            };
            timing = leading_timing;
            condition = Some(match leading_condition {
                Some(leading_condition) => {
                    PredicateAst::And(Box::new(leading_condition), Box::new(limit))
                }
                None => limit,
            });
        }
    }
    // "Activate only during your upkeep and only if you control a Swamp.":
    // the timing parse reads the window only; read the "only if" half as its
    // own condition so it is not dropped.
    if condition.is_none()
        && timing.is_some_and(|timing| timing != ActivationTiming::OncePerTurn)
        && let Some(and_idx) = crate::slice_primitives::find_window_by(tokens, 3, |window| {
            window[0].is_word("and") && window[1].is_word("only") && window[2].is_word("if")
        })
    {
        let mut prefixed_right = vec![
            OwnedLexToken::synthetic_word("activate"),
            OwnedLexToken::synthetic_word("only"),
            OwnedLexToken::synthetic_word("if"),
        ];
        prefixed_right.extend_from_slice(&tokens[and_idx + 3..]);
        condition = abilities::parse_activation_condition_lexed(&prefixed_right);
    }
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
        TextOnlyActivationRestriction::SourceDidNotAttackThisTurn => PredicateAst::Not(Box::new(
            PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn),
        )),
        TextOnlyActivationRestriction::SourceAttackedThisTurn => {
            PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn)
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
