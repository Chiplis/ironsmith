use crate::ConditionExpr;
use crate::ability::ActivationTiming;
use crate::runtime_backend::lexer::{OwnedLexToken, render_token_slice};
use crate::runtime_backend::semantic::{
    ActivationRestrictionNormalizationFact, ParsedActivationRestriction, ParsedManaRestriction,
    ParsedTriggerRestriction,
};

use super::abilities;
use super::restriction_normalization::{
    ActivationRestrictionNormalization, TextOnlyActivationRestriction,
    parse_once_per_turn_activation_restriction_tokens,
    parse_text_only_activation_restriction_tokens,
};

pub(crate) fn parse_activation_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ParsedActivationRestriction> {
    abilities::is_activate_only_restriction_sentence_lexed(tokens)
        .then(|| parse_activation_restriction_surface_tokens(tokens))
}

pub(crate) fn parse_activation_restriction_surface_tokens(
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
    ParsedActivationRestriction {
        presentation_text: normalized_surface(tokens),
        timing,
        condition: abilities::parse_activation_condition_lexed(tokens),
        text_only_condition: parse_text_only_activation_restriction_tokens(tokens)
            .map(text_only_condition),
        normalization,
        mana_usage_restriction: abilities::parse_mana_usage_restriction_sentence_lexed(tokens)
            .or_else(|| abilities::parse_mana_spend_bonus_sentence_lexed(tokens)),
    }
}

pub(crate) fn parse_trigger_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ParsedTriggerRestriction> {
    abilities::is_trigger_only_restriction_sentence_lexed(tokens).then(|| {
        ParsedTriggerRestriction {
            presentation_text: normalized_surface(tokens),
            max_times_each_turn: abilities::parse_triggered_times_each_turn_lexed(tokens),
        }
    })
}

pub(crate) fn parse_mana_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ParsedManaRestriction> {
    let usage_restriction = abilities::parse_mana_usage_restriction_sentence_lexed(tokens)
        .or_else(|| abilities::parse_mana_spend_bonus_sentence_lexed(tokens));
    let recognized = usage_restriction.is_some()
        || abilities::is_spend_mana_restriction_sentence_lexed(tokens)
        || abilities::is_mana_spend_bonus_sentence_lexed(tokens);
    recognized.then(|| parse_mana_restriction_surface_tokens(tokens))
}

pub(crate) fn parse_mana_restriction_surface_tokens(
    tokens: &[OwnedLexToken],
) -> ParsedManaRestriction {
    ParsedManaRestriction {
        presentation_text: normalized_surface(tokens),
        timing: abilities::parse_activate_only_timing_lexed(tokens).unwrap_or_default(),
        condition: abilities::parse_activation_condition_lexed(tokens),
        usage_restriction: abilities::parse_mana_usage_restriction_sentence_lexed(tokens)
            .or_else(|| abilities::parse_mana_spend_bonus_sentence_lexed(tokens)),
    }
}

fn text_only_condition(parsed: TextOnlyActivationRestriction) -> ConditionExpr {
    match parsed {
        TextOnlyActivationRestriction::SourceDidNotAttackThisTurn => {
            ConditionExpr::Not(Box::new(ConditionExpr::SourceAttackedThisTurn))
        }
        TextOnlyActivationRestriction::SourceAttackedThisTurn => {
            ConditionExpr::SourceAttackedThisTurn
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
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("restriction should lex")
    }

    #[test]
    fn activation_fact_carries_timing_condition_and_residual_text() {
        let parsed = parse_activation_restriction_tokens(&lex(
            "Activate only once each turn and only if this creature attacked this turn.",
        ))
        .expect("activation restriction should parse");

        assert_eq!(parsed.timing, Some(ActivationTiming::OncePerTurn));
        assert_eq!(
            parsed.text_only_condition,
            Some(ConditionExpr::SourceAttackedThisTurn)
        );
        assert_eq!(
            parsed.normalization,
            ActivationRestrictionNormalizationFact::Residual(
                "only if this creature attacked this turn".to_string()
            )
        );
    }

    #[test]
    fn trigger_and_mana_facts_are_typed_before_lowering() {
        let trigger =
            parse_trigger_restriction_tokens(&lex("This ability triggers only twice each turn."))
                .expect("trigger restriction should parse");
        assert_eq!(trigger.max_times_each_turn, Some(2));

        let mana =
            parse_mana_restriction_tokens(&lex("Spend this mana only to cast artifact spells."))
                .expect("mana restriction should parse");
        assert!(mana.usage_restriction.is_some());
    }
}
