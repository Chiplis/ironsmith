use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::leaf;
use crate::runtime_backend::front_end::grammar::primitives::{self, WordSliceInput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GainAbilityVerb {
    Gain,
    Lose,
    Has,
    Get,
    Become,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SharedAbilityTail {
    Gain,
    Lose,
    Has,
    Get,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AbilityReferenceSurface {
    ThisAbility,
    AllAbilities,
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GainSubjectShape {
    pub(crate) demonstrative_object: bool,
    pub(crate) demonstrative_player: bool,
    pub(crate) pronoun: bool,
    pub(crate) tagged_pronoun: bool,
    pub(crate) target: bool,
    pub(crate) controller_tail: bool,
    pub(crate) player_any: bool,
    pub(crate) player_you: bool,
    pub(crate) you_and_permanents: bool,
    pub(crate) source_subject: bool,
}

fn gain_word(input: &mut WordSliceInput<'_>) -> WResult<GainAbilityVerb> {
    alt((
        primitives::word_slice_exact("gain").value(GainAbilityVerb::Gain),
        primitives::word_slice_exact("gains").value(GainAbilityVerb::Gain),
    ))
    .parse_next(input)
}

fn lose_word(input: &mut WordSliceInput<'_>) -> WResult<GainAbilityVerb> {
    alt((
        primitives::word_slice_exact("lose").value(GainAbilityVerb::Lose),
        primitives::word_slice_exact("loses").value(GainAbilityVerb::Lose),
    ))
    .parse_next(input)
}

fn has_word(input: &mut WordSliceInput<'_>) -> WResult<GainAbilityVerb> {
    alt((
        primitives::word_slice_exact("has").value(GainAbilityVerb::Has),
        primitives::word_slice_exact("have").value(GainAbilityVerb::Has),
    ))
    .parse_next(input)
}

fn get_word(input: &mut WordSliceInput<'_>) -> WResult<GainAbilityVerb> {
    alt((
        primitives::word_slice_exact("get").value(GainAbilityVerb::Get),
        primitives::word_slice_exact("gets").value(GainAbilityVerb::Get),
    ))
    .parse_next(input)
}

fn become_word(input: &mut WordSliceInput<'_>) -> WResult<GainAbilityVerb> {
    alt((
        primitives::word_slice_exact("become").value(GainAbilityVerb::Become),
        primitives::word_slice_exact("becomes").value(GainAbilityVerb::Become),
    ))
    .parse_next(input)
}

#[derive(Clone, Copy)]
enum VerbSearch {
    GainLoseHas,
    Gain,
    Lose,
    Get,
    Become,
}

fn first_matching_verb(words: &[&str], search: VerbSearch) -> Option<(usize, GainAbilityVerb)> {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        let verb = match search {
            VerbSearch::GainLoseHas => alt((gain_word, lose_word, has_word))
                .parse_next(&mut input)
                .ok(),
            VerbSearch::Gain => gain_word.parse_next(&mut input).ok(),
            VerbSearch::Lose => lose_word.parse_next(&mut input).ok(),
            VerbSearch::Get => get_word.parse_next(&mut input).ok(),
            VerbSearch::Become => become_word.parse_next(&mut input).ok(),
        };
        if let Some(verb) = verb {
            return Some((offset, verb));
        }
        offset += 1;
    }
    None
}

pub(crate) fn find_gain_ability_verb(words: &[&str]) -> Option<(usize, GainAbilityVerb)> {
    first_matching_verb(words, VerbSearch::GainLoseHas)
}

pub(crate) fn find_primary_gain_ability_verb(words: &[&str]) -> Option<(usize, GainAbilityVerb)> {
    let (offset, verb) = find_gain_ability_verb(words)?;
    if verb != GainAbilityVerb::Has {
        return Some((offset, verb));
    }
    let after_has = words.get(offset + 1..)?;
    let mut base_input = after_has;
    let has_base_pt = (
        primitives::word_slice_exact("base"),
        primitives::word_slice_exact("power"),
        primitives::word_slice_exact("and"),
        primitives::word_slice_exact("toughness"),
    )
        .parse_next(&mut base_input)
        .is_ok();
    if !has_base_pt {
        return Some((offset, verb));
    }
    let Some((tail, verb)) = find_shared_ability_tail(after_has, SharedAbilityTail::Gain)
        .map(|tail| (tail, GainAbilityVerb::Gain))
        .or_else(|| {
            find_shared_ability_tail(after_has, SharedAbilityTail::Lose)
                .map(|tail| (tail, GainAbilityVerb::Lose))
        })
    else {
        return Some((offset, GainAbilityVerb::Has));
    };
    Some((offset + 1 + tail + 1, verb))
}

pub(crate) fn find_gain_or_lose_verb(
    words: &[&str],
    losing: bool,
) -> Option<(usize, GainAbilityVerb)> {
    if losing {
        first_matching_verb(words, VerbSearch::Lose)
    } else {
        first_matching_verb(words, VerbSearch::Gain)
    }
}

pub(crate) fn find_get_verb(words: &[&str]) -> Option<usize> {
    first_matching_verb(words, VerbSearch::Get).map(|(offset, _)| offset)
}

pub(crate) fn find_become_verb(words: &[&str]) -> Option<usize> {
    first_matching_verb(words, VerbSearch::Become).map(|(offset, _)| offset)
}

fn shared_tail_parser(input: &mut WordSliceInput<'_>) -> WResult<SharedAbilityTail> {
    primitives::word_slice_exact("and").parse_next(input)?;
    alt((
        gain_word.value(SharedAbilityTail::Gain),
        lose_word.value(SharedAbilityTail::Lose),
        has_word.value(SharedAbilityTail::Has),
        get_word.value(SharedAbilityTail::Get),
    ))
    .parse_next(input)
}

pub(crate) fn find_shared_ability_tail(
    words: &[&str],
    expected: SharedAbilityTail,
) -> Option<usize> {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        if shared_tail_parser
            .parse_next(&mut input)
            .is_ok_and(|parsed| parsed == expected)
        {
            return Some(offset);
        }
        offset += 1;
    }
    None
}

fn word_present(words: &[&str], expected: &'static str) -> bool {
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        if primitives::word_slice_exact(expected)
            .parse_next(&mut input)
            .is_ok()
        {
            return true;
        }
        offset += 1;
    }
    false
}

pub(crate) fn gain_words_include_token_noun(words: &[&str]) -> bool {
    word_present(words, "token") || word_present(words, "tokens")
}

pub(crate) fn gain_words_include_target(words: &[&str]) -> bool {
    word_present(words, "target")
}

pub(crate) fn gain_words_include_control_verb(words: &[&str]) -> bool {
    word_present(words, "control") || word_present(words, "controls")
}

pub(crate) fn gain_word_is_connector(word: &str) -> bool {
    matches!(word, "and" | "then")
}

pub(crate) fn gain_word_is_trigger_intro(word: &str) -> bool {
    matches!(word, "when" | "whenever" | "at")
}

pub(crate) fn gain_word_is_when_intro(word: &str) -> bool {
    matches!(word, "when" | "whenever")
}

pub(crate) fn gain_word_is_pronoun(word: &str) -> bool {
    matches!(word, "it" | "they")
}

pub(crate) fn gain_word_is_source_noun(word: &str) -> bool {
    matches!(word, "creature" | "permanent" | "spell" | "card")
}

pub(crate) fn find_gain_and_separator(words: &[&str], after: usize) -> Option<usize> {
    let mut offset = after;
    while offset < words.len() {
        let mut input = &words[offset..];
        if primitives::word_slice_exact("and")
            .parse_next(&mut input)
            .is_ok()
        {
            return Some(offset);
        }
        offset += 1;
    }
    None
}

pub(crate) fn gain_verb_is_life_or_control_head(word: &str) -> bool {
    matches!(word, "life" | "control")
}

fn gain_subject_player_noun(word: &str) -> bool {
    matches!(
        word,
        "player"
            | "players"
            | "opponent"
            | "opponents"
            | "controller"
            | "controllers"
            | "owner"
            | "owners"
    )
}

fn demonstrative_gain_subject_tail<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    primitives::parse_word_sequence_prefix(words, &["that"])
        .or_else(|| primitives::parse_word_sequence_prefix(words, &["those"]))
        .or_else(|| {
            [
                &["each", "of", "that"][..],
                &["each", "of", "those"][..],
                &["all", "of", "that"][..],
                &["all", "of", "those"][..],
            ]
            .iter()
            .find_map(|prefix| primitives::parse_word_sequence_prefix(words, prefix))
        })
        .or_else(|| match words {
            ["the", noun]
                if matches!(
                    *noun,
                    "card"
                        | "copy"
                        | "creature"
                        | "object"
                        | "permanent"
                        | "spell"
                        | "token"
                ) =>
            {
                Some(&words[1..])
            }
            _ => None,
        })
}

