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
        && !gains_life_outside_quotes(tokens)
        && (common::present_any(ability_words, SIMPLE_ABILITY_WORDS)
            || has_quoted_or_activated_ability)
}

/// Whether "life" follows the sentence's own gain/has/lose verb outside any
/// quoted ability. A life amount inside a granted quote ("gains \"When this
/// creature dies, you gain 1 life\"") belongs to the granted ability, not to
/// the outer sentence, so it must not disqualify the ability grant.
fn gains_life_outside_quotes(tokens: &[OwnedLexToken]) -> bool {
    let mut quoted = false;
    let mut unquoted_words = Vec::new();
    for token in tokens {
        if token.is_quote() {
            quoted = !quoted;
            continue;
        }
        if !quoted {
            unquoted_words.extend(parser_token_word_refs(std::slice::from_ref(token)));
        }
    }
    common::first_word_offset_any(&unquoted_words, GAIN_HAS_LOSE_WORDS)
        .is_some_and(|gain_idx| common::present(&unquoted_words[gain_idx + 1..], &["life"]))
}

pub fn parse_ability_candidate_shape(tokens: &[OwnedLexToken]) -> AbilityCandidateShape {
    let words = parser_token_word_refs(tokens);
    AbilityCandidateShape {
        simple_source_gain: simple_source_gain(&words),
        simple_gain: simple_gain(tokens, &words),
    }
}
