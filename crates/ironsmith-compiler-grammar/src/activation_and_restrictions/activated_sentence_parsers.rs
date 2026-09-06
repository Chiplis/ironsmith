use super::super::grammar::abilities as ability_grammar;
use super::super::grammar::activated_lines::{
    self as activated_line_grammar, OncePerTurnRestrictionNormalization,
};
use super::super::lexer::OwnedLexToken;
use super::{joined_activation_clause_text, merge_mana_activation_conditions};
use crate::ability::ActivationTiming;
use crate::cards::builders::PredicateAst;
use crate::cards::builders::{EffectAst, PlayerAst};

struct ActivateOnlySentenceDetails {
    timing: ActivationTiming,
    condition: Option<PredicateAst>,
    normalized_restriction: Option<String>,
    once_per_turn_after_other_restrictions: bool,
}

enum ActivatedSentenceModifier {
    ActivateOnly(ActivateOnlySentenceDetails),
    ManaUsageRestriction {
        parsed: Option<crate::model::CompilerManaUsageRestriction>,
        fallback_text: String,
    },
    AdditionalRestriction(String),
    TriggerOnly,
    InlineEffect(EffectAst),
}

pub(super) struct ActivatedSentenceScan<'a> {
    pub(super) kept_sentences: Vec<&'a [OwnedLexToken]>,
    pub(super) timing: ActivationTiming,
    pub(super) mana_activation_condition: Option<PredicateAst>,
    pub(super) additional_activation_restrictions: Vec<String>,
    pub(super) has_exhaust_once_restriction: bool,
    pub(super) mana_usage_restrictions: Vec<crate::model::CompilerManaUsageRestriction>,
    pub(super) inline_effects_ast: Vec<EffectAst>,
}

fn parse_activate_only_sentence_details_lexed(
    tokens: &[OwnedLexToken],
    current_timing: &ActivationTiming,
) -> Option<ActivateOnlySentenceDetails> {
    if !is_activate_only_restriction_sentence_lexed(tokens) {
        return None;
    }

    let timing = parse_activate_only_timing_lexed(tokens).unwrap_or(*current_timing);
    let condition = parse_activation_condition_lexed(tokens)
        .and_then(|condition| strip_once_per_turn_condition_redundancy(condition, &timing));
    let normalized_restriction = normalize_activate_only_restriction(tokens, &timing);
    let once_per_turn_after_other_restrictions = timing == ActivationTiming::OncePerTurn
        && crate::slice_primitives::find_window_by(tokens, 3, |window| {
            window[0].is_word("and") && window[1].is_word("only") && window[2].is_word("once")
        })
        .is_some();
    Some(ActivateOnlySentenceDetails {
        timing,
        condition,
        normalized_restriction,
        once_per_turn_after_other_restrictions,
    })
}

