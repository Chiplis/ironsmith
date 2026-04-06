use super::super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed;
use super::super::grammar::primitives::{
    self as grammar, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_period,
};
use super::super::keyword_static::parse_where_x_value_clause;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index, find_window_by, find_window_index, iter_contains, lexed_head_words, rfind_index,
    slice_contains, slice_ends_with, slice_starts_with, split_lexed_once_on_comma_then,
    str_strip_suffix,
};
use super::super::util::{
    is_article, is_source_reference_words, mana_pips_from_token, parse_card_type, parse_color,
    parse_counter_type_from_tokens, token_index_for_word_index, words,
};
use super::super::util::{parse_target_phrase, parse_value, span_from_tokens};
use super::sentence_helpers::*;
use super::verb_handlers::parse_half_rounded_down_draw_count_words;
use super::zone_counter_helpers::parse_convert;
#[allow(unused_imports)]
use super::{
    bind_implicit_player_context, parse_after_turn_sentence, parse_become_clause,
    parse_cant_effect_sentence, parse_delayed_until_next_end_step_sentence,
    parse_delayed_when_that_dies_this_turn_sentence, parse_destroy_or_exile_all_split_sentence,
    parse_each_player_choose_and_sacrifice_rest,
    parse_each_player_put_permanent_cards_exiled_with_source_sentence, parse_earthbend_sentence,
    parse_effect_chain, parse_effect_chain_inner, parse_effect_chain_lexed, parse_effect_clause,
    parse_effect_sentence_lexed, parse_enchant_sentence,
    parse_exile_hand_and_graveyard_bundle_sentence, parse_exile_instead_of_graveyard_sentence,
    parse_exile_then_return_same_object_sentence, parse_exile_up_to_one_each_target_type_sentence,
    parse_for_each_counter_removed_sentence, parse_for_each_destroyed_this_way_sentence,
    parse_for_each_exiled_this_way_sentence, parse_for_each_opponent_doesnt,
    parse_for_each_player_doesnt, parse_for_each_vote_clause, parse_gain_ability_sentence,
    parse_gain_ability_to_source_sentence, parse_gain_life_equal_to_age_sentence,
    parse_gain_life_equal_to_power_sentence, parse_gain_x_plus_life_sentence,
    parse_look_at_hand_sentence, parse_look_at_top_then_exile_one_sentence, parse_mana_symbol,
    parse_monstrosity_sentence, parse_play_from_graveyard_sentence, parse_prevent_damage_sentence,
    parse_same_name_gets_fanout_sentence, parse_same_name_target_fanout_sentence,
    parse_search_library_sentence, parse_sentence_counter_target_spell_if_it_was_kicked,
    parse_sentence_counter_target_spell_thats_second_cast_this_turn,
    parse_sentence_delayed_trigger_this_turn,
    parse_sentence_exile_target_creature_with_greatest_power,
    parse_shared_color_target_fanout_sentence, parse_shuffle_graveyard_into_library_sentence,
    parse_shuffle_object_into_library_sentence, parse_subtype_word, parse_take_extra_turn_sentence,
    parse_target_player_exiles_creature_and_graveyard_sentence, parse_vote_extra_sentence,
    parse_vote_start_sentence, parse_you_and_each_opponent_voted_with_you_sentence, trim_commas,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, TagKey, TargetAst, TextSpan,
};
#[allow(unused_imports)]
use crate::effect::{ChoiceCount, Until, Value};
use crate::mana::ManaSymbol;
#[allow(unused_imports)]
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
#[allow(unused_imports)]
use crate::types::{CardType, Subtype};
#[allow(unused_imports)]
use crate::zone::Zone;
use std::cell::OnceCell;
use std::sync::LazyLock;
use winnow::Parser as _;

pub(crate) type SentencePrimitiveParser =
    fn(&[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError>;

type SentencePrimitiveNormalizedWords = TokenWordView;

pub(crate) struct SentencePrimitive {
    pub(crate) name: &'static str,
    pub(crate) parser: SentencePrimitiveParser,
}

pub(crate) struct SentencePrimitiveIndex {
    by_head: std::collections::HashMap<&'static str, Vec<usize>>,
    by_head_pair: std::collections::HashMap<(&'static str, &'static str), Vec<usize>>,
}

#[derive(Clone, Copy)]
enum SentencePrimitiveHeadHint {
    Single(&'static str),
    Pair(&'static str, &'static str),
}

fn find_token_word(tokens: &[OwnedLexToken], word: &str) -> Option<usize> {
    find_index(tokens, |token| token.is_word(word))
}

fn rfind_token_word(tokens: &[OwnedLexToken], word: &str) -> Option<usize> {
    rfind_index(tokens, |token| token.is_word(word))
}

fn find_comma_then_idx(tokens: &[OwnedLexToken]) -> Option<usize> {
    split_lexed_once_on_comma_then(tokens).map(|(head, _)| head.len())
}

fn contains_word_window(words: &[&str], pattern: &[&str]) -> bool {
    find_window_index(words, pattern).is_some()
}

fn strip_quoted_possessive_suffix(word: &str) -> &str {
    str_strip_suffix(word, "'s")
        .or_else(|| str_strip_suffix(word, "’s"))
        .or_else(|| str_strip_suffix(word, "s'"))
        .or_else(|| str_strip_suffix(word, "s’"))
        .unwrap_or(word)
}

fn parse_pluralized_subtype_word(word: &str) -> Option<Subtype> {
    parse_subtype_word(word).or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
}

fn sentence_primitive_head_hints(name: &'static str) -> Vec<SentencePrimitiveHeadHint> {
    if name == "transform-with-followup" {
        return vec![
            SentencePrimitiveHeadHint::Single("transform"),
            SentencePrimitiveHeadHint::Single("convert"),
        ];
    }
    if name == "shared-color-target-fanout" {
        return vec![
            SentencePrimitiveHeadHint::Single("target"),
            SentencePrimitiveHeadHint::Pair("target", "radiance"),
        ];
    }
    if matches!(name, "for-each-player-doesnt" | "for-each-opponent-doesnt") {
        return vec![
            SentencePrimitiveHeadHint::Single("for"),
            SentencePrimitiveHeadHint::Single("then"),
            SentencePrimitiveHeadHint::Single("each"),
            SentencePrimitiveHeadHint::Pair("for", "each"),
            SentencePrimitiveHeadHint::Pair("then", "each"),
        ];
    }

    let parts = name.split('-').collect::<Vec<_>>();
    let Some(first) = parts.first().copied() else {
        return Vec::new();
    };
    let supports_single = matches!(
        first,
        "if" | "you"
            | "target"
            | "each"
            | "for"
            | "return"
            | "destroy"
            | "exile"
            | "counter"
            | "draw"
            | "put"
            | "gets"
            | "sacrifice"
            | "take"
            | "earthbend"
            | "enchant"
            | "cant"
            | "prevent"
            | "gain"
            | "search"
            | "shuffle"
            | "look"
            | "play"
            | "vote"
            | "after"
            | "reveal"
            | "damage"
            | "unless"
            | "monstrosity"
            | "choose"
    );
    let mut hints = Vec::new();
    if supports_single {
        hints.push(SentencePrimitiveHeadHint::Single(first));
    }
    if parts.len() >= 2 && supports_single {
        hints.push(SentencePrimitiveHeadHint::Pair(first, parts[1]));
    }
    hints
}

fn build_sentence_primitive_index(
    primitives: &'static [SentencePrimitive],
) -> SentencePrimitiveIndex {
    let mut by_head = std::collections::HashMap::<&'static str, Vec<usize>>::new();
    let mut by_head_pair =
        std::collections::HashMap::<(&'static str, &'static str), Vec<usize>>::new();
    for (idx, primitive) in primitives.iter().enumerate() {
        for head in sentence_primitive_head_hints(primitive.name) {
            match head {
                SentencePrimitiveHeadHint::Single(word) => {
                    by_head.entry(word).or_default().push(idx);
                }
                SentencePrimitiveHeadHint::Pair(first, second) => {
                    by_head_pair.entry((first, second)).or_default().push(idx);
                }
            }
        }
    }
    SentencePrimitiveIndex {
        by_head,
        by_head_pair,
    }
}

fn run_sentence_primitive(
    primitive: &SentencePrimitive,
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    match (primitive.parser)(tokens) {
        Ok(Some(effects)) => {
            let stage = format!("parse_effect_sentence:primitive-hit:{}", primitive.name);
            parser_trace(&stage, tokens);
            if effects.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "primitive '{}' produced empty effects (clause: '{}')",
                    primitive.name,
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            }
            Ok(Some(effects))
        }
        Ok(None) => Ok(None),
        Err(err) => {
            if parser_trace_enabled() {
                eprintln!(
                    "[parser-flow] stage=parse_effect_sentence:primitive-error primitive={} clause='{}' error={err:?}",
                    primitive.name,
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                );
            }
            Err(err)
        }
    }
}

fn normalize_parser_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    for token in &mut normalized {
        match token.kind {
            crate::cards::builders::parser::lexer::TokenKind::Word
            | crate::cards::builders::parser::lexer::TokenKind::Number
            | crate::cards::builders::parser::lexer::TokenKind::Tilde => {
                let replacement = token.parser_text().to_string();
                let _ = token.replace_word(replacement);
            }
            _ => {}
        }
    }
    normalized
}

fn run_sentence_primitive_lexed(
    primitive: &SentencePrimitive,
    tokens: &[OwnedLexToken],
    lowered: &OnceCell<Vec<OwnedLexToken>>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let lowered_tokens = lowered.get_or_init(|| normalize_parser_tokens(tokens));
    run_sentence_primitive(primitive, lowered_tokens)
}

pub(crate) fn run_sentence_primitives_lexed(
    tokens: &[OwnedLexToken],
    primitives: &'static [SentencePrimitive],
    index: &SentencePrimitiveIndex,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head, second) = lexed_head_words(tokens).unwrap_or(("", None));
    let mut tried = vec![false; primitives.len()];
    let lowered = OnceCell::new();
    if let Some(second) = second
        && let Some(candidate_indices) = index.by_head_pair.get(&(head, second))
    {
        for &idx in candidate_indices {
            tried[idx] = true;
            if let Some(effects) = run_sentence_primitive_lexed(&primitives[idx], tokens, &lowered)?
            {
                return Ok(Some(effects));
            }
        }
    }

    if let Some(candidate_indices) = index.by_head.get(head) {
        for &idx in candidate_indices {
            tried[idx] = true;
            if let Some(effects) = run_sentence_primitive_lexed(&primitives[idx], tokens, &lowered)?
            {
                return Ok(Some(effects));
            }
        }
    }

    for (idx, primitive) in primitives.iter().enumerate() {
        if tried[idx] {
            continue;
        }
        if let Some(effects) = run_sentence_primitive_lexed(primitive, tokens, &lowered)? {
            return Ok(Some(effects));
        }
    }

    Ok(None)
}

pub(crate) fn parse_sentence_return_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_return_with_counters_on_it(tokens)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_put_onto_battlefield_with_counters_on_it(tokens)
}

pub(crate) fn parse_sentence_exile_source_with_counters_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_exile_source_with_counters(tokens)
}

pub(crate) fn parse_you_and_target_player_each_draw_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if grammar::words_match_prefix(tokens, &["you", "and", "target"]).is_none() {
        return Ok(None);
    }

    let target_player = match clause_words.get(3).copied() {
        Some("opponent" | "opponents") => PlayerAst::TargetOpponent,
        Some("player" | "players") => PlayerAst::Target,
        _ => return Ok(None),
    };

    let mut idx = 4usize;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let remainder_words = &clause_words[idx..];
    if remainder_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some((count, used_words)) = parse_half_rounded_down_draw_count_words(remainder_words) {
        let trailing_words = &remainder_words[used_words..];
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing shared draw clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(vec![
            EffectAst::Draw {
                count: count.clone(),
                player: PlayerAst::You,
            },
            EffectAst::Draw {
                count,
                player: target_player,
            },
        ]));
    }
    let synthetic_tokens = remainder_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_value(&synthetic_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if synthetic_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words =
        crate::cards::builders::parser::token_word_refs(&synthetic_tokens[used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count,
            player: target_player,
        },
    ]))
}

pub(crate) fn parse_sentence_you_and_target_player_each_draw(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_target_player_each_draw_sentence(tokens)
}

