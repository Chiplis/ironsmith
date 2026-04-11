#![allow(dead_code)]

use super::super::clause_support::parse_ability_line_lexed;
use super::super::grammar::primitives as grammar;
use super::super::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::super::token_primitives::{
    contains_window as word_slice_contains_sequence, find_str_by as find_word_index,
    slice_contains_str as word_slice_contains, slice_ends_with as word_slice_ends_with,
    slice_starts_with as word_slice_starts_with,
};
use super::super::util::{parse_zone_word, trim_commas};
use super::chain_carry::{Verb, find_verb};
use super::clause_pattern_helpers::{
    parse_can_attack_as_though_no_defender_clause, parse_prevent_all_damage_clause,
    parse_prevent_next_damage_clause,
};
use super::clause_primitives::{
    parse_attack_or_block_this_turn_if_able_clause, parse_attack_this_turn_if_able_clause,
    parse_must_block_if_able_clause,
};

const EACH_PLAYER_OR_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["each", "player"],
    &["each", "players"],
    &["each", "opponent"],
    &["each", "opponents"],
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
];

const INLINE_TOKEN_RULES_TAIL_PREFIXES: &[&[&str]] = &[
    &["when"],
    &["whenever"],
    &["when", "this", "token"],
    &["whenever", "this", "token"],
    &["this", "token"],
    &["that", "token"],
    &["those", "tokens"],
    &["except", "it"],
    &["except", "they"],
    &["except", "its"],
    &["except", "their"],
    &["this", "creature"],
    &["that", "creature"],
    &["at", "the", "beginning"],
    &["at", "beginning"],
    &["sacrifice", "this", "token"],
    &["sacrifice", "that", "token"],
    &["sacrifice", "this", "permanent"],
    &["sacrifice", "that", "permanent"],
    &["sacrifice", "it"],
    &["sacrifice", "them"],
    &["it", "has"],
    &["it", "gains"],
    &["they", "have"],
    &["they", "gain"],
    &["equip"],
    &["equipped", "creature"],
    &["enchanted", "creature"],
    &["r"],
    &["t"],
];

const DESTROY_EXILE_GAIN_CONTROL_ALL_PREFIXES: &[&[&str]] = &[
    &["destroy", "all"],
    &["exile", "all"],
    &["gain", "control", "of", "all"],
];
const GENERIC_FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"], &["each"]];
const REST_PREFIXES: &[&[&str]] = &[&["the", "rest"], &["rest"]];
const PHASES_END_PREFIXES: &[&[&str]] = &[&["phases", "end"]];
const CLASH_PREFIXES: &[&[&str]] = &[&["clash"], &["clashes"]];
const THAT_MANY_FOLLOWUP_PREFIXES: &[&[&str]] = &[
    &["draw", "that", "many"],
    &["draws", "that", "many"],
    &["create", "that", "many"],
    &["creates", "that", "many"],
];
const LIFE_EQUAL_TO_THAT_PREFIXES: &[&[&str]] = &[
    &["gain", "life", "equal", "to", "that"],
    &["gains", "life", "equal", "to", "that"],
    &["lose", "life", "equal", "to", "that"],
    &["loses", "life", "equal", "to", "that"],
];
const DEAL_DAMAGE_EQUAL_TO_PREFIXES: &[&[&str]] = &[
    &["it", "deal", "damage", "equal", "to"],
    &["it", "deals", "damage", "equal", "to"],
    &["that", "creature", "deal", "damage", "equal", "to"],
    &["that", "creature", "deals", "damage", "equal", "to"],
    &["that", "objects", "deal", "damage", "equal", "to"],
    &["that", "objects", "deals", "damage", "equal", "to"],
];
const PUT_PREFIXES: &[&[&str]] = &[&["put"], &["puts"]];
const PUT_BACK_PREFIXES: &[&[&str]] = &[
    &["put", "it", "back"],
    &["put", "them", "back"],
    &["puts", "it", "back"],
    &["puts", "them", "back"],
];

pub(crate) fn strip_leading_instead_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    if !tokens.first().is_some_and(|token| token.is_word("instead"))
        || tokens
            .get(1)
            .is_some_and(|token| token.is_word("of") || token.is_word("if"))
    {
        return None;
    }

    let stripped = trim_commas(&tokens[1..]);
    if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    }
}

