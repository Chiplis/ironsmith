use super::*;

pub(super) fn simple_gain(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
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

pub fn parse_ability_candidate_shape(tokens: &[OwnedLexToken]) -> AbilityCandidateShape {
    let words = parser_token_word_refs(tokens);
    AbilityCandidateShape {
        simple_source_gain: simple_source_gain(&words),
        simple_gain: simple_gain(tokens, &words),
    }
}
