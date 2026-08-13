use crate::front_end::lexer::{OwnedLexToken, parser_token_word_refs};
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
pub(crate) struct AbilityCandidateShape {
    pub(crate) simple_source_gain: bool,
    pub(crate) simple_gain: bool,
}

fn is_source_reference(words: &[&str]) -> bool {
    leaf::parse_leaf_this_source_reference_words(words).is_some()
        || crate::util::source_reference_surface_for_words(
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

fn is_tagged_object_reference(words: &[&str]) -> bool {
    matches!(
        words,
        ["it"]
            | [
                "that",
                "artifact"
                    | "battle"
                    | "card"
                    | "creature"
                    | "enchantment"
                    | "land"
                    | "object"
                    | "permanent"
                    | "planeswalker"
                    | "spell"
                    | "token"
                    | "vehicle"
            ]
    )
}

/// A later `that creature loses flying` arm is an independent tagged action,
/// not the subject and payload of one whole-clause ability-removal sentence.
/// Keep the ability candidate route from consuming the source damage arm so
/// the ordinary coordinated-chain parser can preserve both actions.
fn source_damage_then_tagged_loses_ability(tokens: &[OwnedLexToken]) -> bool {
    let segments = split_effect_chain_on_and_tokens(tokens, true);
    let [damage_tokens, removal_tokens] = segments.as_slice() else {
        return false;
    };

    let Some(damage_verb) = find_chain_verb_tokens(damage_tokens) else {
        return false;
    };
    let damage_words = parser_token_word_refs(damage_tokens);
    if damage_verb.kind != ChainVerbKind::Deal
        || damage_verb.word_index == 0
        || !is_source_reference(&damage_words[..damage_verb.word_index])
        || !common::present(&damage_words[damage_verb.word_index + 1..], &["damage"])
        || !common::present(&damage_words[damage_verb.word_index + 1..], &["target"])
    {
        return false;
    }

    let Some(removal_verb) = find_chain_verb_tokens(removal_tokens) else {
        return false;
    };
    let removal_words = parser_token_word_refs(removal_tokens);
    removal_verb.kind == ChainVerbKind::Lose
        && removal_verb.word_index > 0
        && is_tagged_object_reference(&removal_words[..removal_verb.word_index])
        && common::present_any(
            &removal_words[removal_verb.word_index + 1..],
            SIMPLE_ABILITY_WORDS,
        )
}

/// Separate gain/loss arms with explicit subjects and a local condition must
/// be parsed as a coordinated chain. Treating the whole sentence as one
/// ability-modifier candidate makes the first arm's trailing condition share
/// a tail with the later arm, so it can no longer be consumed as a predicate.
fn independent_gain_or_lose_arms_with_local_condition(tokens: &[OwnedLexToken]) -> bool {
    let segments = split_effect_chain_on_and_tokens(tokens, true);
    if segments.len() < 2 {
        return false;
    }

    let all_independent_ability_arms = segments.iter().all(|segment| {
        find_chain_verb_tokens(segment).is_some_and(|verb| {
            verb.word_index > 0 && matches!(verb.kind, ChainVerbKind::Gain | ChainVerbKind::Lose)
        })
    });
    all_independent_ability_arms
        && segments[..segments.len() - 1]
            .iter()
            .any(|segment| split_trailing_if_clause_lexed(segment).is_some())
}

/// A later ability modifier must not claim an earlier independent action.
/// In clauses such as `you draw X cards and the chosen creatures get +X/+X
/// and gain trample`, the gain arm shares the creature subject from the pump
/// arm, while the draw remains a separate player action. Let the coordinated
/// chain parser preserve all three actions instead of routing the whole
/// sentence through the broad gain-ability parser.
fn independent_action_precedes_ability_modifier(tokens: &[OwnedLexToken]) -> bool {
    let segments = split_effect_chain_on_and_tokens(tokens, true);
    let Some(first_ability_index) = segments.iter().position(|segment| {
        common::first_word_offset_any(&parser_token_word_refs(segment), GAIN_HAS_LOSE_WORDS)
            .is_some()
    }) else {
        return false;
    };
    if first_ability_index == 0 {
        return false;
    }

    segments[..first_ability_index].iter().any(|segment| {
        find_chain_verb_tokens(segment).is_some_and(|verb| {
            verb.word_index > 0
                && !matches!(
                    verb.kind,
                    ChainVerbKind::Get | ChainVerbKind::Gain | ChainVerbKind::Lose
                )
        })
    })
}

fn simple_gain(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    if source_damage_then_tagged_loses_ability(tokens)
        || independent_gain_or_lose_arms_with_local_condition(tokens)
        || independent_action_precedes_ability_modifier(tokens)
    {
        return false;
    }
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

    #[test]
    fn rejects_source_damage_then_tagged_ability_loss_as_one_grant_candidate() {
        let shape = shape(
            "This creature deals 2 damage to target creature with flying and that creature loses flying until end of turn.",
        );
        assert!(!shape.simple_source_gain);
        assert!(!shape.simple_gain);
    }

    #[test]
    fn rejects_independent_conditioned_gain_loss_arms_as_one_grant_candidate() {
        let shape = shape(
            "Creatures your opponents control lose flying until end of turn if {G} was spent to cast this spell, and creatures you control gain flying until end of turn if {U} was spent to cast this spell.",
        );
        assert!(!shape.simple_source_gain);
        assert!(!shape.simple_gain);
    }

    #[test]
    fn rejects_draw_then_pump_and_gain_as_one_grant_candidate() {
        let shape = shape(
            "You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn, where X is the difference between the chosen creatures' powers.",
        );
        assert!(!shape.simple_source_gain);
        assert!(!shape.simple_gain);
    }
}