pub(crate) fn strip_leading_instead_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    if !tokens.first().is_some_and(|token| token.is_word("instead"))
        || tokens
            .get(1)
            .is_some_and(|token| token.is_word("of") || token.is_word("if"))
    {
        return None;
    }

    let stripped = trim_lexed_commas(&tokens[1..]);
    if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    }
}

fn is_basic_color_word(word: &str) -> bool {
    matches!(
        word,
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}

fn starts_with_each_player_or_opponent(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_match_any_prefix(tokens, EACH_PLAYER_OR_OPPONENT_PREFIXES).is_some()
}

pub(crate) fn starts_with_inline_token_rules_tail(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_match_any_prefix(tokens, INLINE_TOKEN_RULES_TAIL_PREFIXES).is_some()
}

fn starts_with_inline_token_rules_continuation(words: &[&str]) -> bool {
    matches!(
        words.first().copied(),
        Some(
            "it" | "they"
                | "that"
                | "those"
                | "this"
                | "gain"
                | "gains"
                | "draw"
                | "draws"
                | "add"
                | "deal"
                | "deals"
                | "destroy"
                | "destroys"
                | "exile"
                | "exiles"
                | "return"
                | "returns"
                | "tap"
                | "untap"
                | "sacrifice"
                | "create"
                | "put"
                | "fights"
                | "fight"
        )
    )
}

fn starts_with_nonverb_effect_head(words: &[&str]) -> bool {
    words.first().is_some_and(|word| {
        matches!(
            *word,
            "double"
                | "distribute"
                | "support"
                | "bolster"
                | "adapt"
                | "open"
                | "manifest"
                | "populate"
                | "connive"
                | "earthbend"
        )
    })
}

pub(crate) fn is_token_creation_context(tokens: &[OwnedLexToken]) -> bool {
    tokens.first().is_some_and(|t| t.is_word("create"))
        && (grammar::contains_word(tokens, "token") || grammar::contains_word(tokens, "tokens"))
}

fn has_inline_token_rules_context(words: &[&str]) -> bool {
    word_slice_contains_sequence(words, &["when", "this", "token"])
        || word_slice_contains_sequence(words, &["whenever", "this", "token"])
        || word_slice_contains_sequence(words, &["at", "the", "beginning", "of"])
        || (word_slice_contains(words, "except")
            && word_slice_contains(words, "copy")
            && word_slice_contains(words, "token"))
}

fn should_keep_and_for_token_rules(current: &[OwnedLexToken], remaining: &[OwnedLexToken]) -> bool {
    should_keep_and_for_token_rules_lexed(current, remaining)
}

fn should_keep_and_for_attachment_object_list(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_attachment_object_list_lexed(current, remaining)
}

fn should_keep_and_for_each_player_may_clause(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_each_player_may_clause_lexed(current, remaining)
}

fn should_keep_and_for_put_rest_clause(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_put_rest_clause_lexed(current, remaining)
}

fn should_keep_and_for_steps_and_phases_end(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_steps_and_phases_end_lexed(current, remaining)
}

fn should_keep_and_for_exchange_zones(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_exchange_zones_lexed(current, remaining)
}

pub(crate) fn split_effect_chain_on_and(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("and") {
            let prev_word = current.last().and_then(OwnedLexToken::as_word);
            let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
            let is_color_pair = prev_word.zip(next_word).is_some_and(|(left, right)| {
                is_basic_color_word(left) && is_basic_color_word(right)
            });
            if is_color_pair
                || should_keep_and_for_token_rules(&current, &tokens[idx + 1..])
                || should_keep_and_for_attachment_object_list(&current, &tokens[idx + 1..])
                || should_keep_and_for_each_player_may_clause(&current, &tokens[idx + 1..])
                || should_keep_and_for_put_rest_clause(&current, &tokens[idx + 1..])
                || should_keep_and_for_steps_and_phases_end(&current, &tokens[idx + 1..])
                || should_keep_and_for_exchange_zones(&current, &tokens[idx + 1..])
            {
                current.push(token.clone());
                continue;
            }
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn find_verb_lexed(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        let lower = word.to_ascii_lowercase();
        if matches!(lower.as_str(), "counter" | "counters")
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .map(str::to_ascii_lowercase)
                .as_deref()
                .is_some_and(|next| matches!(next, "on" | "from" | "among"))
        {
            continue;
        }
        let local = match lower.as_str() {
            "adds" | "add" => Verb::Add,
            "moves" | "move" => Verb::Move,
            "deals" | "deal" => Verb::Deal,
            "draws" | "draw" => Verb::Draw,
            "counters" | "counter" => Verb::Counter,
            "destroys" | "destroy" => Verb::Destroy,
            "exiles" | "exile" => Verb::Exile,
            "reveals" | "reveal" => Verb::Reveal,
            "looks" | "look" => Verb::Look,
            "loses" | "lose" => Verb::Lose,
            "gains" | "gain" => Verb::Gain,
            "puts" | "put" => Verb::Put,
            "sacrifices" | "sacrifice" => Verb::Sacrifice,
            "creates" | "create" => Verb::Create,
            "investigates" | "investigate" => Verb::Investigate,
            "proliferates" | "proliferate" => Verb::Proliferate,
            "taps" | "tap" => Verb::Tap,
            "attaches" | "attach" => Verb::Attach,
            "untaps" | "untap" => Verb::Untap,
            "scries" | "scry" => Verb::Scry,
            "discards" | "discard" => Verb::Discard,
            "transforms" | "transform" => Verb::Transform,
            "converts" | "convert" => Verb::Convert,
            "flips" | "flip" => Verb::Flip,
            "rolls" | "roll" => Verb::Roll,
            "regenerates" | "regenerate" => Verb::Regenerate,
            "mills" | "mill" => Verb::Mill,
            "gets" | "get" => Verb::Get,
            "removes" | "remove" => Verb::Remove,
            "returns" | "return" => Verb::Return,
            "exchanges" | "exchange" => Verb::Exchange,
            "becomes" | "become" => Verb::Become,
            "switches" | "switch" => Verb::Switch,
            "skips" | "skip" => Verb::Skip,
            "surveils" | "surveil" => Verb::Surveil,
            "shuffles" | "shuffle" => Verb::Shuffle,
            "reorders" | "reorder" => Verb::Reorder,
            "pays" | "pay" => Verb::Pay,
            "detains" | "detain" => Verb::Detain,
            "goads" | "goad" => Verb::Goad,
            _ => continue,
        };
        return Some((local, idx));
    }

    None
}

fn should_keep_and_for_token_rules_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = token_word_refs(current);
    if current_words.is_empty() {
        return false;
    }
    if !is_token_creation_context(current) && !has_inline_token_rules_context(&current_words) {
        return false;
    }
    starts_with_inline_token_rules_tail(remaining)
}

fn should_keep_and_for_attachment_object_list_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = token_word_refs(current);
    let remaining_words = token_word_refs(remaining);
    if current_words.is_empty() || remaining_words.is_empty() {
        return false;
    }

    let starts_attachment_subject = remaining_words.first().is_some_and(|word| {
        matches!(
            *word,
            "aura"
                | "auras"
                | "equipment"
                | "equipments"
                | "enchantment"
                | "enchantments"
                | "artifact"
                | "artifacts"
        )
    });
    if !starts_attachment_subject || !grammar::contains_word(remaining, "attached") {
        return false;
    }

    grammar::words_match_any_prefix(current, DESTROY_EXILE_GAIN_CONTROL_ALL_PREFIXES).is_some()
}

