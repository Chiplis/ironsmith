#![allow(dead_code)]

use super::super::clause_support::rewrite_parse_ability_line_lexed;
use super::super::keyword_static::parse_ability_line;
use super::super::lexer::{OwnedLexToken, TokenKind, lexed_words, trim_lexed_commas};
use super::super::util::{trim_commas, words};
use super::chain_carry::{Verb, find_verb};
use super::clause_pattern_helpers::{
    parse_can_attack_as_though_no_defender_clause, parse_prevent_all_damage_clause,
    parse_prevent_next_damage_clause,
};
use super::clause_primitives::{
    parse_attack_or_block_this_turn_if_able_clause, parse_attack_this_turn_if_able_clause,
    parse_must_block_if_able_clause,
};

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

pub(crate) fn starts_with_inline_token_rules_tail(words: &[&str]) -> bool {
    words.starts_with(&["when"])
        || words.starts_with(&["whenever"])
        || words.starts_with(&["when", "this", "token"])
        || words.starts_with(&["whenever", "this", "token"])
        || words.starts_with(&["this", "token"])
        || words.starts_with(&["that", "token"])
        || words.starts_with(&["those", "tokens"])
        || words.starts_with(&["except", "it"])
        || words.starts_with(&["except", "they"])
        || words.starts_with(&["except", "its"])
        || words.starts_with(&["except", "their"])
        || words.starts_with(&["this", "creature"])
        || words.starts_with(&["that", "creature"])
        || words.starts_with(&["at", "the", "beginning"])
        || words.starts_with(&["at", "beginning"])
        || words.starts_with(&["sacrifice", "this", "token"])
        || words.starts_with(&["sacrifice", "that", "token"])
        || words.starts_with(&["sacrifice", "this", "permanent"])
        || words.starts_with(&["sacrifice", "that", "permanent"])
        || words.starts_with(&["sacrifice", "it"])
        || words.starts_with(&["sacrifice", "them"])
        || words.starts_with(&["it", "has"])
        || words.starts_with(&["it", "gains"])
        || words.starts_with(&["they", "have"])
        || words.starts_with(&["they", "gain"])
        || words.starts_with(&["equip"])
        || words.starts_with(&["equipped", "creature"])
        || words.starts_with(&["enchanted", "creature"])
        || words.starts_with(&["r"])
        || words.starts_with(&["t"])
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

pub(crate) fn is_token_creation_context(words: &[&str]) -> bool {
    words.first().copied() == Some("create")
        && words.iter().any(|word| matches!(*word, "token" | "tokens"))
}

fn has_inline_token_rules_context(words: &[&str]) -> bool {
    words.windows(3).any(|window| {
        matches!(
            window,
            ["when", "this", "token"] | ["whenever", "this", "token"]
        )
    }) || words
        .windows(4)
        .any(|window| window == ["at", "the", "beginning", "of"])
        || (words.contains(&"except") && words.contains(&"copy") && words.contains(&"token"))
}

fn should_keep_and_for_token_rules(current: &[OwnedLexToken], remaining: &[OwnedLexToken]) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = words(current);
    if current_words.is_empty() {
        return false;
    }
    if !is_token_creation_context(&current_words) && !has_inline_token_rules_context(&current_words)
    {
        return false;
    }
    let remaining_words = words(remaining);
    starts_with_inline_token_rules_tail(&remaining_words)
}

fn should_keep_and_for_attachment_object_list(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = words(current);
    let remaining_words = words(remaining);
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
    if !starts_attachment_subject || !remaining_words.contains(&"attached") {
        return false;
    }

    current_words.starts_with(&["destroy", "all"])
        || current_words.starts_with(&["exile", "all"])
        || current_words.starts_with(&["gain", "control", "of", "all"])
}

fn should_keep_and_for_each_player_may_clause(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = words(current);
    if current_words.is_empty() || !current_words.contains(&"may") {
        return false;
    }

    let starts_for_each_player_or_opponent = current_words.starts_with(&["each", "player"])
        || current_words.starts_with(&["each", "players"])
        || current_words.starts_with(&["each", "opponent"])
        || current_words.starts_with(&["each", "opponents"])
        || current_words.starts_with(&["for", "each", "player"])
        || current_words.starts_with(&["for", "each", "players"])
        || current_words.starts_with(&["for", "each", "opponent"])
        || current_words.starts_with(&["for", "each", "opponents"]);
    if !starts_for_each_player_or_opponent {
        return false;
    }

    let remaining_words = words(remaining);
    if remaining_words.is_empty() {
        return false;
    }
    if remaining_words.starts_with(&["for", "each"]) || remaining_words.starts_with(&["each"]) {
        return false;
    }

    true
}

