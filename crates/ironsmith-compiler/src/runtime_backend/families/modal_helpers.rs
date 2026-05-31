#![allow(dead_code, unused_imports)]

use crate::cards::builders::IfResultPredicate;

use super::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::lexer::{OwnedLexToken, TokenWordView};
pub(crate) use super::util::{
    find_activation_cost_start, non_article_word_refs, replace_unbound_x_with_value,
    starts_with_activation_cost, value_contains_unbound_x,
};

const RESULT_VERB_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["remove"],
            &["removed"],
            &["sacrifice"],
            &["sacrificed"],
            &["discard"],
            &["discarded"],
            &["exile"],
            &["exiled"],
        ]
);
const THIS_WAY_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["this", "way"]);
const RESULT_QUALIFIER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&[], &["it"], &["them"], &["that"]]);
const CONTRACTED_NEGATION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["dont"], &["doesnt"], &["didnt"], &["cant"]]);
const SPLIT_NEGATION_FIRST_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["do"], &["does"], &["did"], &["can"]]);
const NOT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not"]);
const YOU_DO_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you", "do"]);
const THEY_DO_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["they", "do"]);
const PLAYER_DOES_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["player", "do"],
            &["player", "does"],
            &["players", "do"],
            &["players", "does"]
        ]
);
const YOU_WIN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "win"], &["you", "won"]]);
const CLASH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["clash"]);
const YOU_SEARCHED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "searched"]; suffix & ["this", "way"]);
const PLAYER_DEALT_DAMAGE_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "player", "is", "dealt", "damage", "this", "way"],
            &["player", "is", "dealt", "damage", "this", "way"],
        ]
);
const SPELL_COUNTERED_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any & [&["that", "spell"], &["it", "spell"]];
    suffix & ["this", "way"];
    contains_words & ["countered"]
);
const DIES_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "creature", "dies", "this", "way"],
            &["that", "permanent", "dies", "this", "way"],
            &["that", "card", "dies", "this", "way"],
            &["it", "creature", "dies", "this", "way"],
            &["it", "permanent", "dies", "this", "way"],
            &["it", "card", "dies", "this", "way"],
        ]
);
const WOULD_DIE_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "creature", "dealt", "damage", "this", "way", "would", "die", "this", "turn"
            ],
            &[
                "permanent",
                "dealt",
                "damage",
                "this",
                "way",
                "would",
                "die",
                "this",
                "turn"
            ],
            &[
                "card", "dealt", "damage", "this", "way", "would", "die", "this", "turn"
            ],
        ]
);
const EXCESS_DAMAGE_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["it", "deals", "excess", "damage", "this", "way"]);
const POWER_BECOMES_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(prefix_any & [&["its", "power", "becomes"], &["it", "power", "becomes"]]; suffix & ["this", "way"]);

fn modal_words_match_shape(words: &[&str], shape: &ClauseShape<'static>) -> bool {
    shape.matches_words(words)
}

pub(crate) fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    let word_view = TokenWordView::new(tokens);
    let raw_words = word_view.to_word_refs();
    let words = non_article_word_refs(&raw_words);
    let is_result_verb = |word: &str| RESULT_VERB_WORD_PATTERN.matches_word(word);
    let is_unqualified_this_way_result = |subject: &str| {
        if words.len() < 4
            || words.first().copied() != Some(subject)
            || !is_result_verb(words[1])
            || !modal_words_match_shape(&words, &THIS_WAY_SUFFIX_PATTERN)
        {
            return false;
        }
        let qualifiers = &words[2..words.len() - 2];
        modal_words_match_shape(qualifiers, &RESULT_QUALIFIER_PATTERN)
    };
    let is_exact_negated_result = |subject: &str| {
        (words.len() == 2
            && words.first().copied() == Some(subject)
            && CONTRACTED_NEGATION_WORD_PATTERN.matches_word(words[1]))
            || (words.len() == 3
                && words.first().copied() == Some(subject)
                && SPLIT_NEGATION_FIRST_WORD_PATTERN.matches_word(words[1])
                && NOT_WORD_PATTERN.matches_word(words[2]))
    };
    let is_negated_this_way_result = |subject: &str| {
        let action_idx = if words.len() >= 5
            && words.first().copied() == Some(subject)
            && CONTRACTED_NEGATION_WORD_PATTERN.matches_word(words[1])
        {
            2
        } else if words.len() >= 6
            && words.first().copied() == Some(subject)
            && SPLIT_NEGATION_FIRST_WORD_PATTERN.matches_word(words[1])
            && NOT_WORD_PATTERN.matches_word(words[2])
        {
            3
        } else {
            return false;
        };
        if !is_result_verb(words[action_idx])
            || !modal_words_match_shape(&words, &THIS_WAY_SUFFIX_PATTERN)
        {
            return false;
        }
        let qualifiers = &words[action_idx + 1..words.len() - 2];
        modal_words_match_shape(qualifiers, &RESULT_QUALIFIER_PATTERN)
    };

    if words.is_empty() {
        None
    } else if is_unqualified_this_way_result("if") || is_exact_negated_result("if") {
        Some(IfResultPredicate::Did)
    } else if is_negated_this_way_result("if") {
        Some(IfResultPredicate::DidNot)
    } else if is_unqualified_this_way_result("when") || is_exact_negated_result("when") {
        Some(IfResultPredicate::Did)
    } else if is_negated_this_way_result("when") {
        Some(IfResultPredicate::DidNot)
    } else {
        None
    }
}

