use super::super::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::super::grammar::abilities as ability_grammar;
use super::super::grammar::filters::spell_filters::parse_spell_filter_with_grammar_entrypoint;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::keyword_static::parse_cost_modifier_mana_cost;
use super::super::lexer::OwnedLexToken;
use super::super::token_primitives::find_index;
use super::{joined_activation_clause_text, merge_mana_activation_conditions};
use crate::ability::ActivationTiming;
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::effect::Value;

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

const THIS_ABILITY_COSTS_PREFIXES: &[&[&str]] = &[&["this", "ability", "costs"]];
const THE_NEXT_PREFIXES: &[&[&str]] = &[&["the", "next"]];
const SPELL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["spell"]);
const COSTS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["costs"]);
const LESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["less"]);
const NEXT_SPELL_YOU_CAST_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["spell", "you", "cast", "this", "turn"]);
const LESS_TO_CAST_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["less", "to", "cast"]);
const ACTIVATE_ONLY_ONCE_EACH_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["activate", "only", "once", "each", "turn"]);
const ACTIVATE_ONLY_ONCE_EACH_TURN_AND_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["activate", "only", "once", "each", "turn", "and"]);
const AND_ONLY_ONCE_EACH_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["and", "only", "once", "each", "turn"]);

pub(super) struct ActivatedSentenceScan<'a> {
    pub(super) kept_sentences: Vec<&'a [OwnedLexToken]>,
    pub(super) timing: ActivationTiming,
    pub(super) mana_activation_condition: Option<crate::ConditionExpr>,
    pub(super) additional_activation_restrictions: Vec<String>,
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
    let words = TokenWordView::new(tokens);
    let clause_words = words.to_word_refs();
    if grammar::words_match_prefix(tokens, &["the", "next"]).is_none() {
        return None;
    }

    let spell_idx = find_index(&clause_words, |word| SPELL_WORD_PATTERN.matches_word(word))?;
    let costs_idx = find_index(&clause_words, |word| COSTS_WORD_PATTERN.matches_word(word))?;
    let less_idx = find_index(&clause_words, |word| LESS_WORD_PATTERN.matches_word(word))?;
    if !NEXT_SPELL_YOU_CAST_THIS_TURN_TAIL_PATTERN.matches_words(&clause_words[spell_idx..])
        || !LESS_TO_CAST_PATTERN.matches_words(&clause_words[less_idx..])
        || costs_idx <= spell_idx
    {
        return None;
    }

    let filter_start = words.token_index_after_words(2).unwrap_or(spell_idx);
    let spell_token_idx = words.token_index_for_word_index(spell_idx)?;
    let costs_token_idx = words.token_index_for_word_index(costs_idx)?;
    let less_token_idx = words.token_index_for_word_index(less_idx)?;
    let spell_filter_tokens = super::trim_commas(&tokens[filter_start..spell_token_idx]).to_vec();
    let reduction_tokens =
        super::trim_commas(&tokens[costs_token_idx + 1..less_token_idx]).to_vec();
    let filter = parse_spell_filter_with_grammar_entrypoint(&spell_filter_tokens);
    let (reduction, consumed) = parse_cost_modifier_mana_cost(&reduction_tokens)?;
    if consumed != reduction_tokens.len() {
        return None;
    }

    Some(EffectAst::subject_verb_reduce_next_spell_cost_this_turn(
        PlayerAst::You,
        filter,
        reduction,
    ))
}

fn is_inline_activated_text_modifier_sentence(tokens: &[OwnedLexToken]) -> bool {
    if grammar::words_match_any_prefix(tokens, THIS_ABILITY_COSTS_PREFIXES).is_some()
        && grammar::words_find_phrase(tokens, &["less", "to", "activate"]).is_some()
    {
        return true;
    }

    grammar::words_match_any_prefix(tokens, THE_NEXT_PREFIXES).is_some()
        && grammar::words_find_phrase(tokens, &["spell"]).is_some()
        && grammar::words_find_phrase(tokens, &["costs"]).is_some()
        && grammar::words_find_phrase(tokens, &["less"]).is_some()
        && grammar::words_find_phrase(tokens, &["cast"]).is_some()
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
    let mut mana_usage_restrictions = Vec::new();
    let mut inline_effects_ast = Vec::new();
    let mut kept_sentences = Vec::new();

    for sentence in sentences {
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
        mana_usage_restrictions,
        inline_effects_ast,
    }
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

    let mut words = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }
    let word_refs = words.iter().map(String::as_str).collect::<Vec<_>>();
    if ACTIVATE_ONLY_ONCE_EACH_TURN_PATTERN.matches_words(&word_refs) {
        return None;
    }
    if ACTIVATE_ONLY_ONCE_EACH_TURN_AND_PREFIX_PATTERN.matches_words(&word_refs) {
        words.drain(0..6);
    }
    let mut index = 0usize;
    while index + 5 <= words.len() {
        let tail_refs = words[index..]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if AND_ONLY_ONCE_EACH_TURN_PATTERN.matches_words(&tail_refs) {
            words.drain(index..index + 5);
        } else {
            index += 1;
        }
    }
    if words.is_empty() {
        None
    } else {
        Some(words.join(" "))
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

pub(crate) fn parse_triggered_times_each_turn_sentence(
    sentences: &[Vec<OwnedLexToken>],
) -> Option<u32> {
    sentences
        .iter()
        .find_map(|sentence| parse_triggered_times_each_turn_lexed(sentence))
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    ability_grammar::parse_triggered_times_each_turn_from_words(words)
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    ability_grammar::parse_triggered_times_each_turn_lexed(tokens)
}

pub(crate) fn parse_activation_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    ability_grammar::parse_activation_condition_lexed(tokens)
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    ability_grammar::parse_activation_count_per_turn(words)
}
