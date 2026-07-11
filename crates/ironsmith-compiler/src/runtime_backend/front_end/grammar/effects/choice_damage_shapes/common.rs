use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::primitives::{self, WordSliceInput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChoiceDamageScope {
    Opponent,
    Player,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AlternateDamageTargetShape {
    Them,
    ThatPlayer,
}

fn parse_phrase_words(input: &mut WordSliceInput<'_>, expected: &[&'static str]) -> bool {
    let mut probe = *input;
    for word in expected {
        if primitives::word_slice_exact(word)
            .parse_next(&mut probe)
            .is_err()
        {
            return false;
        }
    }
    *input = probe;
    true
}

fn phrase_occurs(words: &[&str], expected: &[&'static str]) -> bool {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        if parse_phrase_words(&mut input, expected) {
            return true;
        }
        offset += 1;
    }
    false
}

fn exact_phrase(words: &[&str], expected: &[&'static str]) -> bool {
    let mut input = words;
    parse_phrase_words(&mut input, expected) && input.is_empty()
}

fn word_occurs(words: &[&str], expected: &'static str) -> bool {
    phrase_occurs(words, &[expected])
}

pub(crate) fn first_choice_damage_word_is(words: &[&str], expected: &'static str) -> bool {
    let mut input = words;
    primitives::word_slice_exact(expected)
        .parse_next(&mut input)
        .is_ok()
}

pub(crate) fn parse_alternate_damage_target_shape(
    words: &[&str],
) -> Option<AlternateDamageTargetShape> {
    if exact_phrase(words, &["them"]) {
        Some(AlternateDamageTargetShape::Them)
    } else if exact_phrase(words, &["that", "player"]) {
        Some(AlternateDamageTargetShape::ThatPlayer)
    } else {
        None
    }
}

pub(crate) fn is_choice_damage_source_subject_shape(words: &[&str]) -> bool {
    exact_phrase(words, &["this", "aura"])
        || exact_phrase(words, &["this", "permanent"])
        || exact_phrase(words, &["this", "enchantment"])
}

pub(crate) fn is_that_player_target_shape(words: &[&str]) -> bool {
    exact_phrase(words, &["that", "player"])
}

pub(crate) fn is_choice_damage_drain_shape(words: &[&str]) -> bool {
    (phrase_occurs(words, &["lose", "x", "life"]) || phrase_occurs(words, &["loses", "x", "life"]))
        && phrase_occurs(words, &["you", "gain", "x", "life"])
}

pub(crate) fn is_random_card_descriptor_shape(words: &[&str]) -> bool {
    word_occurs(words, "card") && phrase_occurs(words, &["at", "random"])
}

pub(crate) fn is_create_token_sacrifice_counter_shape(words: &[&str]) -> bool {
    first_choice_damage_word_is(words, "create")
        && word_occurs(words, "token")
        && word_occurs(words, "sacrifice")
        && word_occurs(words, "counter")
}

pub(crate) fn is_up_to_one_target_shape(words: &[&str]) -> bool {
    exact_phrase(words, &["up", "to", "one", "target"])
}

pub(crate) fn is_card_noun_at(words: &[&str], offset: usize) -> bool {
    words
        .get(offset)
        .is_some_and(|word| matches!(*word, "card" | "cards"))
}

pub(crate) fn is_reveal_article_word(word: &str) -> bool {
    matches!(word, "a" | "an" | "one")
}

pub(crate) fn is_damage_word(word: &str) -> bool {
    word == "damage"
}

pub(crate) fn parse_choice_damage_scope(words: &[&str]) -> Option<ChoiceDamageScope> {
    let opponent = [
        &["for", "each", "opponent"][..],
        &["for", "each", "opponents"][..],
        &["each", "opponent"][..],
        &["each", "opponents"][..],
    ];
    for prefix in opponent {
        let mut input = words;
        if parse_phrase_words(&mut input, prefix) {
            return Some(ChoiceDamageScope::Opponent);
        }
    }
    let mut input = words;
    parse_phrase_words(&mut input, &["each", "player"]).then_some(ChoiceDamageScope::Player)
}

pub(crate) fn has_leading_to_shape(words: &[&str]) -> bool {
    first_choice_damage_word_is(words, "to")
}

pub(crate) fn has_all_or_each_at(words: &[&str], offset: usize) -> bool {
    words
        .get(offset)
        .is_some_and(|word| matches!(*word, "all" | "each"))
}

pub(crate) fn has_choice_damage_condition_boundary(words: &[&str]) -> bool {
    let mut offset = 0usize;
    while offset < words.len() {
        if words.get(offset).is_some_and(|word| {
            matches!(
                *word,
                "if" | "unless" | "then" | "where" | "when" | "whenever"
            )
        }) {
            return true;
        }
        offset += 1;
    }
    false
}

pub(crate) fn has_if_or_unless_shape(words: &[&str]) -> bool {
    word_occurs(words, "if") || word_occurs(words, "unless")
}

pub(crate) fn has_unless_shape(words: &[&str]) -> bool {
    word_occurs(words, "unless")
}

pub(crate) fn is_that_controller_has_shape(words: &[&str]) -> bool {
    if !first_choice_damage_word_is(words, "that") {
        return false;
    }
    let mut has_offset = 1usize;
    while has_offset < words.len() {
        let Some(word) = words.get(has_offset) else {
            return false;
        };
        if matches!(*word, "has" | "have") {
            return words[1..has_offset]
                .iter()
                .copied()
                .any(|word| matches!(word, "controller" | "controllers"));
        }
        has_offset += 1;
    }
    false
}

pub(crate) fn is_hand_reference_shape(words: &[&str]) -> bool {
    exact_phrase(words, &["their", "hand"])
        || exact_phrase(words, &["their", "hands"])
        || exact_phrase(words, &["your", "hand"])
        || exact_phrase(words, &["your", "hands"])
        || exact_phrase(words, &["that", "player", "hand"])
        || exact_phrase(words, &["that", "player", "hands"])
        || exact_phrase(words, &["target", "player", "hand"])
        || exact_phrase(words, &["target", "player", "hands"])
}

pub(crate) fn is_likely_named_or_source_reference_shape(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if crate::runtime_backend::front_end::shared::util::is_source_reference_words(words) {
        return true;
    }
    let mut offset = 0usize;
    while offset < words.len() {
        let Some(word) = words.get(offset) else {
            return false;
        };
        if matches!(
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
        ) {
            return false;
        }
        if matches!(
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
        ) {
            return false;
        }
        offset += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_targets_scopes_and_clause_facts() {
        assert_eq!(
            parse_alternate_damage_target_shape(&["that", "player"]),
            Some(AlternateDamageTargetShape::ThatPlayer)
        );
        assert_eq!(
            parse_choice_damage_scope(&["for", "each", "opponent", "draws"]),
            Some(ChoiceDamageScope::Opponent)
        );
        assert!(is_choice_damage_drain_shape(&[
            "loses", "x", "life", "and", "you", "gain", "x", "life",
        ]));
        assert!(is_hand_reference_shape(&["target", "player", "hand"]));
    }
}