pub(crate) fn parse_sentence_choose_player_to_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let mut stripped = trim_commas(tokens);
    while stripped
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        stripped.remove(0);
    }
    if stripped.is_empty() {
        return Ok(None);
    }

    let Some((choose_slice, tail_slice)) =
        grammar::split_lexed_once_on_separator(&stripped, || grammar::kw("to").void())
    else {
        return Ok(None);
    };

    let choose_tokens = trim_commas(choose_slice);
    let tail_tokens = trim_commas(tail_slice);
    if choose_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }
    let Some((chooser, filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(&choose_tokens)?
    else {
        return Ok(None);
    };

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    for effect in &mut tail_effects {
        bind_implicit_player_context(effect, PlayerAst::That);
    }

    let mut effects = vec![EffectAst::ChoosePlayer {
        chooser,
        filter,
        tag: TagKey::from(IT_TAG),
        random,
        exclude_previous_choices,
    }];
    effects.extend(tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_damage_to_that_player_half_damage_of_those_spells(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut stripped = trim_commas(tokens);
    while stripped
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        stripped.remove(0);
    }
    if stripped.is_empty() {
        return Ok(None);
    }

    use super::super::grammar::primitives as grammar;

    let deal_split =
        grammar::split_lexed_once_on_separator(&stripped, || grammar::kw("deal").void()).or_else(
            || grammar::split_lexed_once_on_separator(&stripped, || grammar::kw("deals").void()),
        );
    let Some((_before_deal, after_deal)) = deal_split else {
        return Ok(None);
    };
    let tail_words = crate::cards::builders::parser::token_word_refs(after_deal);
    if tail_words.len() != 20 {
        return Ok(None);
    }
    if tail_words[..14]
        != [
            "damage", "to", "that", "player", "equal", "to", "half", "the", "damage", "dealt",
            "by", "one", "of", "those",
        ]
        || tail_words[15..] != ["spells", "this", "turn", "rounded", "down"]
    {
        return Ok(None);
    }

    let card_type = parse_card_type(tail_words[14]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported spell type in historical half-damage sentence (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;
    Ok(Some(vec![
        EffectAst::ChooseSpellCastHistory {
            chooser: PlayerAst::You,
            cast_by: PlayerAst::That,
            filter: ObjectFilter::default().with_type(card_type),
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::DealDamage {
            amount: Value::HalfRoundedDown(Box::new(Value::DamageDealtThisTurnByTaggedSpellCast(
                TagKey::from(IT_TAG),
            ))),
            target: TargetAst::Player(PlayerFilter::target_player(), None),
        },
    ]))
}

pub(crate) fn parse_draw_for_each_card_exiled_from_hand_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }

    let clause_words = crate::cards::builders::parser::token_word_refs(&clause_tokens);
    let (player, mut effects) = match clause_words.as_slice() {
        [
            "that",
            "player",
            "shuffles",
            "then",
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "their",
            "hand",
            "this",
            "way",
        ] => (
            PlayerAst::That,
            vec![EffectAst::ShuffleLibrary {
                player: PlayerAst::That,
            }],
        ),
        [
            "that",
            "player",
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "their",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::That, Vec::new()),
        [
            "you",
            "draw",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "your",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::You, Vec::new()),
        [
            "draw",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "your",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::Implicit, Vec::new()),
        [
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "their",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::That, Vec::new()),
        [
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "your",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::Implicit, Vec::new()),
        _ => return Ok(None),
    };

    effects.push(EffectAst::DrawForEachTaggedMatching {
        player,
        tag: TagKey::from(IT_TAG),
        filter: ObjectFilter::default().in_zone(Zone::Hand),
    });
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_draw_for_each_card_exiled_from_hand_this_way(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_for_each_card_exiled_from_hand_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_you_and_attacking_player_each_draw_and_lose(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.len() < 11 || grammar::words_match_prefix(tokens, &["you", "and"]).is_none() {
        return Ok(None);
    }

    let mut idx = 2usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"attacking") || clause_words.get(idx + 1) != Some(&"player") {
        return Ok(None);
    }
    idx += 2;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let draw_tokens = clause_words[idx..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let draw_words = crate::cards::builders::parser::token_word_refs(&draw_tokens);
    let (draw_count, after_draw_words) = if let Some((draw_count, used_words)) =
        parse_half_rounded_down_draw_count_words(&draw_words)
    {
        (draw_count, draw_words[used_words..].to_vec())
    } else {
        let (draw_count, draw_used) = parse_value(&draw_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing shared draw count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if draw_tokens
            .get(draw_used)
            .and_then(OwnedLexToken::as_word)
            .is_none_or(|word| word != "card" && word != "cards")
        {
            return Err(CardTextError::ParseError(format!(
                "missing card keyword in shared draw/lose sentence (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        (
            draw_count,
            crate::cards::builders::parser::token_word_refs(&draw_tokens[draw_used + 1..]),
        )
    };
    if after_draw_words.first() != Some(&"and")
        || !matches!(after_draw_words.get(1).copied(), Some("lose" | "loses"))
    {
        return Ok(None);
    }

    let lose_tokens = after_draw_words[2..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (lose_amount, lose_used) = parse_value(&lose_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared life-loss amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if lose_tokens
        .get(lose_used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "life")
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in shared draw/lose sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words =
        crate::cards::builders::parser::token_word_refs(&lose_tokens[lose_used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw/lose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: draw_count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count: draw_count,
            player: PlayerAst::Attacking,
        },
        EffectAst::LoseLife {
            amount: lose_amount.clone(),
            player: PlayerAst::You,
        },
        EffectAst::LoseLife {
            amount: lose_amount,
            player: PlayerAst::Attacking,
        },
    ]))
}

pub(crate) fn parse_sentence_sacrifice_it_next_end_step(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    // "sacrifice <object> at the beginning of [the] next end step"
    let Some(object_tokens) = grammar::strip_lexed_prefix_phrase(tokens, &["sacrifice"]) else {
        return Ok(None);
    };
    let Some((object_tokens, _timing)) =
        grammar::split_lexed_once_on_separator(object_tokens, || {
            winnow::combinator::alt((
                grammar::phrase(&["at", "the", "beginning", "of", "the", "next", "end", "step"]),
                grammar::phrase(&["at", "the", "beginning", "of", "next", "end", "step"]),
            ))
        })
    else {
        return Ok(None);
    };

    let object_tokens = trim_commas(object_tokens);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in delayed next-end-step clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let object_words = crate::cards::builders::parser::token_word_refs(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["the", "creature"]
            | ["that", "creature"]
            | ["the", "permanent"]
            | ["that", "permanent"]
            | ["the", "token"]
            | ["that", "token"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: PlayerFilter::Any,
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
        }],
    }]))
}

pub(crate) fn parse_sentence_if_tagged_cards_remain_exiled(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let has_prefix = grammar::strip_lexed_prefix_phrase(
        tokens,
        &["if", "any", "of", "those", "cards", "remain", "exiled"],
    )
    .is_some()
        || grammar::strip_lexed_prefix_phrase(
            tokens,
            &["if", "those", "cards", "remain", "exiled"],
        )
        .is_some()
        || grammar::strip_lexed_prefix_phrase(tokens, &["if", "that", "card", "remains", "exiled"])
            .is_some()
        || grammar::strip_lexed_prefix_phrase(tokens, &["if", "it", "remains", "exiled"]).is_some();
    if !has_prefix {
        return Ok(None);
    }

    parse_conditional_sentence_with_grammar_entrypoint_lexed(tokens, parse_effect_chain_lexed)
        .map(Some)
}

pub(crate) fn parse_sentence_sacrifice_at_end_of_combat(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    // "sacrifice <object> at [the] end of combat"
    let Some(object_tokens) = grammar::strip_lexed_prefix_phrase(tokens, &["sacrifice"]) else {
        return Ok(None);
    };
    let Some((object_tokens, _timing)) =
        grammar::split_lexed_once_on_separator(object_tokens, || {
            winnow::combinator::alt((
                grammar::phrase(&["at", "end", "of", "combat"]),
                grammar::phrase(&["at", "the", "end", "of", "combat"]),
            ))
        })
    else {
        return Ok(None);
    };

    let object_tokens = trim_commas(object_tokens);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in end-of-combat clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let object_words = crate::cards::builders::parser::token_word_refs(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["that", "token"]
            | ["this", "token"]
            | ["that", "permanent"]
            | ["this", "permanent"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
        }],
    }]))
}

pub(crate) fn parse_sentence_each_player_choose_and_sacrifice_rest(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_each_player_choose_and_sacrifice_rest(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_exile_instead_of_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_exile_instead_of_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_monstrosity(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_monstrosity_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_counter_removed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_counter_removed_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_counter_kind_put_or_remove(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    // "for each kind of counter on <target>, put another counter of that kind on it or remove one from it"
    let Some(after_prefix) =
        grammar::strip_lexed_prefix_phrase(tokens, &["for", "each", "kind", "of", "counter", "on"])
    else {
        return Ok(None);
    };
    let Some((target_tokens, tail_tokens)) =
        grammar::split_lexed_once_on_delimiter(after_prefix, super::super::lexer::TokenKind::Comma)
    else {
        return Ok(None);
    };

    let target_tokens = trim_commas(target_tokens);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(&target_tokens)?;

    if !grammar::contains_phrase(
        tail_tokens,
        &[
            "put", "another", "counter", "of", "that", "kind", "on", "it", "or", "remove", "one",
            "from",
        ],
    ) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::ForEachCounterKindPutOrRemove {
        target,
    }]))
}

pub(crate) fn parse_put_counter_ladder_segments(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_comma(tokens);
    if segments.len() != 3 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = trim_commas(segment).to_vec();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }
        if clause.is_empty() {
            return Ok(None);
        }

        let Some(on_idx) = find_index(&clause, |token| token.is_word("on")) else {
            return Ok(None);
        };
        let descriptor = trim_commas(&clause[..on_idx]);
        let target_tokens = trim_commas(&clause[on_idx + 1..]);
        if descriptor.is_empty() || target_tokens.is_empty() {
            return Ok(None);
        }

        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        let target = parse_target_phrase(&target_tokens)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target,
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_put_counter_sequence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("put")) {
        return Ok(None);
    }
    if !tokens
        .iter()
        .any(|token| token.is_word("counter") || token.is_word("counters"))
    {
        return Ok(None);
    }

    if let Some(effects) = parse_put_counter_ladder_segments(tokens)? {
        return Ok(Some(effects));
    }

    if let Some(on_idx) = find_index(tokens, |token| token.is_word("on")) {
        let descriptor_tokens = trim_commas(&tokens[1..on_idx]);
        let target_tokens = trim_commas(&tokens[on_idx + 1..]);
        if !descriptor_tokens.is_empty() && !target_tokens.is_empty() {
            let mut descriptors: Vec<Vec<OwnedLexToken>> = Vec::new();
            let comma_segments = split_lexed_slices_on_comma(&descriptor_tokens);
            if comma_segments.len() >= 2 {
                for segment in comma_segments {
                    let mut clause = trim_commas(segment);
                    if clause.first().is_some_and(|token| token.is_word("and")) {
                        clause.remove(0);
                    }
                    if clause.is_empty() {
                        descriptors.clear();
                        break;
                    }
                    descriptors.push(clause);
                }
            } else if let Some(and_idx) =
                find_index(&descriptor_tokens, |token| token.is_word("and"))
            {
                let first = trim_commas(&descriptor_tokens[..and_idx]);
                let second = trim_commas(&descriptor_tokens[and_idx + 1..]);
                if !first.is_empty() && !second.is_empty() {
                    descriptors.push(first);
                    descriptors.push(second);
                }
            }

            if descriptors.len() >= 2 {
                let target = parse_target_phrase(&target_tokens)?;
                let mut effects = Vec::new();
                for descriptor in descriptors {
                    let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
                    effects.push(EffectAst::PutCounters {
                        counter_type,
                        count: Value::Fixed(count as i32),
                        target: target.clone(),
                        target_count: None,
                        distributed: false,
                    });
                }
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... counter on X and it gains ... until end of turn."
    if let Some(and_idx) = find_window_by(tokens, 2, |window| {
        window[0].is_word("and") && window[1].is_word("it")
    }) {
        let first_clause = trim_commas(&tokens[1..and_idx]);
        let second_clause = trim_commas(&tokens[and_idx + 1..]);
        if !first_clause.is_empty()
            && !second_clause.is_empty()
            && second_clause.iter().any(|token| {
                token.is_word("gain")
                    || token.is_word("gains")
                    || token.is_word("has")
                    || token.is_word("have")
            })
            && let Ok(first) = parse_put_counters(&first_clause)
            && let Some(mut gain_effects) = parse_gain_ability_sentence(&second_clause)?
        {
            let source_target = match &first {
                EffectAst::PutCounters { target, .. } => Some(target.clone()),
                EffectAst::Conditional { if_true, .. }
                    if if_true.len() == 1
                        && matches!(if_true.first(), Some(EffectAst::PutCounters { .. })) =>
                {
                    if let Some(EffectAst::PutCounters { target, .. }) = if_true.first() {
                        Some(target.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(source_target) = source_target {
                for effect in &mut gain_effects {
                    match effect {
                        EffectAst::Pump { target, .. }
                        | EffectAst::GrantAbilitiesToTarget { target, .. }
                        | EffectAst::GrantToTarget { target, .. }
                        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                            if let TargetAst::Tagged(tag, _) = target
                                && tag.as_str() == IT_TAG
                            {
                                *target = source_target.clone();
                            }
                        }
                        _ => {}
                    }
                }

                let mut effects = vec![first];
                effects.append(&mut gain_effects);
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... and ... counter on ..." without comma separation.
    if let Some(and_idx) = find_index(tokens, |token| token.is_word("and")) {
        let first_clause = trim_commas(&tokens[1..and_idx]);
        let second_clause = trim_commas(&tokens[and_idx + 1..]);
        if !first_clause.is_empty() && !second_clause.is_empty() {
            if let (Ok(first), Ok(second)) = (
                parse_put_counters(&first_clause),
                parse_put_counters(&second_clause),
            ) {
                return Ok(Some(vec![first, second]));
            }
        }
    }

    let segments = split_lexed_slices_on_comma(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = segment.to_vec();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }

        if clause.is_empty() {
            return Ok(None);
        }

        if !grammar::contains_word(&clause, "counter")
            && !grammar::contains_word(&clause, "counters")
        {
            return Ok(None);
        }

        let Ok(effect) = parse_put_counters(&clause) else {
            return Ok(None);
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

pub(crate) fn is_pump_like_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::Pump { .. }
            | EffectAst::PumpByLastEffect { .. }
            | EffectAst::SetBasePowerToughness { .. }
            | EffectAst::SetBasePower { .. }
    )
}

pub(crate) fn parse_gets_then_fights_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let body_tokens = grammar::strip_lexed_prefix_phrase(tokens, &["then"]).unwrap_or(tokens);
    if body_tokens.is_empty() {
        return Ok(None);
    }

    // Split on "fight"/"fights"
    let fight_split =
        grammar::split_lexed_once_on_separator(body_tokens, || grammar::kw("fight").void())
            .or_else(|| {
                grammar::split_lexed_once_on_separator(body_tokens, || grammar::kw("fights").void())
            });
    let Some((left_slice, right_slice)) = fight_split else {
        return Ok(None);
    };

    let mut left_tokens = trim_commas(left_slice).to_vec();
    while left_tokens.last().is_some_and(|token| token.is_word("and")) {
        left_tokens.pop();
    }
    let left_tokens = trim_commas(&left_tokens);
    let right_tokens = trim_commas(right_slice);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return Ok(None);
    }

    // Split left side on "get"/"gets" to extract subject
    let get_split =
        grammar::split_lexed_once_on_separator(&left_tokens, || grammar::kw("get").void()).or_else(
            || grammar::split_lexed_once_on_separator(&left_tokens, || grammar::kw("gets").void()),
        );
    let Some((subject_slice, _modifier_slice)) = get_split else {
        return Ok(None);
    };

    let pump_effect = parse_effect_clause(&left_tokens)?;
    if !is_pump_like_effect(&pump_effect) {
        return Ok(None);
    }

    let subject_tokens = trim_commas(subject_slice);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let creature1 = parse_target_phrase(&subject_tokens)?;
    let creature2 = parse_target_phrase(&right_tokens)?;
    if matches!(
        creature1,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) || matches!(
        creature2,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "fight target must be a creature (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![
        pump_effect,
        EffectAst::Fight {
            creature1,
            creature2,
        },
    ]))
}

pub(crate) fn parse_sentence_gets_then_fights(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gets_then_fights_sentence(tokens)
}

pub(crate) fn parse_return_with_counters_on_it_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter(|word| !is_article(word))
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }

    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before destination (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let destination_tokens = trim_commas(&tokens[to_idx + 1..]);
    if destination_tokens.is_empty() {
        return Ok(None);
    }
    if !grammar::contains_word(&destination_tokens, "battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = find_token_word(&destination_tokens, "with") else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_tokens.len() {
        return Ok(None);
    }

    let base_destination_word_storage =
        crate::cards::builders::parser::token_word_refs(&destination_tokens[..with_idx]);
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    let Some(battlefield_idx) = find_index(&base_destination_words, |word| *word == "battlefield")
    else {
        return Ok(None);
    };
    let tapped = slice_contains(&base_destination_words, &"tapped");
    let destination_tail: Vec<&str> = base_destination_words[battlefield_idx + 1..]
        .iter()
        .copied()
        .filter(|word| *word != "tapped")
        .collect();
    let battlefield_controller = if destination_tail.is_empty()
        || destination_tail == ["under", "its", "control"]
        || destination_tail == ["under", "their", "control"]
    {
        ReturnControllerAst::Preserve
    } else if destination_tail == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail == ["under", "its", "owner", "control"]
        || destination_tail == ["under", "their", "owner", "control"]
        || destination_tail == ["under", "that", "player", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    let counter_clause_tokens = trim_commas(&destination_tokens[with_idx + 1..]);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::parser::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let mut descriptors = Vec::new();
    for descriptor in split_lexed_slices_on_and(&descriptor_tokens) {
        let descriptor = trim_commas(&descriptor);
        if descriptor.is_empty() {
            continue;
        }
        descriptors.push(descriptor);
    }
    if descriptors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let mut effects = vec![EffectAst::ReturnToBattlefield {
        target: parse_target_phrase(&target_tokens)?,
        tapped,
        transformed: false,
        converted: false,
        controller: battlefield_controller,
    }];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: tagged_target.clone(),
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_put_onto_battlefield_with_counters_on_it_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter(|word| !is_article(word))
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    if !tokens
        .first()
        .is_some_and(|token| token.is_word("put") || token.is_word("puts"))
    {
        return Ok(None);
    }

    let Some(onto_idx) = find_index(tokens, |token| token.is_word("onto")) else {
        return Ok(None);
    };
    if onto_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..onto_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing put target before destination (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let destination_tokens = trim_commas(&tokens[onto_idx + 1..]);
    if destination_tokens.is_empty() {
        return Ok(None);
    }
    if !grammar::contains_word(&destination_tokens, "battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = find_token_word(&destination_tokens, "with") else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_tokens.len() {
        return Ok(None);
    }

    let base_destination_word_storage =
        crate::cards::builders::parser::token_word_refs(&destination_tokens[..with_idx]);
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    if base_destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }

    let destination_tail = &base_destination_words[1..];
    let supported_control_tail = destination_tail.is_empty()
        || destination_tail == ["under", "your", "control"]
        || destination_tail == ["under", "its", "owner", "control"]
        || destination_tail == ["under", "their", "owner", "control"]
        || destination_tail == ["under", "that", "player", "control"];
    if !supported_control_tail {
        return Ok(None);
    }
    let battlefield_controller = if destination_tail == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail == ["under", "its", "owner", "control"]
        || destination_tail == ["under", "their", "owner", "control"]
        || destination_tail == ["under", "that", "player", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };

    let counter_clause_tokens = trim_commas(&destination_tokens[with_idx + 1..]);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::parser::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() || !grammar::contains_word(&descriptor_tokens, "counter") {
        return Ok(None);
    }

    let mut descriptors = Vec::new();
    for descriptor in split_lexed_slices_on_and(&descriptor_tokens) {
        let descriptor = trim_commas(&descriptor);
        if descriptor.is_empty() {
            continue;
        }
        descriptors.push(descriptor);
    }
    if descriptors.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::MoveToZone {
        target: parse_target_phrase(&target_tokens)?,
        zone: Zone::Battlefield,
        to_top: false,
        battlefield_controller,
        battlefield_tapped: false,
        attached_to: None,
    }];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: tagged_target.clone(),
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_with_counters_on_it(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(tokens)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_put_onto_battlefield_with_counters_on_it_sentence(tokens)
}

pub(crate) fn replace_target_subtype(target: &mut TargetAst, subtype: Subtype) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.subtypes = vec![subtype];
            true
        }
        TargetAst::WithCount(inner, _) => replace_target_subtype(inner, subtype),
        _ => false,
    }
}

pub(crate) fn clone_return_effect_with_subtype(
    base: &EffectAst,
    subtype: Subtype,
) -> Option<EffectAst> {
    match base {
        EffectAst::ReturnToHand { target, random } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(EffectAst::ReturnToHand {
                target: cloned_target,
                random: *random,
            })
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            transformed,
            converted,
            controller,
        } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(
                EffectAst::ReturnToBattlefield {
                    target: cloned_target,
                    tapped: *tapped,
                    transformed: *transformed,
                    converted: *converted,
                    controller: *controller,
                },
            )
        }
        EffectAst::ReturnAllToHand { filter } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToHand {
                filter: cloned_filter,
            })
        }
        EffectAst::ReturnAllToBattlefield { filter, tapped } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToBattlefield {
                filter: cloned_filter,
                tapped: *tapped,
            })
        }
        _ => None,
    }
}

