#![allow(dead_code, unused_imports)]

use crate::cards::builders::IfResultPredicate;

pub(crate) use super::util::{
    find_activation_cost_start, replace_unbound_x_with_value, starts_with_activation_cost,
    value_contains_unbound_x,
};
use super::{LowercaseWordView, lexer::OwnedLexToken};

pub(crate) fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    let words: Vec<&str> = super::util::words(tokens)
        .into_iter()
        .filter(|word| !super::util::is_article(word))
        .collect();
    let is_result_verb = |word: &str| {
        matches!(
            word,
            "remove"
                | "removed"
                | "sacrifice"
                | "sacrificed"
                | "discard"
                | "discarded"
                | "exile"
                | "exiled"
        )
    };
    let is_unqualified_this_way_result = |subject: &str| {
        if words.len() < 4
            || words[0] != subject
            || !is_result_verb(words[1])
            || words[words.len() - 2] != "this"
            || words[words.len() - 1] != "way"
        {
            return false;
        }
        let qualifiers = &words[2..words.len() - 2];
        matches!(qualifiers, [] | ["it"] | ["them"] | ["that"])
    };
    let is_exact_negated_result = |subject: &str| {
        (words.len() == 2 && words[0] == subject && matches!(words[1], "dont" | "didnt" | "cant"))
            || (words.len() == 3
                && words[0] == subject
                && (matches!(words[1], "do" | "did" | "can") && words[2] == "not"))
    };
    let is_negated_this_way_result = |subject: &str| {
        let action_idx =
            if words.len() >= 5 && words[0] == subject && matches!(words[1], "dont" | "didnt") {
                2
            } else if words.len() >= 6
                && words[0] == subject
                && ((words[1] == "do" && words[2] == "not")
                    || (words[1] == "did" && words[2] == "not"))
            {
                3
            } else {
                return false;
            };
        if !is_result_verb(words[action_idx])
            || words[words.len() - 2] != "this"
            || words[words.len() - 1] != "way"
        {
            return false;
        }
        let qualifiers = &words[action_idx + 1..words.len() - 2];
        matches!(qualifiers, [] | ["it"] | ["them"] | ["that"])
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
    let word_view = LowercaseWordView::new(tokens);
    let words: Vec<&str> = word_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !super::util::is_article(word))
        .collect();
    let is_result_verb = |word: &str| {
        matches!(
            word,
            "remove"
                | "removed"
                | "sacrifice"
                | "sacrificed"
                | "discard"
                | "discarded"
                | "exile"
                | "exiled"
        )
    };
    let is_unqualified_this_way_result = |subject: &str| {
        if words.len() < 4
            || words[0] != subject
            || !is_result_verb(words[1])
            || words[words.len() - 2] != "this"
            || words[words.len() - 1] != "way"
        {
            return false;
        }
        let qualifiers = &words[2..words.len() - 2];
        matches!(qualifiers, [] | ["it"] | ["them"] | ["that"])
    };
    let is_exact_negated_result = |subject: &str| {
        (words.len() == 2 && words[0] == subject && matches!(words[1], "dont" | "didnt" | "cant"))
            || (words.len() == 3
                && words[0] == subject
                && (matches!(words[1], "do" | "did" | "can") && words[2] == "not"))
    };
    let is_negated_this_way_result = |subject: &str| {
        let action_idx =
            if words.len() >= 5 && words[0] == subject && matches!(words[1], "dont" | "didnt") {
                2
            } else if words.len() >= 6
                && words[0] == subject
                && ((words[1] == "do" && words[2] == "not")
                    || (words[1] == "did" && words[2] == "not"))
            {
                3
            } else {
                return false;
            };
        if !is_result_verb(words[action_idx])
            || words[words.len() - 2] != "this"
            || words[words.len() - 1] != "way"
        {
            return false;
        }
        let qualifiers = &words[action_idx + 1..words.len() - 2];
        matches!(qualifiers, [] | ["it"] | ["them"] | ["that"])
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