pub(crate) fn parse_if_result_predicate_lexed(
    tokens: &[OwnedLexToken],
) -> Option<IfResultPredicate> {
    let word_view = TokenWordView::new(tokens);
    let raw_words = word_view.to_word_refs();
    let words = non_article_word_refs(&raw_words);
    let is_result_verb = |word: &str| RESULT_VERB_WORD_PATTERN.matches_word(word);
    let is_unqualified_this_way_result = |subject: &str| {
        if words.len() < 4
            || words.first().copied() != Some(subject)
            || !is_result_verb(words[1])
            || !modal_words_match_shape(&words, &THIS_WAY_SUFFIX_PATTERN)
        {
            return false;
        }
        let qualifiers = &words[2..words.len() - 2];
        modal_words_match_shape(qualifiers, &RESULT_QUALIFIER_PATTERN)
    };
    let is_exact_negated_result = |subject: &str| {
        (words.len() == 2
            && words.first().copied() == Some(subject)
            && CONTRACTED_NEGATION_WORD_PATTERN.matches_word(words[1]))
            || (words.len() == 3
                && words.first().copied() == Some(subject)
                && SPLIT_NEGATION_FIRST_WORD_PATTERN.matches_word(words[1])
                && NOT_WORD_PATTERN.matches_word(words[2]))
    };
    let is_negated_this_way_result = |subject: &str| {
        let action_idx = if words.len() >= 5
            && words.first().copied() == Some(subject)
            && CONTRACTED_NEGATION_WORD_PATTERN.matches_word(words[1])
        {
            2
        } else if words.len() >= 6
            && words.first().copied() == Some(subject)
            && SPLIT_NEGATION_FIRST_WORD_PATTERN.matches_word(words[1])
            && NOT_WORD_PATTERN.matches_word(words[2])
        {
            3
        } else {
            return false;
        };
        if !is_result_verb(words[action_idx])
            || !modal_words_match_shape(&words, &THIS_WAY_SUFFIX_PATTERN)
        {
            return false;
        }
        let qualifiers = &words[action_idx + 1..words.len() - 2];
        modal_words_match_shape(qualifiers, &RESULT_QUALIFIER_PATTERN)
    };

    if modal_words_match_shape(&words, &YOU_DO_PATTERN) {
        return Some(IfResultPredicate::Did);
    }
    if modal_words_match_shape(&words, &YOU_WIN_PREFIX_PATTERN)
        && (words.len() == 2 || modal_words_match_shape(&words, &CLASH_WORD_PATTERN))
    {
        return Some(IfResultPredicate::Value(
            crate::effect::Comparison::GreaterThan(0),
        ));
    }
    if modal_words_match_shape(&words, &THEY_DO_PATTERN) {
        return Some(IfResultPredicate::Did);
    }
    if modal_words_match_shape(&words, &PLAYER_DOES_PATTERN) {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 6 && modal_words_match_shape(&words, &YOU_SEARCHED_THIS_WAY_PATTERN) {
        return Some(IfResultPredicate::Did);
    }
    if is_unqualified_this_way_result("you") {
        return Some(IfResultPredicate::Did);
    }
    if is_unqualified_this_way_result("they") {
        return Some(IfResultPredicate::Did);
    }
    if modal_words_match_shape(&words, &PLAYER_DEALT_DAMAGE_THIS_WAY_PATTERN) {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5 && modal_words_match_shape(&words, &SPELL_COUNTERED_THIS_WAY_PATTERN) {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5 && modal_words_match_shape(&words, &DIES_THIS_WAY_PATTERN) {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if words.len() >= 8 && modal_words_match_shape(&words, &WOULD_DIE_THIS_TURN_PATTERN) {
        return Some(IfResultPredicate::DiesThisWay);
    }

    if modal_words_match_shape(&words, &EXCESS_DAMAGE_THIS_WAY_PATTERN)
        || (words.len() == 5 && modal_words_match_shape(&words, &POWER_BECOMES_THIS_WAY_PATTERN))
    {
        return Some(IfResultPredicate::Did);
    }

    if is_exact_negated_result("you") || is_negated_this_way_result("you") {
        return Some(IfResultPredicate::DidNot);
    }
    if is_exact_negated_result("they") || is_negated_this_way_result("they") {
        return Some(IfResultPredicate::DidNot);
    }
    if is_exact_negated_result("player")
        || is_negated_this_way_result("player")
        || is_exact_negated_result("players")
        || is_negated_this_way_result("players")
    {
        return Some(IfResultPredicate::DidNot);
    }

    None
}