pub(crate) fn parse_draw_then_connive_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !tail_tokens
        .iter()
        .any(|token| token.is_word("connive") || token.is_word("connives"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(&tail_tokens)? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_draw_then_connive(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(tokens)
}

pub(crate) fn parse_if_enters_with_additional_counter_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use super::super::lexer::TokenKind;

    // "if <predicate>, it enters with <counter descriptor> on it"
    let Some(after_if) = grammar::strip_lexed_prefix_phrase(tokens, &["if"]) else {
        return Ok(None);
    };
    let Some((predicate_slice, followup_slice)) =
        grammar::split_lexed_once_on_delimiter(after_if, TokenKind::Comma)
    else {
        return Ok(None);
    };

    let predicate_tokens = trim_commas(predicate_slice);
    let predicate_words: Vec<&str> =
        crate::cards::builders::parser::token_word_refs(&predicate_tokens)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
    let predicate_is_supported = predicate_words.as_slice()
        == ["creature", "enters", "this", "way"]
        || predicate_words.as_slice() == ["it", "enters", "as", "creature"];
    if !predicate_is_supported {
        return Ok(None);
    }

    let followup_tokens = trim_commas(followup_slice);
    let Some(counter_clause_slice) =
        grammar::strip_lexed_prefix_phrase(&followup_tokens, &["it", "enters", "with"])
    else {
        return Ok(None);
    };

    let counter_clause_tokens = trim_commas(counter_clause_slice);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::parser::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() || !grammar::contains_word(&descriptor_tokens, "additional") {
        return Ok(None);
    }

    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;
    let put_counter = EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    };
    let apply_only_if_creature = EffectAst::Conditional {
        predicate: PredicateAst::ItMatches(ObjectFilter::creature()),
        if_true: vec![put_counter],
        if_false: Vec::new(),
    };

    Ok(Some(vec![EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![apply_only_if_creature],
    }]))
}

pub(crate) fn parse_each_player_return_with_additional_counter_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let inner_start_word_idx = if grammar::words_match_prefix(tokens, &["for", "each", "player"])
        .is_some()
        || grammar::words_match_prefix(tokens, &["for", "each", "players"]).is_some()
    {
        3
    } else if grammar::words_match_prefix(tokens, &["each", "player"]).is_some()
        || grammar::words_match_prefix(tokens, &["each", "players"]).is_some()
    {
        2
    } else {
        return Ok(None);
    };

    let Some(inner_start_token_idx) = token_index_for_word_index(tokens, inner_start_word_idx)
    else {
        return Ok(None);
    };
    let inner_tokens = trim_commas(&tokens[inner_start_token_idx..]);
    if inner_tokens.is_empty() {
        return Ok(None);
    }
    if !inner_tokens
        .first()
        .is_some_and(|token| token.is_word("return") || token.is_word("returns"))
    {
        return Ok(None);
    }

    let Some(with_idx) = rfind_index(&inner_tokens, |token| token.is_word("with")) else {
        return Ok(None);
    };
    if with_idx + 1 >= inner_tokens.len() {
        return Ok(None);
    }

    let return_clause_tokens = trim_commas(&inner_tokens[..with_idx]);
    if return_clause_tokens.is_empty() {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(&inner_tokens[with_idx + 1..]);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::parser::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() || !grammar::contains_word(&descriptor_tokens, "additional") {
        return Ok(None);
    }

    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;
    let mut per_player_effects = parse_effect_chain_inner(&return_clause_tokens)?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ReturnToBattlefield { .. } | EffectAst::ReturnAllToBattlefield { .. }
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}

pub(crate) fn parse_sentence_each_player_return_with_additional_counter(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(tokens)
}

pub(crate) fn parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_comma(tokens);
    if segments.len() != 3 {
        return Ok(None);
    }

    let reveal_tokens = trim_commas(&segments[0]);
    let reveal_words = crate::cards::builders::parser::token_word_refs(&reveal_tokens);
    let reveal_prefix = [
        "each", "player", "reveals", "a", "number", "of", "cards", "from", "the", "top", "of",
        "their", "library", "equal", "to",
    ];
    if !slice_starts_with(&reveal_words, &reveal_prefix) {
        return Ok(None);
    }
    let Some(count_token_idx) = token_index_for_word_index(&reveal_tokens, reveal_prefix.len())
    else {
        return Ok(None);
    };
    let mut synthetic_where_tokens = vec![
        OwnedLexToken::word("where".to_string(), TextSpan::synthetic()),
        OwnedLexToken::word("x".to_string(), TextSpan::synthetic()),
        OwnedLexToken::word("is".to_string(), TextSpan::synthetic()),
    ];
    synthetic_where_tokens.extend(reveal_tokens[count_token_idx..].iter().cloned());
    let Some(count) = parse_where_x_value_clause(&synthetic_where_tokens) else {
        return Ok(None);
    };

    let put_tokens = trim_commas(&segments[1]);
    if grammar::words_match_prefix(&put_tokens, &["puts", "all", "permanent", "cards"]).is_none()
        || grammar::words_find_phrase(&put_tokens, &["revealed", "this", "way"]).is_none()
        || grammar::words_find_phrase(&put_tokens, &["onto", "the", "battlefield"]).is_none()
    {
        return Ok(None);
    }

    let rest_tokens = trim_commas(&segments[2]);
    let rest_words = crate::cards::builders::parser::token_word_refs(&rest_tokens);
    let rest_words = if rest_words.first().copied() == Some("and") {
        &rest_words[1..]
    } else {
        rest_words.as_slice()
    };
    if rest_words != ["puts", "the", "rest", "into", "their", "graveyard"] {
        return Ok(None);
    }

    let revealed_tag_key = helper_tag_for_tokens(tokens, "revealed");
    let iterated_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::That,
                count,
                tag: revealed_tag_key.clone(),
            },
            EffectAst::RevealTagged {
                tag: revealed_tag_key.clone(),
            },
            EffectAst::ForEachTagged {
                tag: revealed_tag_key,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(ObjectFilter::permanent_card()),
                    if_true: vec![EffectAst::MoveToZone {
                        target: iterated_target.clone(),
                        zone: Zone::Battlefield,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Owner,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                    if_false: vec![EffectAst::MoveToZone {
                        target: iterated_target,
                        zone: Zone::Graveyard,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
            },
        ],
    }]))
}

pub(crate) fn parse_return_then_do_same_for_subtypes_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = crate::cards::builders::parser::token_word_refs(&tail_tokens);
    if grammar::words_match_prefix(&tail_tokens, &["do", "the", "same", "for"]).is_none() {
        return Ok(None);
    }
    let subtype_words = &tail_words[4..];
    if subtype_words.is_empty() {
        return Ok(None);
    }

    let mut extra_subtypes = Vec::new();
    for word in subtype_words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        extra_subtypes.push(subtype);
    }
    if extra_subtypes.is_empty() {
        return Ok(None);
    }

    let mut effects = parse_effect_chain(&head_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in extra_subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_then_do_same_for_subtypes(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(tokens)
}

pub(crate) fn parse_sacrifice_any_number_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let (head_tokens, tail_tokens) = if let Some((head, tail)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
    {
        if head.is_empty() {
            return Ok(None);
        }
        (trim_commas(head), Some(trim_commas(tail)))
    } else {
        (tokens.to_vec(), None)
    };

    if !head_tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    if !(head_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("any"))
        && head_tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number")))
    {
        return Ok(None);
    }
    idx += 2;
    if head_tokens
        .get(idx)
        .is_some_and(|token| token.is_word("of"))
    {
        idx += 1;
    }
    if idx >= head_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let filter_tokens = trim_commas(&head_tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let tag = TagKey::from(IT_TAG);

    let mut effects = vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(tag),
            player: PlayerAst::Implicit,
        },
    ];
    if let Some(tail_tokens) = tail_tokens
        && !tail_tokens.is_empty()
    {
        let mut tail_effects = parse_effect_chain(&tail_tokens)?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_sacrifice_any_number(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(tokens)
}

pub(crate) fn parse_sacrifice_one_or_more_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    let Some((minimum, used)) = parse_number(&tokens[idx..]) else {
        return Ok(None);
    };
    idx += used;
    if !(tokens.get(idx).is_some_and(|token| token.is_word("or"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("more")))
    {
        return Ok(None);
    }
    idx += 2;
    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let filter_tokens = trim_commas(&tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice one or more' (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let filter = parse_object_filter(&filter_tokens, false)?;
    let tag = TagKey::from(IT_TAG);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::at_least(minimum as usize),
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(tag),
            player: PlayerAst::Implicit,
        },
    ]))
}

pub(crate) fn parse_sentence_sacrifice_one_or_more(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_one_or_more_sentence(tokens)
}