fn should_keep_and_for_each_player_may_clause_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = token_word_refs(current);
    if current_words.is_empty() || !grammar::contains_word(current, "may") {
        return false;
    }

    if !starts_with_each_player_or_opponent(current) {
        return false;
    }

    if remaining.is_empty() {
        return false;
    }
    if grammar::words_match_any_prefix(remaining, GENERIC_FOR_EACH_PREFIXES).is_some() {
        return false;
    }

    true
}

fn should_keep_and_for_put_rest_clause_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }

    let current_words = token_word_refs(current);
    if current_words.is_empty() {
        return false;
    }

    let starts_with_rest = grammar::words_match_any_prefix(remaining, REST_PREFIXES).is_some();
    if !starts_with_rest {
        return false;
    }

    grammar::contains_word(current, "put")
        && grammar::contains_word(current, "into")
        && grammar::contains_word(current, "hand")
}

fn should_keep_and_for_steps_and_phases_end_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    grammar::words_match_suffix(current, &["as", "steps"]).is_some()
        && grammar::words_match_any_prefix(remaining, PHASES_END_PREFIXES).is_some()
}

fn should_keep_and_for_exchange_zones_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    let current_words = token_word_refs(current);
    let remaining_words = token_word_refs(remaining);
    current_words.first().copied() == Some("exchange")
        && current_words
            .iter()
            .any(|word| parse_zone_word(word).is_some())
        && remaining_words
            .first()
            .is_some_and(|word| parse_zone_word(word).is_some())
}