pub(crate) fn classify_gain_subject<'a>(words: &'a [&'a str]) -> GainSubjectShape {
    let demonstrative_tail = demonstrative_gain_subject_tail(words);
    let demonstrative_player = demonstrative_tail
        .and_then(|tail| tail.first())
        .is_some_and(|noun| gain_subject_player_noun(noun));
    let demonstrative_object = !demonstrative_player && demonstrative_tail.is_some();
    GainSubjectShape {
        demonstrative_object,
        demonstrative_player,
        pronoun: primitives::parse_full_word_slice(
            words,
            alt((
                primitives::word_slice_exact("it").void(),
                primitives::word_slice_exact("they").void(),
            )),
        )
        .is_some(),
        tagged_pronoun: primitives::parse_full_word_slice(
            words,
            alt((
                primitives::word_slice_exact("it").void(),
                primitives::word_slice_exact("they").void(),
                primitives::word_slice_exact("them").void(),
            )),
        )
        .is_some(),
        target: gain_words_include_target(words),
        controller_tail: gain_words_include_control_verb(words),
        player_any: primitives::parse_full_word_slice(
            words,
            alt((
                primitives::word_slice_exact("players").void(),
                (
                    primitives::word_slice_exact("all"),
                    primitives::word_slice_exact("players"),
                )
                    .void(),
            )),
        )
        .is_some(),
        player_you: primitives::parse_full_word_slice(
            words,
            primitives::word_slice_exact("you").void(),
        )
        .is_some(),
        you_and_permanents: primitives::parse_full_word_slice(
            words,
            (
                primitives::word_slice_exact("you"),
                primitives::word_slice_exact("and"),
                primitives::word_slice_exact("permanents"),
                primitives::word_slice_exact("you"),
                primitives::word_slice_exact("control"),
            )
                .void(),
        )
        .is_some(),
        source_subject: [
            &["this"][..],
            &["this", "creature"][..],
            &["this", "permanent"][..],
        ]
        .iter()
        .any(|expected| primitives::parse_word_sequence_complete(words, expected).is_some()),
    }
}