pub(crate) fn parse_sentence_keyword_then_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let Some((head_slice, tail_slice)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
    else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let Some(head_effect) = parse_keyword_mechanic_clause(&head_tokens)? else {
        return Ok(None);
    };

    let tail_tokens = trim_commas(tail_slice);
    if tail_tokens.is_empty() {
        return Ok(Some(vec![head_effect]));
    }

    let mut effects = vec![head_effect];
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(&tail_tokens)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_chain_then_keyword(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = find_comma_then_idx(tokens)
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            find_token_word(tokens, "then")
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..head_end]);
    let tail_tokens = trim_commas(&tokens[tail_start..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let Some(keyword_effect) = parse_keyword_mechanic_clause(&tail_tokens)? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_return_then_create(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let split = split_lexed_once_on_comma_then(tokens)
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let tail_tokens = trim_commas(tail_slice);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !head_tokens.first().is_some_and(|t| t.is_word("return"))
        || !tail_tokens.first().is_some_and(|t| t.is_word("create"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_exile_then_may_put_from_exile(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let split = split_lexed_once_on_comma_then(tokens)
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((head_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(head_slice);
    let tail_tokens = trim_commas(tail_slice);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !grammar::strip_lexed_prefix_phrase(
        &tail_tokens,
        &["you", "may", "put", "any", "number", "of"],
    )
    .is_some()
        || !grammar::contains_word(&tail_tokens, "from")
        || !grammar::contains_word(&tail_tokens, "exile")
        || !grammar::contains_word(&tail_tokens, "battlefield")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_exile_source_with_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    // "exile <source> with <counter descriptor> on it/them"
    let Some(after_exile) = grammar::strip_lexed_prefix_phrase(tokens, &["exile"]) else {
        return Ok(None);
    };
    let Some((source_name_slice, counter_clause_slice)) =
        grammar::split_lexed_once_on_separator(after_exile, || grammar::kw("with").void())
    else {
        return Ok(None);
    };

    let source_name_tokens = trim_commas(source_name_slice);
    if source_name_tokens.is_empty() {
        return Ok(None);
    }
    let source_name_words = crate::cards::builders::parser::token_word_refs(&source_name_tokens);
    if !is_likely_named_or_source_reference_words(&source_name_words) {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(counter_clause_slice);
    let Some(on_idx) = rfind_token_word(&counter_clause_tokens, "on") else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words =
        crate::cards::builders::parser::token_word_refs(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() {
        return Ok(None);
    }
    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;

    let source_target = TargetAst::Source(span_from_tokens(tokens));
    Ok(Some(vec![
        EffectAst::Exile {
            target: source_target.clone(),
            face_down: false,
        },
        EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: source_target,
            target_count: None,
            distributed: false,
        },
    ]))
}

pub(crate) fn parse_sentence_exile_source_with_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_source_with_counters_sentence(tokens)
}

pub(crate) fn parse_sentence_comma_then_chain_special(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        words
            .iter()
            .filter_map(|word| match *word {
                "s" | "'" | "’" => None,
                _ => Some(strip_quoted_possessive_suffix(word)),
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let head_word_storage = crate::cards::builders::parser::token_word_refs(&head_tokens);
    let tail_word_storage = crate::cards::builders::parser::token_word_refs(&tail_tokens);
    let head_words = normalize_words(&head_word_storage);
    let tail_words = normalize_words(&tail_word_storage);
    let is_that_player_tail = slice_starts_with(&tail_words, &["that", "player"]);
    let is_return_source_tail = slice_starts_with(&tail_words, &["return", "this"])
        && slice_contains(&tail_words, &"owner")
        && slice_contains(&tail_words, &"hand");
    let is_put_source_on_top_of_library_tail = slice_starts_with(&tail_words, &["put", "this"])
        && slice_contains(&tail_words, &"top")
        && slice_contains(&tail_words, &"owner")
        && tail_words.last().copied() == Some("library");
    let is_choose_card_name_tail =
        (slice_starts_with(&tail_words, &["choose", "any", "card", "name"])
            || slice_starts_with(&tail_words, &["choose", "a", "card", "name"]))
            && head_words.first().copied() == Some("look");
    if !is_that_player_tail
        && !is_return_source_tail
        && !is_put_source_on_top_of_library_tail
        && !is_choose_card_name_tail
    {
        return Ok(None);
    }
    if is_return_source_tail
        && !head_words
            .first()
            .is_some_and(|word| matches!(*word, "tap" | "untap"))
    {
        return Ok(None);
    }
    if is_put_source_on_top_of_library_tail
        && !head_words.first().is_some_and(|word| *word == "draw")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub(crate) fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = find_comma_then_idx(tokens) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = crate::cards::builders::parser::token_word_refs(&tail_tokens);
    let suffix = [
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ];
    let Some(suffix_start) = find_window_index(&tail_words, &suffix) else {
        return Ok(None);
    };
    if suffix_start == 0 || !matches!(tail_words[suffix_start - 1], "deal" | "deals") {
        return Ok(None);
    }
    if suffix_start + suffix.len() != tail_words.len() {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if !head_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Destroy { .. }))
    {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(IT_TAG);
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::DealDamage {
        amount: Value::Count(count_filter),
        target: TargetAst::Player(
            PlayerFilter::ControllerOf(tagged_ref),
            span_from_tokens(&tail_tokens),
        ),
    });
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_destroy_all_attached_to_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    // "destroy all/each <filter> attached to <target>"
    if !tokens.first().is_some_and(|token| token.is_word("destroy")) {
        return Ok(None);
    }
    if !tokens
        .get(1)
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        return Ok(None);
    }

    let Some((filter_slice, target_slice)) =
        grammar::split_lexed_once_on_separator(&tokens[2..], || {
            grammar::phrase(&["attached", "to"]).void()
        })
    else {
        return Ok(None);
    };

    let mut filter_tokens = trim_commas(filter_slice).to_vec();
    while filter_tokens
        .last()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
    {
        filter_tokens.pop();
    }
    let target_tokens = trim_commas(target_slice);
    let has_timing_tail = target_tokens.iter().any(|token| {
        token.as_word().is_some_and(|w| {
            matches!(
                w,
                "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
            )
        })
    });
    let supported_target = target_tokens.first().is_some_and(|t| t.is_word("target"))
        || grammar::contains_word(&target_tokens, "it") && target_tokens.len() == 1
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "creature"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "permanent"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "land"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "artifact"]).is_some()
        || grammar::strip_lexed_prefix_phrase(&target_tokens, &["that", "enchantment"]).is_some();
    if filter_tokens.is_empty() || target_tokens.is_empty() || !supported_target || has_timing_tail
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(vec![EffectAst::DestroyAllAttachedTo {
        filter,
        target,
    }]))
}

pub(crate) fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(tokens)
}

pub(crate) fn add_chosen_creature_type_constraint_to_target(target: &mut TargetAst) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.chosen_creature_type = true;
            true
        }
        TargetAst::WithCount(inner, _) => add_chosen_creature_type_constraint_to_target(inner),
        _ => false,
    }
}

pub(crate) fn find_creature_type_choice_phrase(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    for idx in 0..tokens.len() {
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("the"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 4).is_some_and(|token| token.is_word("of"))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 6)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 7));
        }
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("of"))
            && tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 6));
        }
    }
    None
}

pub(crate) fn find_color_choice_phrase(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    for idx in 0..tokens.len() {
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("the"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("color"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("of"))
            && (tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("your"))
                || tokens
                    .get(idx + 4)
                    .is_some_and(|token| token.is_word("their")))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 6));
        }
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("color"))
            && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
            && (tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("your"))
                || tokens
                    .get(idx + 3)
                    .is_some_and(|token| token.is_word("their")))
            && tokens
                .get(idx + 4)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 5));
        }
    }
    None
}

pub(crate) fn parse_sentence_destroy_creature_type_of_choice(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if grammar::strip_lexed_prefix_phrase(tokens, &["destroy", "all", "creatures"]).is_none() {
        return Ok(None);
    }
    if find_creature_type_choice_phrase(tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::ChooseCreatureType {
            player: PlayerAst::You,
            excluded_subtypes: vec![],
        },
        EffectAst::DestroyAll {
            filter: ObjectFilter::creature().of_chosen_creature_type(),
        },
    ]))
}

pub(crate) fn parse_sentence_pump_creature_type_of_choice(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(get_idx) = find_index(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    }) else {
        return Ok(None);
    };
    if get_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..get_idx]);
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(&subject_tokens) else {
        return Ok(None);
    };
    let trailing_subject = trim_commas(&subject_tokens[choice_idx + consumed..]);
    if !trailing_subject.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let trimmed_subject_tokens = trim_commas(&subject_tokens[..choice_idx]).to_vec();
    if trimmed_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_tokens = trimmed_subject_tokens.clone();
    gain_candidate_tokens.extend_from_slice(&tokens[get_idx..]);
    if let Some(mut gain_effects) = parse_gain_ability_sentence(&gain_candidate_tokens)? {
        let mut patched = false;
        for effect in &mut gain_effects {
            match effect {
                EffectAst::PumpAll { filter, .. }
                | EffectAst::GrantAbilitiesAll { filter, .. }
                | EffectAst::GrantAbilitiesChoiceAll { filter, .. } => {
                    filter.chosen_creature_type = true;
                    patched = true;
                }
                _ => {}
            }
        }
        if patched {
            let mut effects = vec![EffectAst::ChooseCreatureType {
                player: PlayerAst::You,
                excluded_subtypes: vec![],
            }];
            effects.extend(gain_effects);
            return Ok(Some(effects));
        }
    }

    let mut filter_tokens = trimmed_subject_tokens;
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("all"))
    {
        filter_tokens.remove(0);
    }
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let modifier = tokens
        .get(get_idx + 1)
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;
    let (power, toughness, duration, condition) =
        parse_get_modifier_values_with_tail(&tokens[get_idx + 1..], base_power, base_toughness)?;
    if condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gets duration in creature-type choice pump clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::ChooseCreatureType {
            player: PlayerAst::You,
            excluded_subtypes: vec![],
        },
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        },
    ]))
}

pub(crate) fn parse_sentence_return_targets_of_creature_type_of_choice(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    if !grammar::contains_word(&tokens[to_idx + 1..], "hand")
        && !grammar::contains_word(&tokens[to_idx + 1..], "hands")
    {
        return Ok(None);
    }

    let target_tokens = &tokens[1..to_idx];
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(target_tokens) else {
        return Ok(None);
    };

    let trimmed_target = trim_commas(&target_tokens[..choice_idx]);
    if trimmed_target.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before creature-type choice phrase (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let trailing = trim_commas(&target_tokens[choice_idx + consumed..]);
    if !trailing.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing return target clause after creature-type choice phrase (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let mut target = parse_target_phrase(&trimmed_target)?;
    if !add_chosen_creature_type_constraint_to_target(&mut target) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice return target must be object-based (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::ChooseCreatureType {
            player: PlayerAst::You,
            excluded_subtypes: vec![],
        },
        EffectAst::ReturnToHand {
            target,
            random: false,
        },
    ]))
}

pub(crate) fn parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let starts_choose_all = grammar::words_match_prefix(tokens, &["choose", "all"]).is_some();
    let starts_put_all = grammar::words_match_prefix(tokens, &["put", "all"]).is_some();
    if !starts_choose_all && !starts_put_all {
        return Ok(None);
    }
    if !((grammar::contains_word(tokens, "battlefield")
        || grammar::contains_word(tokens, "command"))
        && grammar::contains_word(tokens, "graveyard")
        && grammar::contains_word(tokens, "hand"))
    {
        return Ok(None);
    }

    let Some(from_idx) = find_index(&clause_words, |word| *word == "from") else {
        return Ok(None);
    };
    let zone_pair = if contains_word_window(
        &clause_words[from_idx..],
        &[
            "from",
            "the",
            "battlefield",
            "and",
            "from",
            "your",
            "graveyard",
        ],
    ) {
        [Zone::Battlefield, Zone::Graveyard]
    } else if contains_word_window(
        &clause_words[from_idx..],
        &[
            "from",
            "the",
            "command",
            "zone",
            "and",
            "from",
            "your",
            "graveyard",
        ],
    ) {
        [Zone::Command, Zone::Graveyard]
    } else {
        return Ok(None);
    };
    if from_idx <= 2 {
        return Ok(None);
    }

    let Some(from_token_idx) = token_index_for_word_index(tokens, from_idx) else {
        return Ok(None);
    };

    let filter_tokens = trim_commas(&tokens[2..from_token_idx]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if starts_choose_all {
        let Some(put_idx) = find_index(&clause_words, |word| *word == "put") else {
            return Ok(None);
        };
        let Some(put_token_idx) = token_index_for_word_index(tokens, put_idx) else {
            return Ok(None);
        };
        if grammar::words_match_prefix(
            &tokens[put_token_idx..],
            &["put", "them", "into", "your", "hand"],
        )
        .is_none()
            && grammar::words_match_prefix(
                &tokens[put_token_idx..],
                &["put", "them", "in", "your", "hand"],
            )
            .is_none()
        {
            return Ok(None);
        }
    } else if grammar::words_match_suffix(tokens, &["into", "your", "hand"]).is_none()
        && grammar::words_match_suffix(tokens, &["in", "your", "hand"]).is_none()
    {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    base_filter.controller = None;

    let mut battlefield_filter = base_filter.clone();
    battlefield_filter.zone = Some(zone_pair[0]);

    let mut graveyard_filter = base_filter;
    graveyard_filter.zone = Some(zone_pair[1]);

    Ok(Some(vec![
        EffectAst::ReturnAllToHand {
            filter: battlefield_filter,
        },
        EffectAst::ReturnAllToHand {
            filter: graveyard_filter,
        },
    ]))
}

pub(crate) fn return_segment_mentions_zone(tokens: &[OwnedLexToken]) -> bool {
    grammar::contains_word(tokens, "graveyard")
        || grammar::contains_word(tokens, "graveyards")
        || grammar::contains_word(tokens, "battlefield")
        || grammar::contains_word(tokens, "hand")
        || grammar::contains_word(tokens, "hands")
        || grammar::contains_word(tokens, "library")
        || grammar::contains_word(tokens, "libraries")
        || grammar::contains_word(tokens, "exile")
}

pub(crate) fn parse_sentence_return_multiple_targets(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = rfind_index(tokens, |token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let dest_tokens = &tokens[to_idx + 1..];
    let is_hand =
        grammar::contains_word(dest_tokens, "hand") || grammar::contains_word(dest_tokens, "hands");
    let is_battlefield = grammar::contains_word(dest_tokens, "battlefield");
    let tapped = grammar::contains_word(dest_tokens, "tapped");
    if !is_hand && !is_battlefield {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    let has_multi_separator = target_tokens.iter().any(|token| {
        token.is_word("and") || token.is_comma() || token.is_word("or") || token.is_word("and/or")
    });
    if !has_multi_separator {
        return Ok(None);
    }

    let mut segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    for and_segment in split_lexed_slices_on_and(&target_tokens) {
        for comma_segment in split_lexed_slices_on_comma(and_segment) {
            let trimmed = trim_commas(&comma_segment);
            if !trimmed.is_empty() {
                let trimmed_words = crate::cards::builders::parser::token_word_refs(&trimmed);
                let starts_new_target = trimmed_words.first().is_some_and(|word| {
                    matches!(
                        *word,
                        "target"
                            | "up"
                            | "another"
                            | "other"
                            | "this"
                            | "that"
                            | "it"
                            | "them"
                            | "all"
                            | "each"
                    )
                });
                let mentions_target = grammar::contains_word(&trimmed, "target");
                let starts_like_zone_suffix = trimmed_words
                    .first()
                    .is_some_and(|word| matches!(*word, "from" | "to" | "in" | "on" | "under"));
                if !segments.is_empty()
                    && !starts_new_target
                    && !mentions_target
                    && !starts_like_zone_suffix
                {
                    let last = segments.last_mut().expect("segments is non-empty");
                    last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                    last.extend(trimmed.to_vec());
                } else {
                    segments.push(trimmed.to_vec());
                }
            }
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(|segment| segment.first())
        .and_then(OwnedLexToken::as_word)
        .filter(|word| matches!(*word, "all" | "each"))
        .map(str::to_string);

    let shared_suffix = segments
        .last()
        .and_then(|segment| {
            find_index(segment, |token| token.is_word("from")).map(|idx| segment[idx..].to_vec())
        })
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        if !return_segment_mentions_zone(&segment) && !shared_suffix.is_empty() {
            segment.extend(shared_suffix.clone());
        }
        if let Some(quantifier) = shared_quantifier.as_deref() {
            let segment_words = crate::cards::builders::parser::token_word_refs(&segment);
            let has_explicit_quantifier =
                matches!(segment_words.first().copied(), Some("all" | "each"));
            let starts_like_target_reference = matches!(
                segment_words.first().copied(),
                Some("target" | "up" | "this" | "that" | "it" | "them" | "another")
            );
            if !has_explicit_quantifier
                && !starts_like_target_reference
                && !grammar::contains_word(&segment, "target")
            {
                segment.insert(
                    0,
                    OwnedLexToken::word(quantifier.to_string(), TextSpan::synthetic()),
                );
            }
        }
        let segment_words = crate::cards::builders::parser::token_word_refs(&segment);
        if matches!(segment_words.first().copied(), Some("all" | "each")) {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            }
            let filter = parse_object_filter(&segment[1..], false)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnAllToBattlefield { filter, tapped });
            } else {
                effects.push(EffectAst::ReturnAllToHand { filter });
            }
        } else {
            let target = parse_target_phrase(&segment)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnToBattlefield {
                    target,
                    tapped,
                    transformed: false,
                    converted: false,
                    controller: ReturnControllerAst::Preserve,
                });
            } else {
                effects.push(EffectAst::ReturnToHand {
                    target,
                    random: false,
                });
            }
        }
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_for_each_of_target_objects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if grammar::strip_lexed_prefix_phrase(tokens, &["for", "each"]).is_none()
        && !tokens.first().is_some_and(|t| t.is_word("each"))
    {
        return Ok(None);
    }

    let Some((subject_slice, effect_slice)) =
        grammar::split_lexed_once_on_delimiter(tokens, super::super::lexer::TokenKind::Comma)
    else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(subject_slice);
    let Some((mut filter, count)) = parse_for_each_targeted_object_subject(&subject_tokens)? else {
        return Ok(None);
    };
    if filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.tagged_constraints.is_empty()
    {
        // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects
        // compilation for plain "target permanent(s)" clauses.
        filter.controller = Some(PlayerFilter::Any);
    }

    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let effect_tokens = trim_commas(effect_slice);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after for-each target subject (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut per_target_effects = parse_effect_chain(&effect_tokens)?;
    for effect in &mut per_target_effects {
        bind_implicit_player_context(effect, PlayerAst::You);
    }
    if per_target_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "for-each target follow-up produced no effects (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            player: PlayerAst::Implicit,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: per_target_effects,
        },
    ]))
}

