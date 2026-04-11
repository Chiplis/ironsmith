use super::super::activation_helpers::parse_subtype_flexible;
use super::super::effect_sentences::parse_subtype_word;
use super::super::grammar::filters::parse_spell_filter_with_grammar_entrypoint;
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::keyword_static::parse_cost_modifier_mana_cost;
use super::super::lexer::OwnedLexToken;
use super::super::token_primitives::{
    find_index, slice_contains, slice_starts_with, str_strip_suffix,
};
use super::super::util::{parse_card_type, parse_counter_type_from_tokens, parse_number_word_u32};
use super::{joined_activation_clause_text, merge_mana_activation_conditions, parse_named_number};
use crate::ability::ActivationTiming;
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::effect::Value;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

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

const ACTIVATE_ONLY_RESTRICTION_PREFIXES: &[&[&str]] =
    &[&["activate", "only"], &["activate", "no", "more", "than"]];
const SPEND_MANA_RESTRICTION_PREFIXES: &[&[&str]] = &[
    &["spend", "this", "mana", "only"],
    &["spend", "that", "mana", "only"],
];
const SPEND_MANA_CAST_PREFIXES: &[&[&str]] = &[
    &["spend", "this", "mana", "only", "to", "cast"],
    &["spend", "that", "mana", "only", "to", "cast"],
];
const IF_MANA_SPENT_TO_CAST_PREFIXES: &[&[&str]] = &[
    &["if", "this", "mana", "is", "spent", "to", "cast"],
    &["if", "that", "mana", "is", "spent", "to", "cast"],
];
const DURING_OPPONENTS_TURN_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "during", "an", "opponents", "turn"],
    &["activate", "only", "during", "opponents", "turn"],
];
const ACTIVATE_ONLY_INSTANT_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "as", "an", "instant"],
    &["activate", "only", "as", "instant"],
];
const ACTIVATE_ONLY_IF_THERE_PREFIXES: &[&[&str]] = &[
    &["activate", "only", "if", "there", "is"],
    &["activate", "only", "if", "there", "are"],
];
const THIS_ABILITY_COSTS_PREFIXES: &[&[&str]] = &[&["this", "ability", "costs"]];
const THE_NEXT_PREFIXES: &[&[&str]] = &[&["the", "next"]];
const ACTIVATE_ONLY_SORCERY_PREFIXES: &[&[&str]] = &[&["activate", "only", "as", "a", "sorcery"]];
const ACTIVATE_ONLY_ONCE_EACH_TURN_PREFIXES: &[&[&str]] =
    &[&["activate", "only", "once", "each", "turn"]];
const ACTIVATE_ONLY_DURING_COMBAT_PREFIXES: &[&[&str]] =
    &[&["activate", "only", "during", "combat"]];
const ACTIVATE_ONLY_DURING_YOUR_TURN_PREFIXES: &[&[&str]] =
    &[&["activate", "only", "during", "your", "turn"]];