fn strip_once_per_turn_condition_redundancy(
    condition: PredicateAst,
    timing: &ActivationTiming,
) -> Option<PredicateAst> {
    if timing != &ActivationTiming::OncePerTurn {
        return Some(condition);
    }

    match condition {
        PredicateAst::MaxActivationsPerTurn(1) => None,
        PredicateAst::And(left, right) => {
            let left = strip_once_per_turn_condition_redundancy(*left, timing);
            let right = strip_once_per_turn_condition_redundancy(*right, timing);
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

fn parse_next_spell_cost_reduction_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let parsed = activated_line_grammar::parse_next_spell_cost_reduction_tokens(tokens)?;

    Some(EffectAst::subject_verb_reduce_next_spell_cost_this_turn(
        PlayerAst::You,
        parsed.spell_filter,
        parsed.reduction,
    ))
}

fn is_inline_activated_text_modifier_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_line_grammar::parse_inline_activated_sentence_kind_tokens(tokens).is_some()
}

fn parse_activated_sentence_modifier_lexed(
    tokens: &[OwnedLexToken],
    current_timing: &ActivationTiming,
) -> Option<ActivatedSentenceModifier> {
    if let Some(parsed) = parse_activate_only_sentence_details_lexed(tokens, current_timing) {
        return Some(ActivatedSentenceModifier::ActivateOnly(parsed));
    }

    if is_spend_mana_restriction_sentence_lexed(tokens) {
        return Some(ActivatedSentenceModifier::ManaUsageRestriction {
            parsed: parse_mana_usage_restriction_sentence_lexed(tokens),
            fallback_text: joined_activation_clause_text(tokens),
        });
    }

    if ability_grammar::is_mana_spend_bonus_sentence_lexed(tokens) {
        return Some(ActivatedSentenceModifier::ManaUsageRestriction {
            parsed: parse_mana_spend_bonus_sentence_lexed(tokens),
            fallback_text: joined_activation_clause_text(tokens),
        });
    }

    if is_any_player_may_activate_sentence_lexed(tokens) {
        return Some(ActivatedSentenceModifier::AdditionalRestriction(
            joined_activation_clause_text(tokens),
        ));
    }

    if is_trigger_only_restriction_sentence_lexed(tokens) {
        return Some(ActivatedSentenceModifier::TriggerOnly);
    }

    if let Some(effect) = parse_next_spell_cost_reduction_sentence(tokens) {
        return Some(ActivatedSentenceModifier::InlineEffect(effect));
    }

    if is_inline_activated_text_modifier_sentence(tokens) {
        return Some(ActivatedSentenceModifier::AdditionalRestriction(
            joined_activation_clause_text(tokens),
        ));
    }

    None
}

pub(super) fn collect_activated_sentence_modifiers<'a>(
    sentences: &[&'a [OwnedLexToken]],
    initial_timing: ActivationTiming,
) -> ActivatedSentenceScan<'a> {
    let mut timing = initial_timing;
    let mut mana_activation_condition = None;
    let mut additional_activation_restrictions = Vec::new();
    let mut has_exhaust_once_restriction = false;
    let mut mana_usage_restrictions = Vec::new();
    let mut inline_effects_ast = Vec::new();
    let mut kept_sentences = Vec::new();

    for sentence in sentences {
        has_exhaust_once_restriction |= tokens_are_exhaust_once_restriction(sentence);
        let Some(parsed) = parse_activated_sentence_modifier_lexed(sentence, &timing) else {
            kept_sentences.push(*sentence);
            continue;
        };

        match parsed {
            ActivatedSentenceModifier::ActivateOnly(parsed) => {
                timing = parsed.timing;
                if let Some(condition) = parsed.condition {
                    mana_activation_condition =
                        merge_mana_activation_conditions(mana_activation_condition, condition);
                }
                if let Some(restriction) = parsed.normalized_restriction {
                    additional_activation_restrictions.push(restriction);
                }
                if parsed.once_per_turn_after_other_restrictions {
                    additional_activation_restrictions
                        .push("__ironsmith_once_per_turn_after_other_restrictions".to_string());
                }
            }
            ActivatedSentenceModifier::ManaUsageRestriction {
                parsed,
                fallback_text,
            } => {
                if let Some(restriction) = parsed {
                    mana_usage_restrictions.push(restriction);
                } else {
                    additional_activation_restrictions.push(fallback_text);
                }
            }
            ActivatedSentenceModifier::AdditionalRestriction(restriction) => {
                additional_activation_restrictions.push(restriction);
            }
            ActivatedSentenceModifier::TriggerOnly => {}
            ActivatedSentenceModifier::InlineEffect(effect) => {
                inline_effects_ast.push(effect);
            }
        }
    }

    ActivatedSentenceScan {
        kept_sentences,
        timing,
        mana_activation_condition,
        additional_activation_restrictions,
        has_exhaust_once_restriction,
        mana_usage_restrictions,
        inline_effects_ast,
    }
}

pub(super) fn tokens_are_exhaust_once_restriction(tokens: &[OwnedLexToken]) -> bool {
    activated_line_grammar::parse_exhaust_once_restriction_tokens(tokens).is_some()
}

pub fn parse_activate_only_timing_lexed(tokens: &[OwnedLexToken]) -> Option<ActivationTiming> {
    ability_grammar::parse_activate_only_timing_lexed(tokens)
}

pub fn normalize_activate_only_restriction(
    tokens: &[OwnedLexToken],
    timing: &ActivationTiming,
) -> Option<String> {
    if timing == &ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep {
        return None;
    }
    if timing != &ActivationTiming::OncePerTurn {
        return Some(crate::lexer::token_word_refs(tokens).join(" "));
    }
    match activated_line_grammar::parse_once_per_turn_restriction_normalization_tokens(tokens) {
        OncePerTurnRestrictionNormalization::Redundant => None,
        OncePerTurnRestrictionNormalization::Residual(restriction) => Some(restriction),
    }
}