pub(crate) fn parse_distribute_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.first().copied() != Some("distribute") {
        return Ok(None);
    }

    let (count, used) = parse_number(&tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed counter amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let rest = &tokens[1 + used..];
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let among_idx = find_index(rest, |token| token.is_word("among")).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed target clause after 'among' (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let target_tokens = trim_commas(&rest[among_idx + 1..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(&target_tokens)?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_phrase = &target_tokens[used_count..];
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target = parse_target_phrase(target_phrase)?;

    Ok(Some(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target,
        target_count: Some(target_count),
        distributed: true,
    }))
}

pub(crate) fn parse_sentence_distribute_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let (head_tokens, tail_tokens) = if let Some((head, tail)) =
        split_lexed_once_on_comma_then(tokens).or_else(|| {
            grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
        }) {
        (head.to_vec(), trim_commas(tail))
    } else {
        (tokens.to_vec(), Vec::new())
    };

    let Some(primary) = parse_distribute_counters_sentence(&head_tokens)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if !tail_tokens.is_empty() {
        effects.extend(parse_effect_chain(&tail_tokens)?);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_take_extra_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_take_extra_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_earthbend(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(earthbend) = parse_earthbend_sentence(tokens)? else {
        return Ok(None);
    };

    // Support chained text like "earthbend 8, then untap that land."
    let Some((_, used)) = parse_number(&tokens[1..]) else {
        return Ok(Some(vec![earthbend]));
    };
    let mut tail = trim_commas(&tokens[1 + used..]).to_vec();
    while tail.first().is_some_and(|token| token.is_word("then")) {
        tail.remove(0);
    }
    if tail.is_empty() {
        return Ok(Some(vec![earthbend]));
    }

    let mut effects = vec![earthbend];
    effects.extend(parse_effect_chain(&tail)?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_transform_with_followup(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = tokens.first() else {
        return Ok(None);
    };
    let is_transform = first.is_word("transform");
    let is_convert = first.is_word("convert");
    if !is_transform && !is_convert {
        return Ok(None);
    }

    let (head_tokens, tail_tokens) = if let Some((head, tail)) =
        split_lexed_once_on_comma_then(tokens).or_else(|| {
            super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
                super::super::grammar::primitives::kw("then").void()
            })
        }) {
        (head.to_vec(), trim_commas(tail))
    } else {
        (tokens.to_vec(), Vec::new())
    };

    let target_tokens = trim_commas(&head_tokens[1..]);
    let transform = if is_transform {
        parse_transform(&target_tokens)?
    } else {
        parse_convert(&target_tokens)?
    };
    if tail_tokens.is_empty() {
        return Ok(Some(vec![transform]));
    }

    let mut effects = vec![transform];
    effects.extend(parse_effect_chain(&tail_tokens)?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_enchant(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_enchant_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_cant_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence(tokens)
}

pub(crate) fn parse_sentence_prevent_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_prevent_damage_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_gain_ability_to_source(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_gain_ability_to_source_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_gain_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_ability_sentence(tokens)
}

pub(crate) fn parse_sentence_you_and_each_opponent_voted_with_you(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_each_opponent_voted_with_you_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_life_equal_to_power(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_power_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_x_plus_life(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_x_plus_life_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_exiled_this_way(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_exiled_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_put_permanent_cards_exiled_with_source_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_destroyed_this_way(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_destroyed_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_then_return_same_object(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_then_return_same_object_sentence(tokens)
}

pub(crate) fn parse_sentence_search_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_search_library_sentence(tokens)
}

pub(crate) fn parse_sentence_shuffle_graveyard_into_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shuffle_graveyard_into_library_sentence(tokens)
}

pub(crate) fn parse_sentence_shuffle_object_into_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shuffle_object_into_library_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_hand_and_graveyard_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_hand_and_graveyard_bundle_sentence(tokens)
}

pub(crate) fn parse_sentence_target_player_exiles_creature_and_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_target_player_exiles_creature_and_graveyard_sentence(tokens)
}

pub(crate) fn parse_sentence_play_from_graveyard(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_play_from_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_look_at_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_hand_sentence(tokens)
}

pub(crate) fn parse_sentence_look_at_top_then_exile_one(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_top_then_exile_one_sentence(tokens)
}

pub(crate) fn parse_sentence_gain_life_equal_to_age(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_age_sentence(tokens)
}

pub(crate) fn parse_sentence_for_each_opponent_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_opponent_doesnt(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_player_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_player_doesnt(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_each_opponent_loses_x_and_you_gain_x(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if grammar::strip_lexed_prefix_phrase(tokens, &["each", "opponent"]).is_none()
        && grammar::strip_lexed_prefix_phrase(tokens, &["each", "opponents"]).is_none()
    {
        return Ok(None);
    }

    let sentence_words = crate::cards::builders::parser::token_word_refs(tokens);
    let has_lose_x = find_window_by(&sentence_words, 3, |window| {
        (window[0] == "lose" || window[0] == "loses") && window[1] == "x" && window[2] == "life"
    })
    .is_some();
    let has_gain_x = grammar::contains_phrase(tokens, &["you", "gain", "x", "life"]);
    let Some(where_token_idx) = grammar::find_phrase_start(tokens, &["where", "x", "is"]) else {
        return Ok(None);
    };
    if !has_lose_x || !has_gain_x {
        return Ok(None);
    }

    let where_tokens = &tokens[where_token_idx..];
    let where_value = parse_where_x_value_clause(where_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-x value in opponent life-drain clause (clause: '{}')",
            sentence_words.join(" ")
        ))
    })?;

    Ok(Some(vec![
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::LoseLife {
                amount: where_value.clone(),
                player: PlayerAst::Implicit,
            }],
        },
        EffectAst::GainLife {
            amount: where_value,
            player: PlayerAst::You,
        },
    ]))
}

pub(crate) fn parse_sentence_vote_start(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_start_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_for_each_vote_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_vote_clause(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_vote_extra(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_extra_sentence(tokens).map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_after_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_after_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn parse_sentence_same_name_target_fanout(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_target_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_shared_color_target_fanout(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shared_color_target_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_same_name_gets_fanout(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_gets_fanout_sentence(tokens)
}

pub(crate) fn parse_sentence_delayed_until_next_end_step(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_delayed_until_next_end_step_sentence(tokens)
}

pub(crate) fn parse_sentence_destroy_or_exile_all_split(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_or_exile_all_split_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_up_to_one_each_target_type(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_up_to_one_each_target_type_sentence(tokens)
}

pub(crate) fn parse_sentence_exile_multi_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if !tokens.first().is_some_and(|t| t.is_word("exile"))
        || grammar::contains_word(tokens, "unless")
    {
        return Ok(None);
    }

    let mut split_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") || idx == 0 || idx + 1 >= tokens.len() {
            continue;
        }
        let tail = &tokens[idx + 1..];
        let starts_second_target = tail.first().is_some_and(|t| t.is_word("target"))
            || (grammar::strip_lexed_prefix_phrase(tail, &["up", "to"]).is_some()
                && grammar::contains_word(tail, "target"));
        if starts_second_target {
            split_idx = Some(idx);
            break;
        }
    }

    let Some(and_idx) = split_idx else {
        return Ok(None);
    };

    let first_tokens = trim_commas(&tokens[1..and_idx]);
    let second_tokens = trim_commas(&tokens[and_idx + 1..]);
    if first_tokens.is_empty() || second_tokens.is_empty() {
        return Ok(None);
    }

    let first_words = crate::cards::builders::parser::token_word_refs(&first_tokens);
    let first_is_explicit_target = first_tokens.first().is_some_and(|t| t.is_word("target"))
        || (grammar::strip_lexed_prefix_phrase(&first_tokens, &["up", "to"]).is_some()
            && grammar::contains_word(&first_tokens, "target"));
    let second_is_explicit_target = second_tokens.first().is_some_and(|t| t.is_word("target"))
        || (grammar::strip_lexed_prefix_phrase(&second_tokens, &["up", "to"]).is_some()
            && grammar::contains_word(&second_tokens, "target"));

    let mut first_target = match parse_target_phrase(&first_tokens) {
        Ok(target) => target,
        Err(_)
            if !first_is_explicit_target
                && is_likely_named_or_source_reference_words(&first_words) =>
        {
            TargetAst::Source(span_from_tokens(&first_tokens))
        }
        Err(err) => return Err(err),
    };
    let mut second_target = parse_target_phrase(&second_tokens)?;

    if first_is_explicit_target
        && second_is_explicit_target
        && let (Some((mut first_filter, first_count)), Some((mut second_filter, second_count))) = (
            object_target_with_count(&first_target),
            object_target_with_count(&second_target),
        )
        && first_filter.zone == Some(Zone::Graveyard)
        && second_filter.zone == Some(Zone::Graveyard)
    {
        if first_filter.controller.is_none() {
            first_filter.controller = Some(PlayerFilter::Any);
        }
        if second_filter.controller.is_none() {
            second_filter.controller = Some(PlayerFilter::Any);
        }
        let tag = helper_tag_for_tokens(tokens, "exiled");
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(tag, None),
                face_down: false,
            },
        ]));
    }

    apply_exile_subject_hand_owner_context(&mut first_target, None);
    apply_exile_subject_hand_owner_context(&mut second_target, None);
    Ok(Some(vec![
        EffectAst::Exile {
            target: first_target,
            face_down: false,
        },
        EffectAst::Exile {
            target: second_target,
            face_down: false,
        },
    ]))
}

pub(crate) fn split_destroy_target_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut raw_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    for and_segment in split_lexed_slices_on_and(tokens) {
        for comma_segment in split_lexed_slices_on_comma(and_segment) {
            let trimmed = trim_commas(&comma_segment);
            if !trimmed.is_empty() {
                raw_segments.push(trimmed.to_vec());
            }
        }
    }

    let mut segments = Vec::new();
    for segment in raw_segments {
        let split_starts = segment
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| {
                if idx >= 3
                    && token.is_word("target")
                    && segment[idx - 3].is_word("up")
                    && segment[idx - 2].is_word("to")
                    && segment[idx - 1].is_word("one")
                {
                    Some(idx - 3)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if split_starts.len() <= 1 {
            segments.push(segment);
            continue;
        }

        for (idx, start) in split_starts.iter().enumerate() {
            let end = split_starts.get(idx + 1).copied().unwrap_or(segment.len());
            let trimmed = trim_commas(&segment[*start..end]);
            if !trimmed.is_empty() {
                segments.push(trimmed.to_vec());
            }
        }
    }

    segments
}

pub(crate) fn parse_sentence_destroy_multi_target(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    if !tokens.first().is_some_and(|t| t.is_word("destroy")) {
        return Ok(None);
    }
    if tokens
        .get(1)
        .is_some_and(|t| t.is_word("all") || t.is_word("each"))
    {
        return Ok(None);
    }
    if grammar::contains_word(tokens, "unless") || grammar::contains_word(tokens, "if") {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..]);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let has_separator = target_tokens
        .iter()
        .any(|token| token.is_word("and") || token.is_comma());
    let mut repeated_up_to_one_targets = 0usize;
    let mut start = 0usize;
    while start + 4 <= target_tokens.len() {
        let window = &target_tokens[start..start + 4];
        if window[0].is_word("up")
            && window[1].is_word("to")
            && window[2].is_word("one")
            && window[3].is_word("target")
        {
            repeated_up_to_one_targets += 1;
        }
        start += 1;
    }
    let has_repeated_up_to_one_targets = repeated_up_to_one_targets >= 2;
    if !has_separator && !has_repeated_up_to_one_targets {
        return Ok(None);
    }

    let segments = split_destroy_target_segments(&target_tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment in segments {
        let segment_words = crate::cards::builders::parser::token_word_refs(&segment);
        if segment_words.iter().any(|word| {
            matches!(
                *word,
                "then" | "if" | "unless" | "where" | "when" | "whenever"
            )
        }) {
            return Ok(None);
        }
        let is_explicit_target = segment_words.first() == Some(&"target")
            || (grammar::words_match_prefix(&segment, &["up", "to"]).is_some()
                && grammar::contains_word(&segment, "target"));
        if !is_explicit_target && !is_likely_named_or_source_reference_words(&segment_words) {
            return Ok(None);
        }
        let target = match parse_target_phrase(&segment) {
            Ok(target) => target,
            Err(_)
                if !is_explicit_target
                    && is_likely_named_or_source_reference_words(&segment_words) =>
            {
                TargetAst::Source(span_from_tokens(&segment))
            }
            Err(err) => return Err(err),
        };
        effects.push(EffectAst::Destroy { target });
    }

    if effects.len() < 2 {
        return Ok(None);
    }
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_reveal_selected_cards_in_your_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.first() != Some(&"reveal") {
        return Ok(None);
    }
    if clause_words.iter().any(|word| {
        matches!(
            *word,
            "then" | "if" | "unless" | "where" | "when" | "whenever"
        )
    }) {
        return Ok(None);
    }

    use super::super::grammar::primitives as grammar;

    let in_your_hand = grammar::strip_lexed_suffix_phrase(tokens, &["in", "your", "hand"])
        .or_else(|| grammar::strip_lexed_suffix_phrase(tokens, &["in", "your", "hands"]));
    let Some(before_in) = in_your_hand else {
        return Ok(None);
    };
    if before_in.is_empty() {
        return Ok(None);
    }

    let mut descriptor_tokens = trim_commas(&before_in[1..]);
    if descriptor_tokens.is_empty() {
        return Ok(None);
    }

    let mut count = ChoiceCount::exactly(1);
    let descriptor_words = crate::cards::builders::parser::token_word_refs(&descriptor_tokens);
    if grammar::words_match_prefix(&descriptor_tokens, &["any", "number", "of"]).is_some() {
        count = ChoiceCount::any_number();
        descriptor_tokens = trim_commas(&descriptor_tokens[3..]);
    } else if grammar::words_match_prefix(&descriptor_tokens, &["up", "to"]).is_some() {
        if let Some((value, used)) = parse_number(&descriptor_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
            descriptor_tokens = trim_commas(&descriptor_tokens[2 + used..]);
            if descriptor_tokens
                .first()
                .is_some_and(|token| token.is_word("of"))
            {
                descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
            }
        } else {
            return Ok(None);
        }
    } else if descriptor_words.first() == Some(&"x") {
        count = ChoiceCount::any_number();
        descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
    } else if descriptor_words
        .first()
        .is_some_and(|word| matches!(*word, "a" | "an" | "one"))
    {
        descriptor_tokens = trim_commas(&descriptor_tokens[1..]);
    } else if descriptor_words
        .first()
        .is_some_and(|word| matches!(*word, "all" | "each"))
    {
        return Ok(None);
    }

    if descriptor_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = match parse_object_filter(&descriptor_tokens, false) {
        Ok(filter) => filter,
        Err(_) => {
            let descriptor_words =
                crate::cards::builders::parser::token_word_refs(&descriptor_tokens);
            let mut filter = ObjectFilter::default();
            let mut idx = 0usize;
            if let Some(color) = descriptor_words.get(idx).and_then(|word| parse_color(word)) {
                filter.colors = Some(color.into());
                idx += 1;
            }
            if !descriptor_words
                .get(idx)
                .is_some_and(|word| matches!(*word, "card" | "cards"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported reveal-hand clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            filter
        }
    };
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::You);

    let tag = helper_tag_for_tokens(tokens, "revealed");
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::RevealTagged { tag },
    ]))
}

pub(crate) fn object_target_with_count(target: &TargetAst) -> Option<(ObjectFilter, ChoiceCount)> {
    match target {
        TargetAst::Object(filter, _, _) => Some((filter.clone(), ChoiceCount::exactly(1))),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => Some((filter.clone(), count.clone())),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn is_likely_named_or_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if is_source_reference_words(words) {
        return true;
    }
    if words.iter().any(|word| {
        matches!(
            *word,
            "then"
                | "if"
                | "unless"
                | "where"
                | "when"
                | "whenever"
                | "for"
                | "each"
                | "search"
                | "destroy"
                | "exile"
                | "draw"
                | "gain"
                | "lose"
                | "counter"
                | "put"
                | "return"
                | "create"
                | "sacrifice"
                | "deal"
                | "populate"
        )
    }) {
        return false;
    }
    !words.iter().any(|word| {
        matches!(
            *word,
            "a" | "an"
                | "the"
                | "this"
                | "that"
                | "those"
                | "it"
                | "them"
                | "target"
                | "all"
                | "any"
                | "each"
                | "another"
                | "other"
                | "up"
                | "to"
                | "card"
                | "cards"
                | "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "artifact"
                | "artifacts"
                | "enchantment"
                | "enchantments"
                | "land"
                | "lands"
                | "planeswalker"
                | "planeswalkers"
                | "spell"
                | "spells"
        )
    })
}

pub(crate) fn parse_sentence_damage_unless_controller_has_source_deal_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let Some((before_slice, after_unless_slice)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("unless").void())
    else {
        return Ok(None);
    };

    let before_tokens = trim_commas(before_slice);
    if before_tokens.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(&before_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let Some(main_damage) = effects.first() else {
        return Ok(None);
    };
    let EffectAst::DealDamage {
        amount: main_amount,
        target: main_target,
    } = main_damage
    else {
        return Ok(None);
    };
    if !matches!(
        main_target,
        TargetAst::Object(_, _, _) | TargetAst::WithCount(_, _)
    ) {
        return Ok(None);
    }

    let after_unless = trim_commas(after_unless_slice);
    let has_controller_clause = grammar::words_match_prefix(&after_unless, &["that"]).is_some()
        && (grammar::contains_word(&after_unless, "controller")
            || grammar::contains_word(&after_unless, "controllers"));
    if !has_controller_clause {
        return Ok(None);
    }
    let Some(has_idx) = find_index(&after_unless, |token| {
        token.is_word("has") || token.is_word("have")
    }) else {
        return Ok(None);
    };
    if has_idx + 1 >= after_unless.len() {
        return Ok(None);
    }

    let alt_tokens = &after_unless[has_idx + 1..];
    let Some(deal_idx) = find_index(&alt_tokens, |token| {
        token.is_word("deal") || token.is_word("deals")
    }) else {
        return Ok(None);
    };
    let deal_tail = &alt_tokens[deal_idx..];
    let Some((alt_amount, used)) = parse_value(&deal_tail[1..]) else {
        return Ok(None);
    };
    if !deal_tail
        .get(1 + used)
        .is_some_and(|token| token.is_word("damage"))
    {
        return Ok(None);
    }

    let mut alt_target_tokens = &deal_tail[2 + used..];
    if alt_target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        alt_target_tokens = &alt_target_tokens[1..];
    }
    let alt_target_words = crate::cards::builders::parser::token_word_refs(alt_target_tokens);
    if !matches!(alt_target_words.as_slice(), ["them"] | ["that", "player"]) {
        return Ok(None);
    }

    let alternative = EffectAst::DealDamage {
        amount: alt_amount,
        target: TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    };
    let unless = EffectAst::UnlessAction {
        effects: vec![EffectAst::DealDamage {
            amount: main_amount.clone(),
            target: main_target.clone(),
        }],
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    };
    Ok(Some(vec![unless]))
}

pub(crate) fn parse_sentence_damage_to_that_player_unless_enchanted_attacked(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::grammar::primitives as grammar;

    let Some((before_slice, after_slice)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("unless").void())
    else {
        return Ok(None);
    };

    let before_tokens = trim_commas(before_slice);
    let after_tokens = trim_commas(after_slice);
    if before_tokens.is_empty() || after_tokens.is_empty() {
        return Ok(None);
    }

    if !matches!(
        crate::cards::builders::parser::token_word_refs(&after_tokens).as_slice(),
        ["that", "creature", "attacked", "this", "turn"]
            | ["enchanted", "creature", "attacked", "this", "turn"]
    ) {
        return Ok(None);
    }

    let deal_split =
        grammar::split_lexed_once_on_separator(&before_tokens, || grammar::kw("deal").void())
            .or_else(|| {
                grammar::split_lexed_once_on_separator(&before_tokens, || {
                    grammar::kw("deals").void()
                })
            });
    let Some((subject_slice, damage_tokens)) = deal_split else {
        return Ok(None);
    };

    if !matches!(
        crate::cards::builders::parser::token_word_refs(subject_slice).as_slice(),
        ["this", "aura"] | ["this", "permanent"] | ["this", "enchantment"]
    ) {
        return Ok(None);
    }
    let Some((amount, used)) = parse_value(damage_tokens) else {
        return Ok(None);
    };
    if !damage_tokens
        .get(used)
        .is_some_and(|token| token.is_word("damage"))
    {
        return Ok(None);
    }

    let mut target_tokens = trim_commas(&damage_tokens[used + 1..]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens.remove(0);
    }
    if crate::cards::builders::parser::token_word_refs(&target_tokens).as_slice()
        != ["that", "player"]
    {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::Conditional {
        predicate: PredicateAst::Not(Box::new(PredicateAst::EnchantedPermanentAttackedThisTurn)),
        if_true: vec![EffectAst::DealDamage {
            amount,
            target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        }],
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_sentence_unless_pays(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Find "unless" in the token stream
    let unless_idx = match find_index(tokens, |t| t.is_word("unless")) {
        Some(idx) => idx,
        None => return Ok(None),
    };

    // Leading form: "Unless you pay ..., <effects>."
    // Rewrite by parsing the effect tail after the first comma and wrapping it
    // in the parsed unless-payment clause.
    if unless_idx == 0 {
        let comma_idx = match find_index(tokens, |token| token.is_comma()) {
            Some(idx) => idx,
            None => return Ok(None),
        };
        if comma_idx + 1 >= tokens.len() {
            return Ok(None);
        }

        let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
        if effects.is_empty() {
            return Ok(None);
        }

        let unless_clause = &tokens[..comma_idx];
        if let Some(unless_effect) = try_build_unless(effects, unless_clause, 0)? {
            return Ok(Some(vec![unless_effect]));
        }
        return Ok(None);
    }

    // Need at least something before "unless" and something after.
    let before_words: Vec<&str> = tokens[..unless_idx]
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect();

    // Skip "counter ... unless" - already handled by parse_counter via CounterUnlessPays
    if before_words.first() == Some(&"counter") {
        return Ok(None);
    }
    // Ignore "unless ... pays" that appears inside quoted token rules text.
    // Example: create token with "{1}, Sacrifice this token: Counter ... unless ...".
    if before_words.first() == Some(&"create")
        && grammar::contains_word(&tokens[..unless_idx], "token")
        && grammar::contains_word(&tokens[..unless_idx], "sacrifice")
        && grammar::contains_word(&tokens[..unless_idx], "counter")
    {
        return Ok(None);
    }

    // Handle "each opponent/player ... unless" by wrapping in ForEachOpponent/ForEachPlayer.
    // Structure: ForEachOpponent { [UnlessPays/UnlessAction { per-player effects }] }
    let each_prefix = if grammar::words_match_prefix(&tokens[..unless_idx], &["each", "opponent"])
        .is_some()
        || grammar::words_match_prefix(&tokens[..unless_idx], &["each", "opponents"]).is_some()
    {
        Some("opponent")
    } else if grammar::words_match_prefix(&tokens[..unless_idx], &["each", "player"]).is_some() {
        Some("player")
    } else {
        None
    };
    if let Some(prefix_kind) = each_prefix {
        // Tokens between "each opponent/player" and "unless" form the per-player effect
        let inner_token_start = tokens
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_word().map(|_| i))
            .nth(2) // skip "each" and "opponent"/"player"
            .unwrap_or(2);
        let inner_tokens = &tokens[inner_token_start..unless_idx];
        if let Ok(inner_effects) = parse_effect_chain(inner_tokens) {
            if !inner_effects.is_empty() {
                if let Some(unless_effect) = try_build_unless(inner_effects, tokens, unless_idx)? {
                    let wrapper = match prefix_kind {
                        "opponent" => EffectAst::ForEachOpponent {
                            effects: vec![unless_effect],
                        },
                        _ => EffectAst::ForEachPlayer {
                            effects: vec![unless_effect],
                        },
                    };
                    return Ok(Some(vec![wrapper]));
                }
            }
        }
        return Ok(None);
    }

    // Normal path: parse effects before "unless", then build unless wrapper
    let effect_tokens = &tokens[..unless_idx];
    if let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(effect_tokens)
    {
        let Some(timing_token_idx) = token_index_for_word_index(effect_tokens, timing_start_word)
        else {
            return Ok(None);
        };
        let delayed_effect_tokens = trim_commas(&effect_tokens[..timing_token_idx]);
        if delayed_effect_tokens.is_empty() {
            return Ok(None);
        }
        let delayed_effects = parse_effect_chain(&delayed_effect_tokens)?;
        if delayed_effects.is_empty() {
            return Ok(None);
        }
        if let Some(unless_effect) = try_build_unless(delayed_effects, tokens, unless_idx)? {
            return Ok(Some(vec![wrap_delayed_next_step_unless_pays(
                step,
                player,
                vec![unless_effect],
            )]));
        }
    }

    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }

    if let Some(unless_effect) = try_build_unless(effects, tokens, unless_idx)? {
        return Ok(Some(vec![unless_effect]));
    }

    Ok(None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DelayedNextStepKind {
    Upkeep,
    DrawStep,
}

fn delayed_next_step_marker(
    tokens: &[OwnedLexToken],
) -> Option<(usize, usize, DelayedNextStepKind, PlayerAst)> {
    let word_storage = SentencePrimitiveNormalizedWords::new(tokens);
    let words = word_storage.to_word_refs();
    let patterns: &[(&[&str], DelayedNextStepKind, PlayerAst)] = &[
        (
            &["at", "the", "beginning", "of", "your", "next", "upkeep"],
            DelayedNextStepKind::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::You,
        ),
        (
            &["at", "the", "beginning", "of", "their", "next", "upkeep"],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::That,
        ),
    ];

    for (pattern, step, player) in patterns {
        if let Some(start) = find_window_index(&words, pattern) {
            return Some((start, start + pattern.len(), *step, *player));
        }
    }

    None
}

fn wrap_delayed_next_step_unless_pays(
    step: DelayedNextStepKind,
    player: PlayerAst,
    effects: Vec<EffectAst>,
) -> EffectAst {
    match step {
        DelayedNextStepKind::Upkeep => EffectAst::DelayedUntilNextUpkeep { player, effects },
        DelayedNextStepKind::DrawStep => EffectAst::DelayedUntilNextDrawStep { player, effects },
    }
}

pub(crate) fn parse_sentence_delayed_next_step_unless_pays(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_period(tokens);
    if segments.is_empty() {
        return Ok(None);
    }

    let (leading_segments, final_segment) = segments.split_at(segments.len() - 1);
    let final_segment = trim_commas(&final_segment[0]);
    let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(&final_segment)
    else {
        return Ok(None);
    };

    let Some(timing_token_idx) = token_index_for_word_index(&final_segment, timing_start_word)
    else {
        return Ok(None);
    };
    let delayed_effect_tokens = trim_commas(&final_segment[..timing_token_idx]);
    if delayed_effect_tokens.is_empty() {
        return Ok(None);
    }

    let delayed_effects = parse_effect_chain(&delayed_effect_tokens)?;
    if delayed_effects.is_empty() {
        return Ok(None);
    }

    let timing_tokens = trim_commas(&final_segment[timing_token_idx..]);
    let Some(unless_idx) = find_token_word(&timing_tokens, "unless") else {
        return Ok(None);
    };
    let Some(unless_effect) = try_build_unless(delayed_effects, &timing_tokens, unless_idx)? else {
        return Ok(None);
    };

    let mut effects = Vec::new();
    for segment in leading_segments {
        let parsed = parse_effect_chain(segment)?;
        if parsed.is_empty() {
            return Ok(None);
        }
        effects.extend(parsed);
    }
    effects.push(wrap_delayed_next_step_unless_pays(
        step,
        player,
        vec![unless_effect],
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_delayed_next_upkeep_unless_pays_lose_game(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_lexed_slices_on_period(tokens);
    if segments.len() != 2 && segments.len() != 3 {
        return Ok(None);
    }

    let (mut effects, upkeep_tokens, lose_tokens) = if segments.len() == 3 {
        let first_effects = parse_effect_chain(&segments[0])?;
        if first_effects.is_empty() {
            return Ok(None);
        }
        (
            first_effects,
            trim_commas(&segments[1]),
            trim_commas(&segments[2]),
        )
    } else {
        (
            Vec::new(),
            trim_commas(&segments[0]),
            trim_commas(&segments[1]),
        )
    };
    let upkeep_words = crate::cards::builders::parser::token_word_refs(&upkeep_tokens);
    let pay_idx = if grammar::words_match_prefix(
        &upkeep_tokens,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "your",
            "next",
            "upkeep",
            "pay",
        ],
    )
    .is_some()
    {
        7usize
    } else if grammar::words_match_prefix(
        &upkeep_tokens,
        &[
            "at",
            "the",
            "beginning",
            "of",
            "the",
            "next",
            "upkeep",
            "pay",
        ],
    )
    .is_some()
    {
        8usize
    } else {
        return Ok(None);
    };

    let Some(pay_token_idx) = token_index_for_word_index(&upkeep_tokens, pay_idx) else {
        return Ok(None);
    };
    let mana_tokens = &upkeep_tokens[pay_token_idx + 1..];
    if mana_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing mana payment in delayed next-upkeep clause (clause: '{}')",
            upkeep_words.join(" ")
        )));
    }

    let mut mana = Vec::new();
    for token in mana_tokens {
        if let Some(pips) = mana_pips_from_token(token) {
            mana.extend(pips);
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if let Some(symbol) = parse_mana_symbol_word_flexible(word) {
            mana.push(symbol);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported mana payment in delayed next-upkeep clause (clause: '{}')",
            upkeep_words.join(" ")
        )));
    }
    if mana.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing mana payment in delayed next-upkeep clause (clause: '{}')",
            upkeep_words.join(" ")
        )));
    }

    let lose_words = crate::cards::builders::parser::token_word_refs(&lose_tokens);
    let valid_lose_clause = lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"];
    if !valid_lose_clause {
        return Ok(None);
    }

    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::LoseGame {
                player: PlayerAst::You,
            }],
            player: PlayerAst::You,
            mana,
        }],
    });
    Ok(Some(effects))
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
pub(crate) fn try_build_unless(
    effects: Vec<EffectAst>,
    tokens: &[OwnedLexToken],
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_unless = &tokens[unless_idx + 1..];
    let after_word_storage = SentencePrimitiveNormalizedWords::new(after_unless);
    let after_words = after_word_storage.to_word_refs();
    let pay_word_idx = find_index(&after_words, |word| matches!(*word, "pay" | "pays"));
    let pay_token_idx = find_index(after_unless, |token| {
        token.is_word("pay") || token.is_word("pays")
    });

    let match_player_prefix = |prefix: &[&str]| -> Option<(PlayerAst, usize)> {
        if prefix == ["you"] {
            Some((PlayerAst::You, 1))
        } else if prefix == ["target", "opponent"] {
            Some((PlayerAst::TargetOpponent, 2))
        } else if prefix == ["target", "player"] {
            Some((PlayerAst::Target, 2))
        } else if prefix == ["any", "player"] {
            Some((PlayerAst::Any, 2))
        } else if prefix == ["they"] {
            Some((PlayerAst::That, 1))
        } else if prefix == ["defending", "player"] {
            Some((PlayerAst::Defending, 2))
        } else if prefix == ["that", "player"] {
            Some((PlayerAst::That, 2))
        } else if prefix == ["its", "controller"] || prefix == ["their", "controller"] {
            Some((PlayerAst::ItsController, 2))
        } else if prefix == ["its", "owner"] || prefix == ["their", "owner"] {
            Some((PlayerAst::ItsOwner, 2))
        } else if prefix.len() >= 6
            && prefix[0] == "that"
            && prefix[1] == "player"
            && prefix[2] == "or"
            && prefix[3] == "that"
            && matches!(
                prefix[4],
                "ability"
                    | "abilitys"
                    | "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[5], "controller" | "controllers")
        {
            Some((PlayerAst::ThatPlayerOrTargetController, 6))
        } else if prefix.len() >= 3
            && prefix[0] == "that"
            && matches!(
                prefix[1],
                "ability"
                    | "abilitys"
                    | "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[2], "controller" | "controllers")
        {
            Some((PlayerAst::ItsController, 3))
        } else if prefix.len() >= 3
            && prefix[0] == "that"
            && matches!(
                prefix[1],
                "ability"
                    | "abilitys"
                    | "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[2], "owner" | "owners")
        {
            Some((PlayerAst::ItsOwner, 3))
        } else if prefix.len() >= 6
            && prefix[0] == "that"
            && matches!(
                prefix[1],
                "card"
                    | "cards"
                    | "creature"
                    | "creatures"
                    | "object"
                    | "objects"
                    | "permanent"
                    | "permanents"
                    | "planeswalker"
                    | "planeswalkers"
                    | "source"
                    | "sources"
                    | "spell"
                    | "spells"
            )
            && matches!(prefix[2], "controller" | "controllers")
            && prefix[3] == "or"
            && prefix[4] == "that"
            && prefix[5] == "player"
        {
            Some((PlayerAst::ThatPlayerOrTargetController, 6))
        } else {
            None
        }
    };

    let match_player_clause_prefix = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        let max_prefix_len = words.len().min(6);
        for prefix_len in 1..=max_prefix_len {
            if let Some((player, consumed)) = match_player_prefix(&words[..prefix_len]) {
                return Some((player, consumed));
            }
        }
        None
    };

    // Determine the player from the "unless" clause
    let Some((player, action_word_start)) = (if let Some(pay_idx) = pay_word_idx {
        match_player_prefix(&after_words[..pay_idx]).map(|(player, _)| (player, pay_idx))
    } else {
        match_player_clause_prefix(&after_words)
    }) else {
        return Ok(None);
    };

    let action_token_idx = if let Some(pay_idx) = pay_token_idx {
        pay_idx
    } else {
        after_word_storage
            .token_index_after_words(action_word_start)
            .unwrap_or(0)
    };

    let action_tokens = &after_unless[action_token_idx..];
    let action_word_storage = SentencePrimitiveNormalizedWords::new(action_tokens);
    let action_words = action_word_storage.to_word_refs();

    // "unless [player] pays N life" should compile as an unless-action branch
    // where the deciding player loses life.
    if action_words.first() == Some(&"pay") || action_words.first() == Some(&"pays") {
        let life_tokens = &action_tokens[1..];
        if let Some((amount, used)) = parse_value(life_tokens)
            && life_tokens
                .get(used)
                .is_some_and(|token| token.is_word("life"))
            && life_tokens
                .get(used + 1)
                .map_or(true, |token| token.is_period())
        {
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative: vec![EffectAst::LoseLife { amount, player }],
                player,
            }));
        }
    }

    // Try mana payment first: "pay(s) {mana} [optional trailing condition]"
    // Uses greedy mana parsing — collects mana symbols until first non-mana word,
    // then categorizes remaining tokens to decide whether to accept.
    if action_words.first() == Some(&"pay") || action_words.first() == Some(&"pays") {
        if contains_word_window(&action_words, &["mana", "cost"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported unless-payment mana-cost clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            )));
        }

        // Skip any non-word tokens between "pay" and mana
        let mana_start = find_index(&action_tokens[1..], |token| {
            token.as_word().is_some() || mana_pips_from_token(token).is_some()
        })
        .map(|idx| idx + 1)
        .unwrap_or(1);
        let mana_tokens = &action_tokens[mana_start..];
        let mut mana = Vec::new();
        let mut remaining_idx = mana_tokens.len();
        for (i, token) in mana_tokens.iter().enumerate() {
            if let Some(group) = mana_pips_from_token(token) {
                mana.extend(group);
                continue;
            }
            if let Some(word) = token.as_word() {
                match parse_mana_symbol(word) {
                    Ok(symbol) => mana.push(symbol),
                    Err(_) => {
                        remaining_idx = i;
                        break;
                    }
                }
            }
        }

        if !mana.is_empty() {
            // Check what follows the mana symbols
            let remaining_word_storage =
                SentencePrimitiveNormalizedWords::new(&mana_tokens[remaining_idx..]);
            let remaining_words = remaining_word_storage.to_word_refs();

            let accept = if remaining_words.is_empty() {
                // Pure mana payment (e.g., "pays {2}")
                true
            } else if remaining_words.first() == Some(&"life") {
                // "pay N life" — not a mana payment, it's a life cost
                false
            } else if remaining_words.first() == Some(&"before") {
                // Timing condition like "before that step" — accept, drop condition
                true
            } else {
                // Unknown trailing tokens (for each, where X is, etc.) — skip for now
                false
            };

            if accept {
                return Ok(Some(EffectAst::UnlessPays {
                    effects,
                    player,
                    mana,
                }));
            }

            if !remaining_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing unless-payment clause (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            }
        }
    }

    // Try full-clause parsing first to preserve existing behavior for explicit
    // player phrasing such as "unless that player ...".
    if let Ok(mut alternative) = parse_effect_chain(after_unless) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(after_unless) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_chain(action_tokens) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_sentence_lexed(action_tokens) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if let Ok(mut alternative) = parse_effect_clause(action_tokens).map(|effect| vec![effect]) {
        if !alternative.is_empty() {
            for effect in &mut alternative {
                bind_implicit_player_context(effect, player);
            }
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    if matches!(action_words.first().copied(), Some("discard" | "discards"))
        && let Ok(mut alternative) =
            super::zone_handlers::parse_discard(action_tokens, None).map(|effect| vec![effect])
    {
        for effect in &mut alternative {
            bind_implicit_player_context(effect, player);
        }
        return Ok(Some(EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        }));
    }

    Ok(None)
}