const THIS_ABILITY_TRIGGERS_ONLY_PREFIXES: &[&[&str]] = &[
    &["this", "ability", "triggers", "only"],
    &["do", "this", "only"],
];

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

    let spell_idx = find_index(&clause_words, |word| *word == "spell")?;
    let costs_idx = find_index(&clause_words, |word| *word == "costs")?;
    let less_idx = find_index(&clause_words, |word| *word == "less")?;
    if clause_words.get(spell_idx + 1).copied() != Some("you")
        || clause_words.get(spell_idx + 2).copied() != Some("cast")
        || clause_words.get(spell_idx + 3).copied() != Some("this")
        || clause_words.get(spell_idx + 4).copied() != Some("turn")
        || clause_words.get(less_idx + 1).copied() != Some("to")
        || clause_words.get(less_idx + 2).copied() != Some("cast")
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

    Some(EffectAst::ReduceNextSpellCostThisTurn {
        player: PlayerAst::You,
        filter,
        reduction,
    })
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

    if grammar::words_match_any_prefix(tokens, IF_MANA_SPENT_TO_CAST_PREFIXES).is_some() {
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
    if grammar::words_match_any_prefix(tokens, ACTIVATE_ONLY_SORCERY_PREFIXES).is_some() {
        return Some(ActivationTiming::SorcerySpeed);
    }
    if grammar::words_match_any_prefix(tokens, ACTIVATE_ONLY_ONCE_EACH_TURN_PREFIXES).is_some()
        || grammar::words_find_phrase(tokens, &["once", "each", "turn"]).is_some()
    {
        return Some(ActivationTiming::OncePerTurn);
    }
    if grammar::words_match_any_prefix(tokens, ACTIVATE_ONLY_DURING_COMBAT_PREFIXES).is_some()
        || grammar::words_find_phrase(tokens, &["during", "combat"]).is_some()
    {
        return Some(ActivationTiming::DuringCombat);
    }
    if grammar::words_match_any_prefix(tokens, ACTIVATE_ONLY_DURING_YOUR_TURN_PREFIXES).is_some()
        || grammar::words_find_phrase(tokens, &["during", "your", "turn"]).is_some()
    {
        return Some(ActivationTiming::DuringYourTurn);
    }
    if grammar::words_match_any_prefix(tokens, DURING_OPPONENTS_TURN_PREFIXES).is_some()
        || grammar::words_find_phrase(tokens, &["during", "an", "opponents", "turn"]).is_some()
        || grammar::words_find_phrase(tokens, &["during", "opponents", "turn"]).is_some()
    {
        return Some(ActivationTiming::DuringOpponentsTurn);
    }
    None
}

pub(crate) fn normalize_activate_only_restriction(
    tokens: &[OwnedLexToken],
    timing: &ActivationTiming,
) -> Option<String> {
    if timing != &ActivationTiming::OncePerTurn {
        return Some(crate::cards::builders::compiler::token_word_refs(tokens).join(" "));
    }

    let mut words = crate::cards::builders::compiler::token_word_refs(tokens)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }
    if words == ["activate", "only", "once", "each", "turn"] {
        return None;
    }
    if words.len() >= 6 && words[0..6] == ["activate", "only", "once", "each", "turn", "and"] {
        words.drain(0..6);
    }
    let mut index = 0usize;
    while index + 5 <= words.len() {
        if words[index..index + 5] == ["and", "only", "once", "each", "turn"] {
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
    grammar::words_match_any_prefix(tokens, ACTIVATE_ONLY_RESTRICTION_PREFIXES).is_some()
}

pub(crate) fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_match_any_prefix(tokens, SPEND_MANA_RESTRICTION_PREFIXES).is_some()
}