fn is_prevent_next_damage_clause_words_lexed(words: &[&str]) -> bool {
    if words.first().copied() != Some("prevent") {
        return false;
    }

    let mut idx = 1usize;
    if words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if words.get(idx) != Some(&"next") {
        return false;
    }
    idx += 1;

    if words.get(idx).is_none() {
        return false;
    }
    idx += 1;

    words.get(idx) == Some(&"damage")
        && words.get(idx + 1..idx + 5) == Some(["that", "would", "be", "dealt"].as_slice())
        && words.get(idx + 5) == Some(&"to")
        && word_slice_ends_with(words, &["this", "turn"])
        && words.len() > idx + 7
}

fn is_prevent_all_damage_clause_words_lexed(words: &[&str]) -> bool {
    let prefix_target_then_duration = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ];
    let prefix_duration_then_target = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
    ];

    if word_slice_starts_with(words, &prefix_duration_then_target) {
        return words.len() > prefix_duration_then_target.len();
    }

    word_slice_starts_with(words, &prefix_target_then_duration)
        && words.len() > prefix_target_then_duration.len() + 1
        && word_slice_ends_with(words, &["this", "turn"])
}

fn is_can_attack_as_though_no_defender_clause_words_lexed(words: &[&str]) -> bool {
    let Some(can_idx) = find_word_index(words, |word| word == "can") else {
        return false;
    };
    let tail = &words[can_idx..];
    word_slice_starts_with(tail, &["can", "attack"])
        && word_slice_contains_sequence(tail, &["as", "though"])
        && word_slice_contains(tail, "turn")
        && word_slice_contains(tail, "have")
        && tail.last().copied() == Some("defender")
}

fn is_attack_or_block_this_turn_if_able_clause_words_lexed(words: &[&str]) -> bool {
    let Some(attack_idx) = find_word_index(words, |word| matches!(word, "attack" | "attacks"))
    else {
        return false;
    };
    matches!(
        &words[attack_idx..],
        ["attack", "or", "block", "this", "turn", "if", "able"]
            | ["attacks", "or", "blocks", "this", "turn", "if", "able"]
            | ["attacks", "or", "block", "this", "turn", "if", "able"]
            | ["attack", "or", "blocks", "this", "turn", "if", "able"]
    )
}

fn is_attack_this_turn_if_able_clause_words_lexed(words: &[&str]) -> bool {
    let Some(attack_idx) = find_word_index(words, |word| matches!(word, "attack" | "attacks"))
    else {
        return false;
    };
    matches!(
        &words[attack_idx..],
        ["attack", "this", "turn", "if", "able"] | ["attacks", "this", "turn", "if", "able"]
    )
}

fn is_must_block_if_able_clause_words_lexed(words: &[&str]) -> bool {
    if matches!(
        words,
        ["all", "creatures", "able", "to", "block", .., "do", "so"]
    ) {
        return true;
    }

    let Some(block_idx) = find_word_index(words, |word| matches!(word, "block" | "blocks")) else {
        return false;
    };
    if block_idx == 0 || block_idx + 1 >= words.len() {
        return false;
    }

    let tail = &words[block_idx..];
    matches!(
        tail,
        ["block", "this", "turn", "if", "able"] | ["blocks", "this", "turn", "if", "able"]
    ) || word_slice_ends_with(tail, &["if", "able"])
}

pub(crate) fn split_effect_chain_on_and_lexed(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut start = 0usize;

    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let current = trim_lexed_commas(&tokens[start..idx]);
        let remaining = trim_lexed_commas(&tokens[idx + 1..]);
        let prev_word = current.iter().rev().find_map(OwnedLexToken::as_word);
        let next_word = remaining.iter().find_map(OwnedLexToken::as_word);
        let is_color_pair = prev_word
            .zip(next_word)
            .is_some_and(|(left, right)| is_basic_color_word(left) && is_basic_color_word(right));
        if is_color_pair
            || should_keep_and_for_token_rules_lexed(current, remaining)
            || should_keep_and_for_attachment_object_list_lexed(current, remaining)
            || should_keep_and_for_each_player_may_clause_lexed(current, remaining)
            || should_keep_and_for_put_rest_clause_lexed(current, remaining)
            || should_keep_and_for_steps_and_phases_end_lexed(current, remaining)
            || should_keep_and_for_exchange_zones_lexed(current, remaining)
        {
            continue;
        }
        if !current.is_empty() {
            segments.push(current);
        }
        start = idx + 1;
    }

    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