pub(crate) fn parse_sentence_fallback_mechanic_marker(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.as_slice() == ["venture", "into", "the", "dungeon"] {
        return Ok(Some(vec![EffectAst::VentureIntoDungeon {
            player: crate::cards::builders::PlayerAst::You,
            undercity_if_no_active: false,
        }]));
    }

    let is_match = clause_words.as_slice() == ["its", "still", "a", "land"]
        || clause_words.as_slice() == ["it", "still", "a", "land"]
        || grammar::words_match_prefix(tokens, &["you", "choose", "one", "of", "them"]).is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "you", "may", "put", "a", "land", "card", "from", "among", "them", "into", "your",
                "hand",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(tokens, &["stand", "and", "fight"]).is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "chooses",
                "any",
                "number",
                "of",
                "creatures",
                "they",
                "control",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "each",
                "player",
                "chooses",
                "any",
                "number",
                "of",
                "creatures",
                "they",
                "control",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["an", "opponent", "chooses", "one", "of", "those", "piles"],
        )
        .is_some()
        || grammar::words_match_prefix(tokens, &["put", "that", "pile", "into", "your", "hand"])
            .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["cast", "that", "card", "for", "as", "long", "as"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "until", "end", "of", "turn", "this", "creature", "loses", "prevent", "all",
                "damage",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "until",
                "end",
                "of",
                "turn",
                "target",
                "creature",
                "loses",
                "all",
                "abilities",
                "and",
                "has",
                "base",
                "power",
                "and",
                "toughness",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["for", "each", "1", "damage", "prevented", "this", "way"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "for", "each", "card", "less", "than", "two", "a", "player", "draws", "this", "way",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["this", "deals", "4", "damage", "if", "there", "are"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "this", "deals", "4", "damage", "instead", "if", "there", "are",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "that", "spell", "deals", "damage", "to", "each", "opponent", "equal", "to",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "the", "next", "spell", "you", "cast", "this", "turn", "costs",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "there",
                "is",
                "an",
                "additional",
                "combat",
                "phase",
                "after",
                "this",
                "phase",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "that",
                "creature",
                "attacks",
                "during",
                "its",
                "controllers",
                "next",
                "combat",
                "phase",
                "if",
                "able",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to", "target",
                "creature", "you", "control", "by", "a", "source", "of", "your", "choice", "is",
                "dealt", "to", "another", "target", "creature", "instead",
            ],
        )
        .is_some()
        || (grammar::words_match_prefix(tokens, &["it", "doesnt", "untap", "during"]).is_some()
            && grammar::contains_word(tokens, "remains")
            && grammar::contains_word(tokens, "tapped"));
    if !is_match {
        return Ok(None);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported mechanic marker clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_sentence_implicit_become_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let target = if grammar::words_match_prefix(tokens, &["its"]).is_some()
        || grammar::words_match_prefix(tokens, &["it", "is"]).is_some()
        || grammar::words_match_prefix(tokens, &["it", "s"]).is_some()
        || grammar::words_match_prefix(tokens, &["it\u{2019}s"]).is_some()
        || grammar::words_match_prefix(tokens, &["it’s"]).is_some()
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), None)
    } else if grammar::words_match_prefix(tokens, &["each", "of", "them", "is"]).is_some()
        || grammar::words_match_prefix(tokens, &["they", "are"]).is_some()
        || grammar::words_match_prefix(tokens, &["they", "re"]).is_some()
        || grammar::words_match_prefix(tokens, &["theyre"]).is_some()
        || grammar::words_match_prefix(tokens, &["they\u{2019}re"]).is_some()
        || grammar::words_match_prefix(tokens, &["they’re"]).is_some()
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), None)
    } else {
        return Ok(None);
    };
    let rest_word_idx = if grammar::words_match_prefix(tokens, &["its"]).is_some() {
        1usize
    } else if grammar::words_match_prefix(tokens, &["it", "is"]).is_some() {
        2usize
    } else if grammar::words_match_prefix(tokens, &["it", "s"]).is_some() {
        2usize
    } else if grammar::words_match_prefix(tokens, &["it\u{2019}s"]).is_some()
        || grammar::words_match_prefix(tokens, &["it’s"]).is_some()
    {
        1usize
    } else if grammar::words_match_prefix(tokens, &["they", "are"]).is_some() {
        2usize
    } else if grammar::words_match_prefix(tokens, &["they", "re"]).is_some() {
        2usize
    } else if grammar::words_match_prefix(tokens, &["theyre"]).is_some()
        || grammar::words_match_prefix(tokens, &["they\u{2019}re"]).is_some()
        || grammar::words_match_prefix(tokens, &["they’re"]).is_some()
    {
        1usize
    } else {
        4usize
    };

    let rest_token_idx = token_index_for_word_index(tokens, rest_word_idx).unwrap_or(tokens.len());
    let rest_tokens = trim_commas(&tokens[rest_token_idx..]);
    let mut rest_words = crate::cards::builders::parser::token_word_refs(&rest_tokens);
    if rest_words.first().copied() == Some("still") {
        rest_words.remove(0);
    }
    if rest_words.is_empty() {
        return Ok(None);
    }

    let negative_type_words =
        if slice_starts_with(&rest_words, &["not", "a"]) && rest_words.len() > 2 {
            Some(&rest_words[2..])
        } else if slice_starts_with(&rest_words, &["not", "an"]) && rest_words.len() > 2 {
            Some(&rest_words[2..])
        } else if slice_starts_with(&rest_words, &["not"]) && rest_words.len() > 1 {
            Some(&rest_words[1..])
        } else {
            None
        };
    if let Some(type_words) = negative_type_words {
        let mut card_types = Vec::new();
        let mut all_card_types = true;
        for word in type_words {
            if let Some(card_type) = parse_card_type(word) {
                if !iter_contains(card_types.iter(), &card_type) {
                    card_types.push(card_type);
                }
            } else {
                all_card_types = false;
                break;
            }
        }
        if all_card_types && !card_types.is_empty() {
            return Ok(Some(vec![EffectAst::RemoveCardTypes {
                target,
                card_types,
                duration: Until::Forever,
            }]));
        }
    }

    let addition_tail_len = if slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "its", "other", "types"],
    ) || slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "their", "other", "types"],
    ) || slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "its", "other", "type"],
    ) || slice_ends_with(
        &rest_words,
        &["in", "addition", "to", "their", "other", "type"],
    ) {
        Some(6usize)
    } else {
        None
    };

    let body_words = if rest_words
        .first()
        .is_some_and(|word| matches!(*word, "a" | "an" | "the"))
    {
        &rest_words[1..]
    } else {
        &rest_words[..]
    };
    if body_words.is_empty() {
        return Ok(None);
    }

    if let Ok((power, toughness)) = parse_pt_modifier_values(body_words[0])
        && let Some(tail_len) = addition_tail_len
        && body_words.len() > 1 + tail_len
    {
        let subtype_words = &body_words[1..body_words.len().saturating_sub(tail_len)];
        let mut subtypes = Vec::new();
        for word in subtype_words {
            let Some(subtype) = parse_pluralized_subtype_word(word) else {
                return Ok(None);
            };
            if !iter_contains(subtypes.iter(), &subtype) {
                subtypes.push(subtype);
            }
        }
        if subtypes.is_empty() {
            return Ok(None);
        }
        return Ok(Some(vec![
            EffectAst::SetBasePowerToughness {
                power,
                toughness,
                target: target.clone(),
                duration: Until::Forever,
            },
            EffectAst::AddSubtypes {
                target,
                subtypes,
                duration: Until::Forever,
            },
        ]));
    }

    let type_words = if let Some(tail_len) = addition_tail_len {
        &body_words[..body_words.len().saturating_sub(tail_len)]
    } else {
        body_words
    };
    if type_words.is_empty() {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut all_card_types = true;
    for word in type_words {
        if let Some(card_type) = parse_card_type(word) {
            if !iter_contains(card_types.iter(), &card_type) {
                card_types.push(card_type);
            }
        } else {
            all_card_types = false;
            break;
        }
    }
    if all_card_types && !card_types.is_empty() {
        return Ok(Some(vec![EffectAst::AddCardTypes {
            target,
            card_types,
            duration: Until::Forever,
        }]));
    }

    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_pluralized_subtype_word(word) else {
            return Ok(None);
        };
        if !iter_contains(subtypes.iter(), &subtype) {
            subtypes.push(subtype);
        }
    }
    if subtypes.is_empty() {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::AddSubtypes {
        target,
        subtypes,
        duration: Until::Forever,
    }]))
}