pub(crate) fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    let words = TokenWordView::new(tokens);
    if grammar::words_match_any_prefix(tokens, SPEND_MANA_CAST_PREFIXES).is_none() {
        return None;
    }

    let mut spell_idx = None;
    for idx in 0..words.len() {
        if matches!(words.get(idx), Some("spell" | "spells")) {
            spell_idx = Some(idx);
            break;
        }
    }
    let spell_idx = spell_idx?;
    let spec_words = (6..spell_idx)
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if spec_words.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    if matches!(spec_words.first().copied(), Some("a" | "an")) {
        idx += 1;
    }

    let card_type = match spec_words.get(idx).copied()? {
        "artifact" => crate::types::CardType::Artifact,
        "battle" => crate::types::CardType::Battle,
        "creature" => crate::types::CardType::Creature,
        "enchantment" => crate::types::CardType::Enchantment,
        "instant" => crate::types::CardType::Instant,
        "land" => crate::types::CardType::Land,
        "planeswalker" => crate::types::CardType::Planeswalker,
        "sorcery" => crate::types::CardType::Sorcery,
        _ => return None,
    };
    idx += 1;

    if idx != spec_words.len() {
        return None;
    }

    let mut tail = ((spell_idx + 1)..words.len())
        .filter_map(|word_idx| words.get(word_idx))
        .collect::<Vec<_>>();
    let subtype_requirement = if slice_starts_with(&tail, &["of", "the", "chosen", "type"]) {
        tail.drain(0..4);
        Some(crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource)
    } else {
        None
    };

    let grant_uncounterable = tail == ["and", "that", "spell", "can't", "be", "countered"]
        || tail == ["and", "that", "spell", "cant", "be", "countered"];
    if !grant_uncounterable && !tail.is_empty() {
        return None;
    }

    Some(crate::ability::ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement,
        restrict_to_matching_spell: true,
        grant_uncounterable,
        enters_with_counters: vec![],
    })
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    if grammar::words_match_any_prefix(tokens, IF_MANA_SPENT_TO_CAST_PREFIXES).is_none() {
        return None;
    }

    let words = TokenWordView::new(tokens);
    let mut spell_idx = None;
    for idx in 0..words.len() {
        if matches!(words.get(idx), Some("spell" | "spells")) {
            spell_idx = Some(idx);
            break;
        }
    }
    let spell_idx = spell_idx?;

    let spec_words = (7..spell_idx)
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if spec_words.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    if matches!(spec_words.first().copied(), Some("a" | "an")) {
        idx += 1;
    }

    let card_type = parse_card_type(spec_words.get(idx).copied()?)?;
    idx += 1;
    if idx != spec_words.len() {
        return None;
    }

    let comma_idx = find_index(tokens, |token| token.is_comma())?;
    let clause_tokens = super::trim_commas(&tokens[comma_idx + 1..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let clause_word_view = TokenWordView::new(&clause_tokens);
    let clause_words = clause_word_view.to_word_refs();
    if clause_words.len() < 6 || clause_words.first().copied() != Some("that") {
        return None;
    }
    if !matches!(
        clause_words.get(1).copied(),
        Some("creature" | "spell" | "permanent" | "card")
    ) && parse_card_type(clause_words.get(1).copied()?).is_none()
    {
        return None;
    }

    let enters_idx = clause_words
        .iter()
        .position(|word| matches!(*word, "enter" | "enters"))?;
    let with_token_idx = find_index(&clause_tokens, |token| token.is_word("with"))?;
    let after_with = &clause_tokens[with_token_idx + 1..];
    if after_with.is_empty() {
        return None;
    }

    let (count, used) = if after_with
        .first()
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
        && after_with
            .get(1)
            .is_some_and(|token| token.is_word("additional"))
    {
        (1, 2)
    } else if after_with
        .first()
        .is_some_and(|token| token.is_word("additional"))
    {
        (1, 1)
    } else if let Some(word) = after_with.first().and_then(OwnedLexToken::as_word) {
        let parsed = parse_number_word_u32(word)?;
        let used = if after_with
            .get(1)
            .is_some_and(|token| token.is_word("additional"))
        {
            2
        } else {
            1
        };
        (parsed, used)
    } else {
        return None;
    };

    let counter_type = parse_counter_type_from_tokens(&after_with[used..])?;
    let counter_idx = find_index(after_with, |token| {
        token.is_word("counter") || token.is_word("counters")
    })?;
    let tail_tokens = super::trim_commas(&after_with[counter_idx + 1..]);
    let mut tail: &[OwnedLexToken] = &tail_tokens;
    if tail.first().is_some_and(|token| token.is_word("on")) {
        tail = &tail[1..];
    }
    if tail.first().is_some_and(|token| token.is_word("it")) {
        tail = &tail[1..];
    } else if tail.first().is_some_and(|token| token.is_word("that")) {
        tail = &tail[1..];
        if tail
            .first()
            .is_some_and(|token| token.as_word().and_then(parse_card_type).is_some())
            || tail.first().is_some_and(|token| {
                matches!(
                    token.as_word(),
                    Some("creature" | "spell" | "permanent" | "card")
                )
            })
        {
            tail = &tail[1..];
        }
    }
    if tail.iter().any(|token| token.as_word().is_some()) {
        return None;
    }

    if enters_idx <= 1 {
        return None;
    }

    Some(crate::ability::ManaUsageRestriction::CastSpell {
        card_types: vec![card_type],
        subtype_requirement: None,
        restrict_to_matching_spell: false,
        grant_uncounterable: false,
        enters_with_counters: vec![(counter_type, count)],
    })
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.len() == 6
        && grammar::words_match_prefix(
            tokens,
            &["any", "player", "may", "activate", "this", "ability"],
        )
        .is_some()
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_match_any_prefix(tokens, THIS_ABILITY_TRIGGERS_ONLY_PREFIXES).is_some()
}

pub(crate) fn parse_triggered_times_each_turn_sentence(
    sentences: &[Vec<OwnedLexToken>],
) -> Option<u32> {
    sentences
        .iter()
        .find_map(|sentence| parse_triggered_times_each_turn_lexed(sentence))
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    let (count_idx, prefix_len) =
        if slice_starts_with(words, &["this", "ability", "triggers", "only"]) {
            (4usize, 4usize)
        } else if slice_starts_with(words, &["do", "this", "only"]) {
            (3usize, 3usize)
        } else {
            return None;
        };

    if words.len() < prefix_len + 3 {
        return None;
    }

    let mut index = count_idx;
    let count = match words.get(index) {
        Some(word) if *word == "once" => Some(1),
        Some(word) if *word == "twice" => Some(2),
        Some(word) => parse_named_number(word),
        None => None,
    }?;
    index += 1;

    if words.get(index) == Some(&"time") || words.get(index) == Some(&"times") {
        index += 1;
    }

    if words.get(index) == Some(&"each") && words.get(index + 1) == Some(&"turn") {
        Some(count)
    } else {
        None
    }
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    let words = TokenWordView::new(tokens);
    parse_triggered_times_each_turn_from_words(&words.to_word_refs())
}

pub(crate) fn parse_activation_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let words = TokenWordView::new(tokens);
    if words.len() < 5 {
        return None;
    }

    if grammar::words_match_any_prefix(tokens, &[&["activate", "no", "more", "than"]]).is_some() {
        let count_word = words.get(4)?;
        let count = match count_word {
            "once" => 1,
            "twice" => 2,
            other => parse_named_number(other)?,
        };
        let mut index = 5usize;
        if matches!(words.get(index), Some("time" | "times")) {
            index += 1;
        }
        if words.get(index) == Some("each") && words.get(index + 1) == Some("turn") {
            return Some(crate::ConditionExpr::MaxActivationsPerTurn(count));
        }
    }

    let after_activate_only = (2..words.len())
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if let Some(count) = parse_activation_count_per_turn(&after_activate_only) {
        return Some(crate::ConditionExpr::MaxActivationsPerTurn(count));
    }
    if grammar::words_match_any_prefix(tokens, ACTIVATE_ONLY_INSTANT_PREFIXES).is_some() {
        return Some(crate::ConditionExpr::ActivationTiming(
            ActivationTiming::AnyTime,
        ));
    }
    if grammar::words_match_any_prefix(tokens, ACTIVATE_ONLY_IF_THERE_PREFIXES).is_some() {
        let descriptor_start = 5usize;
        let mut in_idx = None;
        for idx in descriptor_start..words.len() {
            if words.get(idx) == Some("in") {
                in_idx = Some(idx);
                break;
            }
        }
        let in_idx = in_idx?;
        let zone_tail = (in_idx..words.len())
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        let points_to_your_graveyard = zone_tail == ["in", "your", "graveyard"]
            || zone_tail == ["in", "graveyard"]
            || zone_tail == ["in", "the", "graveyard"];
        if !points_to_your_graveyard {
            return None;
        }

        let descriptor_words = (descriptor_start..in_idx)
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        if descriptor_words.is_empty() {
            return None;
        }

        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        for word in descriptor_words {
            if let Some(card_type) = parse_card_type(word)
                && !slice_contains(&card_types, &card_type)
            {
                card_types.push(card_type);
            }
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
                && !slice_contains(&subtypes, &subtype)
            {
                subtypes.push(subtype);
            }
        }

        if card_types.is_empty() && subtypes.is_empty() {
            return None;
        }

        return Some(crate::ConditionExpr::CardInYourGraveyard {
            card_types,
            subtypes,
        });
    }
    if grammar::words_match_prefix(
        tokens,
        &[
            "activate",
            "only",
            "if",
            "creatures",
            "you",
            "control",
            "have",
            "total",
            "power",
        ],
    )
    .is_some()
    {
        let threshold_word = words.get(9)?;
        let threshold = parse_number_word_u32(threshold_word)?;
        let tail = (10..words.len())
            .filter_map(|idx| words.get(idx))
            .collect::<Vec<_>>();
        if tail == ["or", "greater"] {
            return Some(crate::ConditionExpr::ControlCreaturesTotalPowerAtLeast(
                threshold,
            ));
        }
        return None;
    }
    if grammar::words_match_prefix(tokens, &["activate", "only", "if", "you", "control"]).is_none()
    {
        return None;
    }

    let control_tail = (5..words.len())
        .filter_map(|idx| words.get(idx))
        .collect::<Vec<_>>();
    if slice_starts_with(&control_tail, &["a", "creature", "with", "power"])
        || slice_starts_with(&control_tail, &["creature", "with", "power"])
    {
        let power_idx = find_index(&control_tail, |word| *word == "power")?;
        let threshold = parse_number_word_u32(control_tail.get(power_idx + 1)?)?;
        let tail = &control_tail[power_idx + 2..];
        if tail == ["or", "greater"] {
            return Some(crate::ConditionExpr::YouControl(
                ObjectFilter::creature().with_power(crate::filter::Comparison::GreaterThanOrEqual(
                    threshold as i32,
                )),
            ));
        }
        return None;
    }
    if let Some(count) = control_tail
        .first()
        .and_then(|word| parse_number_word_u32(word))
    {
        let tail = &control_tail[1..];
        if tail == ["or", "more", "artifact"] || tail == ["or", "more", "artifacts"] {
            let mut filter = ObjectFilter::artifact();
            filter.zone = Some(Zone::Battlefield);
            return Some(crate::ConditionExpr::PlayerControlsAtLeast {
                player: PlayerFilter::You,
                filter,
                count,
            });
        }
        if tail == ["or", "more", "land"] || tail == ["or", "more", "lands"] {
            let mut filter = ObjectFilter::default().with_type(crate::types::CardType::Land);
            filter.zone = Some(Zone::Battlefield);
            return Some(crate::ConditionExpr::PlayerControlsAtLeast {
                player: PlayerFilter::You,
                filter,
                count,
            });
        }
    }
    if control_tail == ["an", "artifact"]
        || control_tail == ["a", "artifact"]
        || control_tail == ["artifact"]
        || control_tail == ["artifacts"]
    {
        let mut filter = ObjectFilter::artifact();
        filter.zone = Some(Zone::Battlefield);
        return Some(crate::ConditionExpr::PlayerControlsAtLeast {
            player: PlayerFilter::You,
            filter,
            count: 1,
        });
    }

    let mut subtypes = Vec::new();
    for idx in 0..words.len() {
        let Some(word) = words.get(idx) else {
            continue;
        };
        if let Some(subtype) = parse_subtype_flexible(word)
            && !slice_contains(&subtypes, &subtype)
        {
            subtypes.push(subtype);
        }
    }

    if subtypes.is_empty() {
        return None;
    }

    let mut combined: Option<crate::ConditionExpr> = None;
    for subtype in subtypes {
        let next = crate::ConditionExpr::YouControl(
            ObjectFilter::default()
                .with_type(crate::types::CardType::Land)
                .with_subtype(subtype),
        );
        combined = Some(match combined {
            Some(existing) => crate::ConditionExpr::Or(Box::new(existing), Box::new(next)),
            None => next,
        });
    }

    combined
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    let count = parse_named_number(words.first()?)?;
    let mut index = 1usize;
    if words
        .get(index)
        .is_some_and(|word| *word == "time" || *word == "times")
    {
        index += 1;
    }
    if words.get(index) == Some(&"each") && words.get(index + 1) == Some(&"turn") {
        Some(count)
    } else {
        None
    }
}
