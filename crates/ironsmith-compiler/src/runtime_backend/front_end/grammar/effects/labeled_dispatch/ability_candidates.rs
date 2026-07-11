use crate::runtime_backend::front_end::lexer::{OwnedLexToken, parser_token_word_refs};
use winnow::Parser as _;
use winnow::combinator::alt;

use super::super::super::{leaf, primitives};
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
pub(crate) struct AbilityCandidateShape {
    pub(crate) simple_source_gain: bool,
    pub(crate) simple_gain: bool,
}

fn is_source_reference(words: &[&str]) -> bool {
    leaf::parse_leaf_this_source_reference_words(words).is_some()
        || crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(
            words,
        )
        .is_some()
}

fn simple_source_gain(words: &[&str]) -> bool {
    let Some(gain_idx) = common::first_word_offset_any(words, GAIN_WORDS) else {
        return false;
    };
    gain_idx > 0
        && is_source_reference(&words[..gain_idx])
        && !common::present_any(&words[gain_idx + 1..], TAIL_STOP_WORDS)
}

fn simple_gain(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    let Some(gain_idx) = common::first_word_offset_any(words, GAIN_HAS_LOSE_WORDS) else {
        return false;
    };
    let ability_words = &words[gain_idx + 1..];
    if matches!(words.get(gain_idx), Some(&("has" | "have")))
        && !common::present_any(ability_words, TRIGGER_WORDS)
    {
        return false;
    }
    if common::present_any(&words[..gain_idx], SUBJECT_EXCLUSION_WORDS)
        || (common::present(words, &["another"]) && common::present(words, &["haste"]))
    {
        return false;
    }
    let has_quoted_or_activated_ability = primitives::find_prefix(tokens, || {
        alt((primitives::quote(), primitives::colon())).void()
    })
    .is_some();
    !ability_words.is_empty()
        && !common::present(ability_words, &["life"])
        && (common::present_any(ability_words, SIMPLE_ABILITY_WORDS)
            || has_quoted_or_activated_ability)
}

pub(crate) fn parse_ability_candidate_shape(tokens: &[OwnedLexToken]) -> AbilityCandidateShape {
    let words = parser_token_word_refs(tokens);
    AbilityCandidateShape {
        simple_source_gain: simple_source_gain(&words),
        simple_gain: simple_gain(tokens, &words),
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::front_end::lexer::lex_line;

    use super::*;

    fn shape(text: &str) -> AbilityCandidateShape {
        parse_ability_candidate_shape(&lex_line(text, 0).expect("lex fixture"))
    }

    #[test]
    fn classifies_source_and_target_ability_grant_candidates() {
        assert!(shape("This creature gains flying until end of turn.").simple_source_gain);
        assert!(shape("Target creature gains flying until end of turn.").simple_gain);
        assert!(!shape("You gain 3 life.").simple_gain);
        assert!(!shape("Another target creature gains haste.").simple_gain);
    }
}