pub(crate) const PRE_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "implicit-become-clause",
        parser: parse_sentence_implicit_become_clause,
    },
    SentencePrimitive {
        name: "fallback-mechanic-marker",
        parser: parse_sentence_fallback_mechanic_marker,
    },
    SentencePrimitive {
        name: "if-tagged-cards-remain-exiled",
        parser: parse_sentence_if_tagged_cards_remain_exiled,
    },
    SentencePrimitive {
        name: "if-enters-with-additional-counter",
        parser: parse_if_enters_with_additional_counter_sentence,
    },
    SentencePrimitive {
        name: "put-multiple-counters-on-target",
        parser: parse_sentence_put_multiple_counters_on_target,
    },
    SentencePrimitive {
        name: "you-and-target-player-each-draw",
        parser: parse_sentence_you_and_target_player_each_draw,
    },
    SentencePrimitive {
        name: "choose-player-to-effect",
        parser: parse_sentence_choose_player_to_effect,
    },
    SentencePrimitive {
        name: "you-and-attacking-player-each-draw-and-lose",
        parser: parse_sentence_you_and_attacking_player_each_draw_and_lose,
    },
    SentencePrimitive {
        name: "sacrifice-it-next-end-step",
        parser: parse_sentence_sacrifice_it_next_end_step,
    },
    SentencePrimitive {
        name: "sacrifice-at-end-of-combat",
        parser: parse_sentence_sacrifice_at_end_of_combat,
    },
    SentencePrimitive {
        name: "each-player-choose-keep-rest-sacrifice",
        parser: parse_sentence_each_player_choose_and_sacrifice_rest,
    },
    SentencePrimitive {
        name: "target-player-choose-then-put-on-top-library",
        parser: parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    },
    SentencePrimitive {
        name: "target-player-choose-then-you-put-it-onto-battlefield",
        parser: parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    },
    SentencePrimitive {
        name: "exile-instead-of-graveyard",
        parser: parse_sentence_exile_instead_of_graveyard,
    },
];