pub(crate) fn has_effect_head_without_verb(tokens: &[OwnedLexToken]) -> bool {
    let token_words = token_word_refs(tokens);
    if matches!(
        token_words.as_slice(),
        ["repeat", "this", "process"] | ["and", "repeat", "this", "process"]
    ) {
        return true;
    }

    if starts_with_nonverb_effect_head(&token_words) {
        return true;
    }

    parse_prevent_next_damage_clause(tokens)
        .ok()
        .flatten()
        .is_some()
        || parse_prevent_all_damage_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_can_attack_as_though_no_defender_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_attack_or_block_this_turn_if_able_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_attack_this_turn_if_able_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_must_block_if_able_clause(tokens)
            .ok()
            .flatten()
            .is_some()
}

pub(crate) fn has_effect_head_without_verb_lexed(tokens: &[OwnedLexToken]) -> bool {
    let token_words = token_word_refs(tokens);
    if matches!(
        token_words.as_slice(),
        ["repeat", "this", "process"] | ["and", "repeat", "this", "process"]
    ) {
        return true;
    }

    if starts_with_nonverb_effect_head(&token_words) {
        return true;
    }

    is_prevent_next_damage_clause_words_lexed(&token_words)
        || is_prevent_all_damage_clause_words_lexed(&token_words)
        || is_can_attack_as_though_no_defender_clause_words_lexed(&token_words)
        || is_attack_or_block_this_turn_if_able_clause_words_lexed(&token_words)
        || is_attack_this_turn_if_able_clause_words_lexed(&token_words)
        || is_must_block_if_able_clause_words_lexed(&token_words)
}

pub(crate) fn segment_has_effect_head_lexed(tokens: &[OwnedLexToken]) -> bool {
    find_verb_lexed(tokens).is_some() || has_effect_head_without_verb_lexed(tokens)
}

pub(crate) fn segment_has_effect_head(tokens: &[OwnedLexToken]) -> bool {
    find_verb(tokens).is_some() || has_effect_head_without_verb(tokens)
}

pub(crate) fn split_segments_on_comma_then(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let segment_refs = segments.iter().map(Vec::as_slice).collect::<Vec<_>>();
    split_segments_on_comma_then_lexed(segment_refs)
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect()
}

pub(crate) fn split_segments_on_comma_effect_head(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let segment_refs = segments.iter().map(Vec::as_slice).collect::<Vec<_>>();
    split_segments_on_comma_effect_head_lexed(segment_refs)
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect()
}