pub fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_activate_only_restriction_sentence_lexed(tokens)
}

pub fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::CompilerManaUsageRestriction> {
    ability_grammar::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::CompilerManaUsageRestriction> {
    ability_grammar::parse_mana_spend_bonus_sentence_lexed(tokens)
}

pub fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_any_player_may_activate_sentence_lexed(tokens)
}

pub fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    ability_grammar::parse_triggered_times_each_turn_lexed(tokens)
}

pub fn parse_activation_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::cards::builders::PredicateAst> {
    ability_grammar::parse_activation_condition_lexed(tokens)
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::SourcePredicateAst;
    use super::*;
    use crate::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("activation restriction should lex")
    }

    #[test]
    fn once_each_turn_timing_does_not_duplicate_the_max_activation_condition() {
        let sentence = lex("Activate only once each turn.");
        let details =
            parse_activate_only_sentence_details_lexed(&sentence, &ActivationTiming::AnyTime)
                .expect("restriction should parse");

        assert_eq!(details.timing, ActivationTiming::OncePerTurn);
        assert_eq!(details.condition, None);
        assert!(!details.once_per_turn_after_other_restrictions);
    }

    #[test]
    fn once_each_turn_keeps_an_independent_activation_condition() {
        let sentence =
            lex("Activate only once each turn and only if this creature attacked this turn.");
        let details =
            parse_activate_only_sentence_details_lexed(&sentence, &ActivationTiming::AnyTime)
                .expect("restriction should parse");

        assert_eq!(details.timing, ActivationTiming::OncePerTurn);
        assert_eq!(
            details.condition,
            Some(PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn))
        );
        assert!(!details.once_per_turn_after_other_restrictions);
    }

    #[test]
    fn once_each_turn_keeps_an_independent_turn_timing() {
        let sentence = lex("Activate only during your turn and only once each turn.");
        let details =
            parse_activate_only_sentence_details_lexed(&sentence, &ActivationTiming::AnyTime)
                .expect("combined restriction should parse");

        assert_eq!(details.timing, ActivationTiming::OncePerTurn);
        assert_eq!(
            details.condition,
            Some(PredicateAst::ActivationTiming(
                ActivationTiming::DuringYourTurn
            ))
        );
        assert!(details.once_per_turn_after_other_restrictions);
    }

    #[test]
    fn trailing_once_each_turn_order_is_kept_with_a_residual_condition() {
        let sentence =
            lex("Activate only if an opponent lost life this turn and only once each turn.");
        let details =
            parse_activate_only_sentence_details_lexed(&sentence, &ActivationTiming::AnyTime)
                .expect("combined restriction should parse");

        assert_eq!(details.timing, ActivationTiming::OncePerTurn);
        assert!(
            details
                .normalized_restriction
                .as_deref()
                .is_some_and(|restriction| restriction.contains("opponent lost life this turn"))
        );
        assert!(details.once_per_turn_after_other_restrictions);
    }

    #[test]
    fn any_player_before_end_step_is_a_typed_activator_relative_window() {
        let sentence = lex(
            "Any player may activate this ability but only during their turn before the end step.",
        );
        let details =
            parse_activate_only_sentence_details_lexed(&sentence, &ActivationTiming::AnyTime)
                .expect("combined authority and timing sentence should parse");

        assert_eq!(
            details.timing,
            ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep
        );
        assert_eq!(details.condition, None);
        assert_eq!(details.normalized_restriction, None);

        let scan = collect_activated_sentence_modifiers(&[&sentence], ActivationTiming::AnyTime);
        assert_eq!(
            scan.timing,
            ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep
        );
        assert!(scan.additional_activation_restrictions.is_empty());
        assert!(scan.kept_sentences.is_empty());
    }

    #[test]
    fn mana_ability_keeps_a_trailing_spending_restriction() {
        let tokens = lex(
            "{T}: Add {C}. Spend this mana only to cast an artifact spell or activate an ability.",
        );
        let parsed = crate::activation_and_restrictions::parse_activated_line_with_raw(&tokens)
            .expect("restricted mana line should parse")
            .expect("restricted mana ability");
        let debug = format!("{parsed:#?}");
        assert!(debug.contains("mana_usage_restrictions"), "{debug}");
        assert!(
            debug.contains("CastSpellOrActivateAbilitySourceMatching")
                || debug.contains("PaymentTransaction"),
            "{debug}"
        );
    }
}