pub(crate) static PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX: LazyLock<SentencePrimitiveIndex> =
    LazyLock::new(|| build_sentence_primitive_index(PRE_CONDITIONAL_SENTENCE_PRIMITIVES));

pub(crate) const POST_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "exile-target-creature-with-greatest-power",
        parser: parse_sentence_exile_target_creature_with_greatest_power,
    },
    SentencePrimitive {
        name: "counter-target-spell-thats-second-cast-this-turn",
        parser: parse_sentence_counter_target_spell_thats_second_cast_this_turn,
    },
    SentencePrimitive {
        name: "counter-target-spell-if-it-was-kicked",
        parser: parse_sentence_counter_target_spell_if_it_was_kicked,
    },
    SentencePrimitive {
        name: "destroy-creature-type-of-choice",
        parser: parse_sentence_destroy_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "pump-creature-type-of-choice",
        parser: parse_sentence_pump_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "return-multiple-targets",
        parser: parse_sentence_return_multiple_targets,
    },
    SentencePrimitive {
        name: "choose-all-battlefield-graveyard-to-hand",
        parser: parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand,
    },
    SentencePrimitive {
        name: "for-each-of-target-objects",
        parser: parse_sentence_for_each_of_target_objects,
    },
    SentencePrimitive {
        name: "return-creature-type-of-choice",
        parser: parse_sentence_return_targets_of_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "distribute-counters",
        parser: parse_sentence_distribute_counters,
    },
    SentencePrimitive {
        name: "keyword-then-chain",
        parser: parse_sentence_keyword_then_chain,
    },
    SentencePrimitive {
        name: "chain-then-keyword",
        parser: parse_sentence_chain_then_keyword,
    },
    SentencePrimitive {
        name: "exile-then-may-put-from-exile",
        parser: parse_sentence_exile_then_may_put_from_exile,
    },
    SentencePrimitive {
        name: "exile-source-with-counters",
        parser: parse_sentence_exile_source_with_counters,
    },
    SentencePrimitive {
        name: "destroy-all-attached-to-target",
        parser: parse_sentence_destroy_all_attached_to_target,
    },
    SentencePrimitive {
        name: "comma-then-chain-special",
        parser: parse_sentence_comma_then_chain_special,
    },
    SentencePrimitive {
        name: "destroy-then-land-controller-graveyard-count-damage",
        parser: parse_sentence_destroy_then_land_controller_graveyard_count_damage,
    },
    SentencePrimitive {
        name: "draw-then-connive",
        parser: parse_sentence_draw_then_connive,
    },
    SentencePrimitive {
        name: "return-then-do-same-for-subtypes",
        parser: parse_sentence_return_then_do_same_for_subtypes,
    },
    SentencePrimitive {
        name: "return-then-create",
        parser: parse_sentence_return_then_create,
    },
    SentencePrimitive {
        name: "put-counter-sequence",
        parser: parse_sentence_put_counter_sequence,
    },
    SentencePrimitive {
        name: "gets-then-fights",
        parser: parse_sentence_gets_then_fights,
    },
    SentencePrimitive {
        name: "return-with-counters-on-it",
        parser: parse_sentence_return_with_counters_on_it,
    },
    SentencePrimitive {
        name: "each-player-return-with-additional-counter",
        parser: parse_sentence_each_player_return_with_additional_counter,
    },
    SentencePrimitive {
        name: "sacrifice-any-number",
        parser: parse_sentence_sacrifice_any_number,
    },
    SentencePrimitive {
        name: "sacrifice-one-or-more",
        parser: parse_sentence_sacrifice_one_or_more,
    },
    SentencePrimitive {
        name: "monstrosity",
        parser: parse_sentence_monstrosity,
    },
    SentencePrimitive {
        name: "for-each-counter-removed",
        parser: parse_sentence_for_each_counter_removed,
    },
    SentencePrimitive {
        name: "for-each-counter-kind-put-or-remove",
        parser: parse_sentence_for_each_counter_kind_put_or_remove,
    },
    SentencePrimitive {
        name: "take-extra-turn",
        parser: parse_sentence_take_extra_turn,
    },
    SentencePrimitive {
        name: "earthbend",
        parser: parse_sentence_earthbend,
    },
    SentencePrimitive {
        name: "transform-with-followup",
        parser: parse_sentence_transform_with_followup,
    },
    SentencePrimitive {
        name: "enchant",
        parser: parse_sentence_enchant,
    },
    SentencePrimitive {
        name: "cant-effect",
        parser: parse_sentence_cant_effect,
    },
    SentencePrimitive {
        name: "prevent-damage",
        parser: parse_sentence_prevent_damage,
    },
    SentencePrimitive {
        name: "shared-color-target-fanout",
        parser: parse_sentence_shared_color_target_fanout,
    },
    SentencePrimitive {
        name: "gain-ability-to-source",
        parser: parse_sentence_gain_ability_to_source,
    },
    SentencePrimitive {
        name: "gain-ability",
        parser: parse_sentence_gain_ability,
    },
    SentencePrimitive {
        name: "vote-with-you",
        parser: parse_sentence_you_and_each_opponent_voted_with_you,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-power",
        parser: parse_sentence_gain_life_equal_to_power,
    },
    SentencePrimitive {
        name: "gain-x-plus-life",
        parser: parse_sentence_gain_x_plus_life,
    },
    SentencePrimitive {
        name: "for-each-exiled-this-way",
        parser: parse_sentence_for_each_exiled_this_way,
    },
    SentencePrimitive {
        name: "draw-for-each-card-exiled-from-hand-this-way",
        parser: parse_sentence_draw_for_each_card_exiled_from_hand_this_way,
    },
    SentencePrimitive {
        name: "each-player-reveals-top-count-put-permanents-rest-graveyard",
        parser:
            parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard,
    },
    SentencePrimitive {
        name: "each-player-put-permanent-cards-exiled-with-source",
        parser: parse_sentence_each_player_put_permanent_cards_exiled_with_source,
    },
    SentencePrimitive {
        name: "for-each-destroyed-this-way",
        parser: parse_sentence_for_each_destroyed_this_way,
    },
    SentencePrimitive {
        name: "delayed-next-step-unless-pays",
        parser: parse_sentence_delayed_next_step_unless_pays,
    },
    SentencePrimitive {
        name: "search-delayed-next-upkeep-unless-pays-lose-game",
        parser: parse_sentence_delayed_next_upkeep_unless_pays_lose_game,
    },
    SentencePrimitive {
        name: "exile-then-return-same-object",
        parser: parse_sentence_exile_then_return_same_object,
    },
    SentencePrimitive {
        name: "search-library",
        parser: parse_sentence_search_library,
    },
    SentencePrimitive {
        name: "shuffle-graveyard-into-library",
        parser: parse_sentence_shuffle_graveyard_into_library,
    },
    SentencePrimitive {
        name: "shuffle-object-into-library",
        parser: parse_sentence_shuffle_object_into_library,
    },
    SentencePrimitive {
        name: "exile-hand-and-graveyard-bundle",
        parser: parse_sentence_exile_hand_and_graveyard_bundle,
    },
    SentencePrimitive {
        name: "target-player-exiles-creature-and-graveyard",
        parser: parse_sentence_target_player_exiles_creature_and_graveyard,
    },
    SentencePrimitive {
        name: "play-from-graveyard",
        parser: parse_sentence_play_from_graveyard,
    },
    SentencePrimitive {
        name: "look-at-top-then-exile-one",
        parser: parse_sentence_look_at_top_then_exile_one,
    },
    SentencePrimitive {
        name: "look-at-hand",
        parser: parse_sentence_look_at_hand,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-age",
        parser: parse_sentence_gain_life_equal_to_age,
    },
    SentencePrimitive {
        name: "for-each-player-doesnt",
        parser: parse_sentence_for_each_player_doesnt,
    },
    SentencePrimitive {
        name: "for-each-opponent-doesnt",
        parser: parse_sentence_for_each_opponent_doesnt,
    },
    SentencePrimitive {
        name: "each-opponent-loses-x-and-you-gain-x",
        parser: parse_sentence_each_opponent_loses_x_and_you_gain_x,
    },
    SentencePrimitive {
        name: "vote-start",
        parser: parse_sentence_vote_start,
    },
    SentencePrimitive {
        name: "for-each-vote-clause",
        parser: parse_sentence_for_each_vote_clause,
    },
    SentencePrimitive {
        name: "vote-extra",
        parser: parse_sentence_vote_extra,
    },
    SentencePrimitive {
        name: "after-turn",
        parser: parse_sentence_after_turn,
    },
    SentencePrimitive {
        name: "same-name-target-fanout",
        parser: parse_sentence_same_name_target_fanout,
    },
    SentencePrimitive {
        name: "same-name-gets-fanout",
        parser: parse_sentence_same_name_gets_fanout,
    },
    SentencePrimitive {
        name: "delayed-next-end-step",
        parser: parse_sentence_delayed_until_next_end_step,
    },
    SentencePrimitive {
        name: "delayed-when-that-dies-this-turn",
        parser: parse_delayed_when_that_dies_this_turn_sentence,
    },
    SentencePrimitive {
        name: "delayed-trigger-this-turn",
        parser: parse_sentence_delayed_trigger_this_turn,
    },
    SentencePrimitive {
        name: "destroy-or-exile-all-split",
        parser: parse_sentence_destroy_or_exile_all_split,
    },
    SentencePrimitive {
        name: "exile-up-to-one-each-target-type",
        parser: parse_sentence_exile_up_to_one_each_target_type,
    },
    SentencePrimitive {
        name: "exile-multi-target",
        parser: parse_sentence_exile_multi_target,
    },
    SentencePrimitive {
        name: "destroy-multi-target",
        parser: parse_sentence_destroy_multi_target,
    },
    SentencePrimitive {
        name: "reveal-selected-cards-in-your-hand",
        parser: parse_sentence_reveal_selected_cards_in_your_hand,
    },
    SentencePrimitive {
        name: "damage-unless-controller-has-source-deal-damage",
        parser: parse_sentence_damage_unless_controller_has_source_deal_damage,
    },
    SentencePrimitive {
        name: "damage-to-that-player-unless-enchanted-attacked",
        parser: parse_sentence_damage_to_that_player_unless_enchanted_attacked,
    },
    SentencePrimitive {
        name: "damage-to-that-player-half-damage-of-those-spells",
        parser: parse_sentence_damage_to_that_player_half_damage_of_those_spells,
    },
    SentencePrimitive {
        name: "unless-pays",
        parser: parse_sentence_unless_pays,
    },
];

pub(crate) static POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX: LazyLock<SentencePrimitiveIndex> =
    LazyLock::new(|| build_sentence_primitive_index(POST_CONDITIONAL_SENTENCE_PRIMITIVES));
