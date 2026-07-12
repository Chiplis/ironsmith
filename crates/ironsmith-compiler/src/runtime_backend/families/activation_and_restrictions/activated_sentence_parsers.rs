use super::super::grammar::abilities as ability_grammar;
use super::super::grammar::activated_lines::{
    self as activated_line_grammar, OncePerTurnRestrictionNormalization,
};
use super::super::lexer::OwnedLexToken;
use super::{joined_activation_clause_text, merge_mana_activation_conditions};
use crate::ability::ActivationTiming;
use crate::cards::builders::{EffectAst, PlayerAst};

struct ActivateOnlySentenceDetails {
    timing: ActivationTiming,
    condition: Option<crate::ConditionExpr>,
    normalized_restriction: Option<String>,
}

enum ActivatedSentenceModifier {
    ActivateOnly(ActivateOnlySentenceDetails),
    ManaUsageRestriction {
        parsed: Option<crate::ability::ManaUsageRestriction>,
        fallback_text: String,
    },
    AdditionalRestriction(String),
    TriggerOnly,
    InlineEffect(EffectAst),
}

pub(super) struct ActivatedSentenceScan<'a> {
    pub(super) kept_sentences: Vec<&'a [OwnedLexToken]>,
    pub(super) timing: ActivationTiming,
    pub(super) mana_activation_condition: Option<crate::ConditionExpr>,
    pub(super) additional_activation_restrictions: Vec<String>,
    pub(super) has_exhaust_once_restriction: bool,
    pub(super) mana_usage_restrictions: Vec<crate::ability::ManaUsageRestriction>,
    pub(super) inline_effects_ast: Vec<EffectAst>,
}

fn parse_activate_only_sentence_details_lexed(
    tokens: &[OwnedLexToken],
    current_timing: &ActivationTiming,
) -> Option<ActivateOnlySentenceDetails> {
    if !is_activate_only_restriction_sentence_lexed(tokens) {
        return None;
    }

    let timing = parse_activate_only_timing_lexed(tokens).unwrap_or_else(|| current_timing.clone());
    Some(ActivateOnlySentenceDetails {
        timing: timing.clone(),
        condition: parse_activation_condition_lexed(tokens),
        normalized_restriction: normalize_activate_only_restriction(tokens, &timing),
    })
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

pub(crate) fn parse_activate_only_timing_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivationTiming> {
    ability_grammar::parse_activate_only_timing_lexed(tokens)
}

pub(crate) fn normalize_activate_only_restriction(
    tokens: &[OwnedLexToken],
    timing: &ActivationTiming,
) -> Option<String> {
    if timing != &ActivationTiming::OncePerTurn {
        return Some(crate::runtime_backend::token_word_refs(tokens).join(" "));
    }
    match activated_line_grammar::parse_once_per_turn_restriction_normalization_tokens(tokens) {
        OncePerTurnRestrictionNormalization::Redundant => None,
        OncePerTurnRestrictionNormalization::Residual(restriction) => Some(restriction),
    }
}

pub(crate) fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_activate_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    ability_grammar::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    ability_grammar::parse_mana_spend_bonus_sentence_lexed(tokens)
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_any_player_may_activate_sentence_lexed(tokens)
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    ability_grammar::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    ability_grammar::parse_triggered_times_each_turn_lexed(tokens)
}

pub(crate) fn parse_activation_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    ability_grammar::parse_activation_condition_lexed(tokens)
}