pub(crate) fn split_segments_on_comma_then_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let back_ref_words = ["that", "it", "them", "its"];
    let mut result = Vec::new();
    for segment in segments {
        let starts_with_for_each_player_or_opponent = starts_with_each_player_or_opponent(segment);
        let mut split_point = None;
        for i in 0..segment.len().saturating_sub(1) {
            if matches!(segment[i].kind, TokenKind::Comma)
                && segment.get(i + 1).is_some_and(|t| t.is_word("then"))
            {
                let before_then = trim_lexed_commas(&segment[..i]);
                let starts_with_clash =
                    grammar::words_match_any_prefix(before_then, CLASH_PREFIXES).is_some();
                let after_then = trim_lexed_commas(&segment[i + 2..]);
                let after_words = token_word_refs(after_then);
                let has_back_ref = after_words
                    .iter()
                    .any(|w| word_slice_contains(&back_ref_words, w));
                let has_nonverb_effect_head = after_words.first().is_some_and(|word| {
                    matches!(
                        *word,
                        "double"
                            | "distribute"
                            | "support"
                            | "bolster"
                            | "adapt"
                            | "open"
                            | "manifest"
                            | "connive"
                            | "earthbend"
                    )
                });
                let has_effect_head = find_verb_lexed(after_then).is_some()
                    || parse_ability_line_lexed(after_then).is_some()
                    || has_nonverb_effect_head;
                let allow_backref_split = has_back_ref
                    && after_words
                        .first()
                        .is_some_and(|word| *word == "put" || *word == "double")
                    && after_words
                        .iter()
                        .any(|word| *word == "counter" || *word == "counters");
                let allow_attach_followup = after_words
                    .first()
                    .is_some_and(|word| matches!(*word, "attach" | "attaches"));
                let allow_that_many_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && grammar::words_match_any_prefix(after_then, THAT_MANY_FOLLOWUP_PREFIXES)
                        .is_some();
                let allow_gain_or_lose_life_equal_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && grammar::words_match_any_prefix(after_then, LIFE_EQUAL_TO_THAT_PREFIXES)
                            .is_some();
                let allow_deal_damage_equal_power_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && grammar::words_match_any_prefix(
                            after_then,
                            DEAL_DAMAGE_EQUAL_TO_PREFIXES,
                        )
                        .is_some();
                let allow_for_each_damage_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, GENERIC_FOR_EACH_PREFIXES)
                        .is_some()
                    && after_words
                        .iter()
                        .any(|word| *word == "deal" || *word == "deals")
                    && after_words.iter().any(|word| *word == "damage");
                let allow_return_with_counter_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && after_words.first().is_some_and(|word| *word == "return")
                    && after_words
                        .iter()
                        .any(|word| *word == "counter" || *word == "counters")
                    && (grammar::words_find_phrase(after_then, &["on", "it"]).is_some()
                        || grammar::words_find_phrase(after_then, &["on", "them"]).is_some());
                let allow_put_into_hand_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, PUT_PREFIXES).is_some()
                    && grammar::contains_word(after_then, "into")
                    && grammar::contains_word(after_then, "hand");
                let allow_put_back_in_any_order_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, PUT_BACK_PREFIXES).is_some()
                    && grammar::contains_word(after_then, "any")
                    && grammar::contains_word(after_then, "order");
                let allow_clash_followup = starts_with_clash;
                if has_effect_head && (!has_back_ref || allow_backref_split)
                    || has_effect_head && allow_clash_followup
                    || has_effect_head && allow_attach_followup
                    || has_effect_head && allow_that_many_followup
                    || has_effect_head && allow_gain_or_lose_life_equal_followup
                    || has_effect_head && allow_deal_damage_equal_power_followup
                    || has_effect_head && allow_for_each_damage_followup
                    || has_effect_head && allow_return_with_counter_followup
                    || has_effect_head && allow_put_into_hand_followup
                    || has_effect_head && allow_put_back_in_any_order_followup
                {
                    split_point = Some(i);
                    break;
                }
            }
        }
        if let Some(idx) = split_point {
            let first_part = trim_lexed_commas(&segment[..idx]);
            let second_part = trim_lexed_commas(&segment[idx + 2..]);
            if !first_part.is_empty() {
                result.push(first_part);
            }
            if !second_part.is_empty() {
                result.push(second_part);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

pub(crate) fn split_segments_on_comma_effect_head_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let mut start = 0usize;
        let mut split_any = false;

        for idx in 0..segment.len() {
            if !matches!(segment[idx].kind, TokenKind::Comma) {
                continue;
            }
            let before = trim_lexed_commas(&segment[start..idx]);
            let after = trim_lexed_commas(&segment[idx + 1..]);
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let before_has_verb = find_verb_lexed(before).is_some();
            let after_starts_effect = find_verb_lexed(after)
                .is_some_and(|(_, verb_idx)| verb_idx == 0)
                || has_effect_head_without_verb_lexed(after);
            let before_words = token_word_refs(before);
            let after_words = token_word_refs(after);
            let duration_trigger_prefix = (before_words.first() == Some(&"until")
                || before_words.first() == Some(&"during"))
                && (grammar::contains_word(before, "whenever")
                    || grammar::contains_word(before, "when")
                    || grammar::words_find_phrase(before, &["at", "the"]).is_some());
            if before_words.first() == Some(&"unless") || duration_trigger_prefix {
                continue;
            }
            if grammar::contains_word(before, "search") && grammar::contains_word(before, "library")
            {
                continue;
            }
            let is_inline_token_rules_split = (is_token_creation_context(before)
                || has_inline_token_rules_context(&before_words))
                && (starts_with_inline_token_rules_tail(after)
                    || starts_with_inline_token_rules_continuation(&after_words));
            if is_inline_token_rules_split {
                continue;
            }
            if before_has_verb && after_starts_effect {
                result.push(before);
                start = idx + 1;
                split_any = true;
            }
        }
        if split_any {
            let tail = trim_lexed_commas(&segment[start..]);
            if !tail.is_empty() {
                result.push(tail);
            }
        } else {
            result.push(segment);
        }
    }
    result
}
