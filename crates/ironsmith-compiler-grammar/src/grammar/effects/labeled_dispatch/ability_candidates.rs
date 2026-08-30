use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use winnow::Parser as _;
use winnow::combinator::alt;

use super::super::super::structure::split_trailing_if_clause_lexed;
use super::super::super::{leaf, primitives};
use super::super::chain_splitting::{
    ChainVerbKind, find_chain_verb_tokens, split_effect_chain_on_and_tokens,
};
use super::common;

const GAIN_WORDS: &[&str] = &["gain", "gains"];
const GAIN_HAS_LOSE_WORDS: &[&str] = &["gain", "gains", "has", "have", "lose", "loses"];
const SIMPLE_ABILITY_WORDS: &[&[&str]] = &[
    &["indestructible"],
    &["haste"],
    &["flying"],
    &["vigilance"],
    &["lifelink"],
    &["trample"],
    &["reach"],
    &["menace"],
    &["fear"],
    &["deathtouch"],
    &["horsemanship"],
    &["hexproof"],
    &["shroud"],
    &["shadow"],
    &["strike"],
    &["protection"],
    &["blocked"],
    &["abilities"],
    &["when"],
    &["whenever"],
];
const TAIL_STOP_WORDS: &[&[&str]] = &[&["and"], &["then"], &["if"]];
const TRIGGER_WORDS: &[&[&str]] = &[&["when"], &["whenever"]];
const SUBJECT_EXCLUSION_WORDS: &[&[&str]] = &[&["shares"], &["choice"]];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityCandidateShape {
    pub simple_source_gain: bool,
    pub simple_gain: bool,
}

fn is_source_reference(words: &[&str]) -> bool {
    leaf::parse_leaf_this_source_reference_words(words).is_some()
        || crate::util::source_reference_surface_for_words(words).is_some()
}

fn simple_source_gain(words: &[&str]) -> bool {
    let Some(gain_idx) = common::first_word_offset_any(words, GAIN_WORDS) else {
        return false;
    };
    gain_idx > 0
        && is_source_reference(&words[..gain_idx])
        && !common::present_any(&words[gain_idx + 1..], TAIL_STOP_WORDS)
}

fn is_tagged_object_reference(words: &[&str]) -> bool {
    crate::word_primitives::parse_sequence_complete(words, &["it"])
        || (words.len() == 2
            && crate::word_primitives::first_is(words, "that")
            && crate::word_primitives::at_is_any(
                words,
                1,
                &[
                    "artifact",
                    "battle",
                    "card",
                    "creature",
                    "enchantment",
                    "land",
                    "object",
                    "permanent",
                    "planeswalker",
                    "spell",
                    "token",
                    "vehicle",
                ],
            ))
}

#[cfg(test)]
#[path = "ability_candidates_inline_tests.rs"]
mod tests;

#[path = "ability_candidates/ability.rs"]
mod ability_programs;
pub use ability_programs::parse_ability_candidate_shape;
use ability_programs::simple_gain;
#[path = "ability_candidates/condition.rs"]
mod condition_programs;
use condition_programs::{
    independent_action_precedes_ability_modifier,
    independent_gain_or_lose_arms_with_local_condition,
};
#[path = "ability_candidates/combat.rs"]
mod combat_programs;
use combat_programs::source_damage_then_tagged_loses_ability;