pub(crate) fn starts_nested_triggered_ability(words: &[&str]) -> bool {
    primitives::parse_word_sequence_prefix(words, &["when"]).is_some()
        || primitives::parse_word_sequence_prefix(words, &["whenever"]).is_some()
        || primitives::parse_word_sequence_prefix(words, &["at", "the"]).is_some()
}

pub(crate) fn classify_ability_reference_surface<'a>(
    words: &'a [&'a str],
) -> AbilityReferenceSurface {
    if primitives::parse_full_word_slice(
        words,
        (
            primitives::word_slice_exact("this"),
            primitives::word_slice_exact("ability"),
        )
            .void(),
    )
    .is_some()
    {
        AbilityReferenceSurface::ThisAbility
    } else if primitives::parse_full_word_slice(
        words,
        (
            primitives::word_slice_exact("all"),
            primitives::word_slice_exact("abilities"),
        )
            .void(),
    )
    .is_some()
    {
        AbilityReferenceSurface::AllAbilities
    } else {
        AbilityReferenceSurface::Other
    }
}

pub(crate) fn is_must_attack_this_combat_tail(words: &[&str]) -> bool {
    primitives::parse_full_word_slice(
        words,
        (
            alt((
                primitives::word_slice_exact("attack"),
                primitives::word_slice_exact("attacks"),
            )),
            primitives::word_slice_exact("this"),
            primitives::word_slice_exact("combat"),
            primitives::word_slice_exact("if"),
            primitives::word_slice_exact("able"),
        )
            .void(),
    )
    .is_some()
}