fn should_keep_and_for_put_rest_clause(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }

    let current_words = words(current);
    let remaining_words = words(remaining);
    if current_words.is_empty() || remaining_words.is_empty() {
        return false;
    }

    let starts_with_rest =
        remaining_words.starts_with(&["the", "rest"]) || remaining_words.starts_with(&["rest"]);
    if !starts_with_rest {
        return false;
    }

    current_words.contains(&"put")
        && current_words.contains(&"into")
        && current_words.contains(&"hand")
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
            "flips" | "flip" => Verb::Flip,
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
    let current_words = lexed_words(current);
    if current_words.is_empty() {
        return false;
    }
    if !is_token_creation_context(&current_words) && !has_inline_token_rules_context(&current_words)
    {
        return false;
    }
    let remaining_words = lexed_words(remaining);
    starts_with_inline_token_rules_tail(&remaining_words)
}

fn should_keep_and_for_attachment_object_list_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = lexed_words(current);
    let remaining_words = lexed_words(remaining);
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
    if !starts_attachment_subject || !remaining_words.contains(&"attached") {
        return false;
    }

    current_words.starts_with(&["destroy", "all"])
        || current_words.starts_with(&["exile", "all"])
        || current_words.starts_with(&["gain", "control", "of", "all"])
}

fn should_keep_and_for_each_player_may_clause_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = lexed_words(current);
    if current_words.is_empty() || !current_words.contains(&"may") {
        return false;
    }

    let starts_with_for_each_player_or_opponent = current_words.starts_with(&["each", "player"])
        || current_words.starts_with(&["each", "players"])
        || current_words.starts_with(&["each", "opponent"])
        || current_words.starts_with(&["each", "opponents"])
        || current_words.starts_with(&["for", "each", "player"])
        || current_words.starts_with(&["for", "each", "players"])
        || current_words.starts_with(&["for", "each", "opponent"])
        || current_words.starts_with(&["for", "each", "opponents"]);
    if !starts_with_for_each_player_or_opponent {
        return false;
    }

    let remaining_words = lexed_words(remaining);
    if remaining_words.is_empty() {
        return false;
    }
    if remaining_words.starts_with(&["for", "each"]) || remaining_words.starts_with(&["each"]) {
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

    let current_words = lexed_words(current);
    let remaining_words = lexed_words(remaining);
    if current_words.is_empty() || remaining_words.is_empty() {
        return false;
    }

    let starts_with_rest =
        remaining_words.starts_with(&["the", "rest"]) || remaining_words.starts_with(&["rest"]);
    if !starts_with_rest {
        return false;
    }

    current_words.contains(&"put")
        && current_words.contains(&"into")
        && current_words.contains(&"hand")
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
        && words.ends_with(&["this", "turn"])
        && words.len() > idx + 7
}

fn is_prevent_all_damage_clause_words_lexed(words: &[&str]) -> bool {
    let prefix_target_then_duration = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
    ];
    let prefix_duration_then_target = [
        "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
    ];

    if words.starts_with(&prefix_duration_then_target) {
        return words.len() > prefix_duration_then_target.len();
    }

    words.starts_with(&prefix_target_then_duration)
        && words.len() > prefix_target_then_duration.len() + 1
        && words.ends_with(&["this", "turn"])
}

fn is_can_attack_as_though_no_defender_clause_words_lexed(words: &[&str]) -> bool {
    let Some(can_idx) = words.iter().position(|word| *word == "can") else {
        return false;
    };
    let tail = &words[can_idx..];
    tail.starts_with(&["can", "attack"])
        && tail.windows(2).any(|window| window == ["as", "though"])
        && tail.contains(&"turn")
        && tail.contains(&"have")
        && tail.last().copied() == Some("defender")
}

