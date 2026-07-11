use super::super::activation_and_restrictions::activated_line_core::parse_named_number;
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::{
    CounterSpellConditionalKind, ForEachPlayerKind, split_for_each_opponent_doesnt_clause_lexed,
    split_for_each_player_doesnt_clause_lexed, split_negated_who_this_way_filter_tokens_lexed,
};
use super::super::grammar::primitives as grammar;
use super::super::grammar::values as shared_values;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::token_primitives::{
    items_end_with, items_have, items_start_with, locate_index, locate_last_index,
    slice_strip_prefix,
};
use super::super::util::{
    is_article, is_permanent_type, is_source_reference_words, parse_card_type,
    parse_counter_type_word, parse_mana_symbol_word_flexible, parse_number,
    parse_subtype_word as parse_shared_subtype_word,
    parse_supertype_word as parse_shared_supertype_word, parse_target_phrase, parse_zone_word,
    span_from_tokens, trim_commas, words,
};
use super::super::value_helpers::parse_filter_comparison_tokens;
use super::{parse_effect_chain, parse_effect_chain_inner, parse_effect_chain_lexed};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExtraTurnAnchorAst, IT_TAG, IfResultPredicate, PlayerAst,
    PredicateAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

#[cfg(test)]
pub(crate) fn parse_conditional_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    super::super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed(
        tokens,
        parse_effect_chain_lexed,
    )
}

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    shared_values::parse_scryfall_mana_cost(raw)
}

pub(crate) fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    shared_values::parse_mana_symbol_group(raw)
}

pub(crate) fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    shared_values::parse_mana_symbol(part)
}

pub(crate) fn parse_type_line(
    raw: &str,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    shared_values::parse_type_line_with(
        raw,
        parse_supertype_word,
        |word| parse_card_type(&word.to_ascii_lowercase()),
        parse_subtype_word,
    )
}

pub(crate) fn parse_supertype_word(word: &str) -> Option<Supertype> {
    parse_shared_supertype_word(word)
}

pub(crate) fn parse_subtype_word(word: &str) -> Option<Subtype> {
    parse_shared_subtype_word(word)
}

pub(crate) fn parse_for_each_opponent_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, true)? {
        return Ok(Some(effect));
    }
    let Some(split) = split_for_each_opponent_doesnt_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each opponent who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(split.effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(split.inner_tokens)?;
    Ok(Some(EffectAst::ForEachOpponentDoesNot {
        effects,
        predicate,
    }))
}

pub(crate) fn parse_for_each_player_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, false)? {
        return Ok(Some(effect));
    }
    let Some(split) = split_for_each_player_doesnt_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each player who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(split.effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(split.inner_tokens)?;
    Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }))
}

pub(crate) fn parse_for_each_doesnt_control_lose_game(
    tokens: &[OwnedLexToken],
    opponent: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = effect_grammar::parse_for_each_no_control_lose_game_tokens(tokens) else {
        return Ok(None);
    };
    let expected_kind = if opponent {
        ForEachPlayerKind::Opponent
    } else {
        ForEachPlayerKind::Player
    };
    if shape.player_kind != expected_kind {
        return Ok(None);
    }
    let filter_tokens = trim_commas(shape.filter_tokens);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false)?;

    let effect = EffectAst::Conditional {
        predicate: PredicateAst::PlayerControlsNo {
            player: PlayerAst::That,
            filter,
        },
        if_true: vec![EffectAst::subject_verb_lose_game(PlayerAst::That)],
        if_false: Vec::new(),
    };

    Ok(Some(if opponent {
        EffectAst::ForEachOpponent {
            effects: vec![effect],
        }
    } else {
        EffectAst::ForEachPlayer {
            effects: vec![effect],
        }
    }))
}

pub(crate) fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    effect_grammar::negated_action_word_index(words)
}

fn parse_negated_who_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(filter_tokens) = split_negated_who_this_way_filter_tokens_lexed(inner_tokens) else {
        return Ok(None);
    };

    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
    }))
}

pub(crate) fn parse_sentence_counter_target_spell_if_it_was_kicked(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_counter_spell_conditional_tokens(tokens) else {
        return Ok(None);
    };
    if shape.kind != CounterSpellConditionalKind::IfKicked {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(shape.target_tokens));
    let counter = EffectAst::subject_verb_counter(target);
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetWasKicked,
        if_true: vec![counter],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_sentence_counter_target_spell_thats_second_cast_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_counter_spell_conditional_tokens(tokens) else {
        return Ok(None);
    };
    if shape.kind != CounterSpellConditionalKind::SecondCastThisTurn {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(shape.target_tokens));
    let counter = EffectAst::subject_verb_counter(target);
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetSpellCastOrderThisTurn(2),
        if_true: vec![counter],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_sentence_exile_target_creature_with_greatest_power(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_exile_greatest_power_creature_tokens(tokens) else {
        return Ok(None);
    };

    let target_tokens = trim_commas(shape.target_tokens);
    let target = parse_target_phrase(&target_tokens)?;
    let exile = EffectAst::subject_verb_exile(target.clone(), false);
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetHasGreatestPowerAmongCreatures,
        if_true: vec![exile],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}