fn count_prefix_start(words: &[&str], subject_offset: usize) -> usize {
    let mut candidate = subject_offset;
    while candidate > 0 {
        candidate -= 1;
        if leaf::parse_leaf_choice_count_prefix_words(&words[candidate..subject_offset])
            .is_some_and(|parsed| parsed.consumed == subject_offset - candidate)
        {
            return candidate;
        }
    }
    subject_offset
}

fn subject_start_at(words: &[&str], offset: usize) -> Option<usize> {
    let word = *words.get(offset)?;
    if gain_word_is_pronoun(word) || word == "target" {
        let mut start = count_prefix_start(words, offset);
        if start == offset && offset >= 1 && words.get(offset - 1).copied() == Some("x") {
            start = offset - 1;
        } else if offset >= 4 {
            let prefix = words.get(offset - 4..offset)?;
            let each_up_to = primitives::parse_full_word_slice(
                prefix,
                (
                    primitives::word_slice_exact("each"),
                    primitives::word_slice_exact("of"),
                    primitives::word_slice_exact("up"),
                    primitives::word_slice_exact("to"),
                )
                    .void(),
            )
            .is_some();
            if each_up_to {
                start = offset - 4;
            }
        }
        return Some(start);
    }
    if word == "this"
        && words
            .get(offset + 1)
            .is_some_and(|next| gain_word_is_source_noun(next))
    {
        return Some(offset);
    }
    None
}

pub(crate) fn find_gain_real_subject_start(words: &[&str], before_get: usize) -> usize {
    let mut offset = before_get;
    while offset > 0 {
        offset -= 1;
        if let Some(start) = subject_start_at(words, offset) {
            return start;
        }
    }
    0
}

pub(crate) fn gain_clause_is_defender_as_if_attack(words: &[&str]) -> bool {
    let mut saw_can_attack = false;
    let mut saw_as_though = false;
    let mut saw_defender = false;
    let mut offset = 0usize;
    while offset < words.len() {
        let mut input = &words[offset..];
        saw_can_attack |= (
            primitives::word_slice_exact("can"),
            primitives::word_slice_exact("attack"),
        )
            .parse_next(&mut input)
            .is_ok();
        let mut input = &words[offset..];
        saw_as_though |= (
            primitives::word_slice_exact("as"),
            primitives::word_slice_exact("though"),
        )
            .parse_next(&mut input)
            .is_ok();
        let mut input = &words[offset..];
        saw_defender |= primitives::word_slice_exact("defender")
            .parse_next(&mut input)
            .is_ok();
        offset += 1;
    }
    saw_can_attack && saw_as_though && saw_defender
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_verbs_tails_and_subjects() {
        assert_eq!(
            find_gain_ability_verb(&["target", "creature", "gains", "flying"]),
            Some((2, GainAbilityVerb::Gain))
        );
        assert_eq!(
            find_shared_ability_tail(&["flying", "and", "gets", "+1/+1"], SharedAbilityTail::Get),
            Some(1)
        );
        let subject = classify_gain_subject(&["each", "of", "those", "creatures"]);
        assert!(subject.demonstrative_object);
        assert!(!subject.demonstrative_player);

        let copy = classify_gain_subject(&["the", "copy"]);
        assert!(copy.demonstrative_object);
        assert!(!copy.demonstrative_player);
    }
}