fn is_attack_or_block_this_turn_if_able_clause_words_lexed(words: &[&str]) -> bool {
    let Some(attack_idx) = words
        .iter()
        .position(|word| matches!(*word, "attack" | "attacks"))
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
    let Some(attack_idx) = words
        .iter()
        .position(|word| matches!(*word, "attack" | "attacks"))
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

    let Some(block_idx) = words
        .iter()
        .position(|word| matches!(*word, "block" | "blocks"))
    else {
        return false;
    };
    if block_idx == 0 || block_idx + 1 >= words.len() {
        return false;
    }

    let tail = &words[block_idx..];
    matches!(
        tail,
        ["block", "this", "turn", "if", "able"] | ["blocks", "this", "turn", "if", "able"]
    ) || tail.ends_with(&["if", "able"])
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
    let token_words = words(tokens);
    if matches!(
        token_words.as_slice(),
        ["repeat", "this", "process"] | ["and", "repeat", "this", "process"]
    ) {
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
    let token_words = lexed_words(tokens);
    if matches!(
        token_words.as_slice(),
        ["repeat", "this", "process"] | ["and", "repeat", "this", "process"]
    ) {
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
    let back_ref_words = ["that", "it", "them", "its"];
    let mut result = Vec::new();
    for segment in segments {
        let segment_words = words(&segment);
        let starts_with_for_each_player_or_opponent = segment_words
            .starts_with(&["each", "player"])
            || segment_words.starts_with(&["each", "players"])
            || segment_words.starts_with(&["each", "opponent"])
            || segment_words.starts_with(&["each", "opponents"])
            || segment_words.starts_with(&["for", "each", "player"])
            || segment_words.starts_with(&["for", "each", "players"])
            || segment_words.starts_with(&["for", "each", "opponent"])
            || segment_words.starts_with(&["for", "each", "opponents"]);
        let mut split_point = None;
        for i in 0..segment.len().saturating_sub(1) {
            if segment[i].is_comma() && segment.get(i + 1).is_some_and(|t| t.is_word("then")) {
                let before_then = &segment[..i];
                let before_words = words(before_then);
                let starts_with_clash =
                    before_words.starts_with(&["clash"]) || before_words.starts_with(&["clashes"]);
                let after_then = &segment[i + 2..];
                let after_words = words(after_then);
                let has_back_ref = after_words.iter().any(|w| back_ref_words.contains(w));
                let has_nonverb_effect_head = after_then
                    .first()
                    .and_then(OwnedLexToken::as_word)
                    .is_some_and(|word| {
                        matches!(
                            word,
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
                let has_effect_head = find_verb(after_then).is_some()
                    || parse_ability_line(after_then).is_some()
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
                    && (after_words.starts_with(&["draw", "that", "many"])
                        || after_words.starts_with(&["draws", "that", "many"])
                        || after_words.starts_with(&["create", "that", "many"])
                        || after_words.starts_with(&["creates", "that", "many"]));
                let allow_gain_or_lose_life_equal_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && (after_words.starts_with(&["gain", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["gains", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["lose", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["loses", "life", "equal", "to", "that"]));
                let allow_deal_damage_equal_power_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && (after_words.starts_with(&["it", "deal", "damage", "equal", "to"])
                            || after_words.starts_with(&["it", "deals", "damage", "equal", "to"])
                            || after_words.starts_with(&[
                                "that", "creature", "deal", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "creature", "deals", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "objects", "deal", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "objects", "deals", "damage", "equal", "to",
                            ]));
                let allow_for_each_damage_followup = has_back_ref
                    && (after_words.starts_with(&["each"])
                        || after_words.starts_with(&["for", "each"]))
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
                    && after_words
                        .windows(2)
                        .any(|window| window == ["on", "it"] || window == ["on", "them"]);
                let allow_put_into_hand_followup = has_back_ref
                    && (after_words.starts_with(&["put"]) || after_words.starts_with(&["puts"]))
                    && after_words.contains(&"into")
                    && after_words.contains(&"hand");
                let allow_put_back_in_any_order_followup = has_back_ref
                    && (after_words.starts_with(&["put", "it", "back"])
                        || after_words.starts_with(&["put", "them", "back"])
                        || after_words.starts_with(&["puts", "it", "back"])
                        || after_words.starts_with(&["puts", "them", "back"]))
                    && after_words.contains(&"any")
                    && after_words.contains(&"order");
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
            let first_part = segment[..idx].to_vec();
            let second_part = segment[idx + 2..].to_vec();
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

pub(crate) fn split_segments_on_comma_effect_head(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut result = Vec::new();
    for segment in segments {
        let mut start = 0usize;
        let mut split_any = false;

        for idx in 0..segment.len() {
            if !segment[idx].is_comma() {
                continue;
            }
            let before = trim_commas(&segment[start..idx]);
            let after = trim_commas(&segment[idx + 1..]);
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let before_has_verb = find_verb(before.as_slice()).is_some();
            let after_starts_effect = find_verb(after.as_slice())
                .is_some_and(|(_, verb_idx)| verb_idx == 0)
                || has_effect_head_without_verb(after.as_slice());
            let before_words = words(before.as_slice());
            let after_words = words(after.as_slice());
            let duration_trigger_prefix = (before_words.first() == Some(&"until")
                || before_words.first() == Some(&"during"))
                && (before_words.contains(&"whenever")
                    || before_words.contains(&"when")
                    || before_words
                        .windows(2)
                        .any(|window| window == ["at", "the"]));
            if before_words.first() == Some(&"unless") || duration_trigger_prefix {
                continue;
            }
            if before_words.contains(&"search") && before_words.contains(&"library") {
                continue;
            }
            let is_inline_token_rules_split = (is_token_creation_context(&before_words)
                || has_inline_token_rules_context(&before_words))
                && (starts_with_inline_token_rules_tail(&after_words)
                    || starts_with_inline_token_rules_continuation(&after_words));
            if is_inline_token_rules_split {
                continue;
            }
            if before_has_verb && after_starts_effect {
                if !split_any && start == 0 {
                    result.push(before.to_vec());
                } else {
                    result.push(segment[start..idx].to_vec());
                }
                start = idx + 1;
                split_any = true;
            }
        }
        if split_any {
            let tail = trim_commas(&segment[start..]).to_vec();
            if !tail.is_empty() {
                result.push(tail);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

pub(crate) fn split_segments_on_comma_then_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let back_ref_words = ["that", "it", "them", "its"];
    let mut result = Vec::new();
    for segment in segments {
        let segment_words = lexed_words(segment);
        let starts_with_for_each_player_or_opponent = segment_words
            .starts_with(&["each", "player"])
            || segment_words.starts_with(&["each", "players"])
            || segment_words.starts_with(&["each", "opponent"])
            || segment_words.starts_with(&["each", "opponents"])
            || segment_words.starts_with(&["for", "each", "player"])
            || segment_words.starts_with(&["for", "each", "players"])
            || segment_words.starts_with(&["for", "each", "opponent"])
            || segment_words.starts_with(&["for", "each", "opponents"]);
        let mut split_point = None;
        for i in 0..segment.len().saturating_sub(1) {
            if matches!(segment[i].kind, TokenKind::Comma)
                && segment.get(i + 1).is_some_and(|t| t.is_word("then"))
            {
                let before_then = trim_lexed_commas(&segment[..i]);
                let before_words = lexed_words(before_then);
                let starts_with_clash =
                    before_words.starts_with(&["clash"]) || before_words.starts_with(&["clashes"]);
                let after_then = trim_lexed_commas(&segment[i + 2..]);
                let after_words = lexed_words(after_then);
                let has_back_ref = after_words.iter().any(|w| back_ref_words.contains(w));
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
                    || rewrite_parse_ability_line_lexed(after_then).is_some()
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
                    && (after_words.starts_with(&["draw", "that", "many"])
                        || after_words.starts_with(&["draws", "that", "many"])
                        || after_words.starts_with(&["create", "that", "many"])
                        || after_words.starts_with(&["creates", "that", "many"]));
                let allow_gain_or_lose_life_equal_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && (after_words.starts_with(&["gain", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["gains", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["lose", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["loses", "life", "equal", "to", "that"]));
                let allow_deal_damage_equal_power_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && (after_words.starts_with(&["it", "deal", "damage", "equal", "to"])
                            || after_words.starts_with(&["it", "deals", "damage", "equal", "to"])
                            || after_words.starts_with(&[
                                "that", "creature", "deal", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "creature", "deals", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "objects", "deal", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "objects", "deals", "damage", "equal", "to",
                            ]));
                let allow_for_each_damage_followup = has_back_ref
                    && (after_words.starts_with(&["each"])
                        || after_words.starts_with(&["for", "each"]))
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
                    && after_words
                        .windows(2)
                        .any(|window| window == ["on", "it"] || window == ["on", "them"]);
                let allow_put_into_hand_followup = has_back_ref
                    && (after_words.starts_with(&["put"]) || after_words.starts_with(&["puts"]))
                    && after_words.contains(&"into")
                    && after_words.contains(&"hand");
                let allow_put_back_in_any_order_followup = has_back_ref
                    && (after_words.starts_with(&["put", "it", "back"])
                        || after_words.starts_with(&["put", "them", "back"])
                        || after_words.starts_with(&["puts", "it", "back"])
                        || after_words.starts_with(&["puts", "them", "back"]))
                    && after_words.contains(&"any")
                    && after_words.contains(&"order");
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
            let before_words = lexed_words(before);
            let after_words = lexed_words(after);
            let duration_trigger_prefix = (before_words.first() == Some(&"until")
                || before_words.first() == Some(&"during"))
                && (before_words.contains(&"whenever")
                    || before_words.contains(&"when")
                    || before_words
                        .windows(2)
                        .any(|window| window == ["at", "the"]));
            if before_words.first() == Some(&"unless") || duration_trigger_prefix {
                continue;
            }
            if before_words.contains(&"search") && before_words.contains(&"library") {
                continue;
            }
            let is_inline_token_rules_split = (is_token_creation_context(&before_words)
                || has_inline_token_rules_context(&before_words))
                && (starts_with_inline_token_rules_tail(&after_words)
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
