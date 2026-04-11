use super::super::super::token_primitives::{find_index, slice_contains, slice_starts_with};
use super::super::super::util::is_article;
use super::super::{parse_effect_sentence_lexed, trim_commas};
use crate::cards::builders::{CardTextError, EffectAst};
use crate::object::CounterType;

pub(super) fn try_parse(
    first: &[crate::cards::builders::OwnedLexToken],
    second: &[crate::cards::builders::OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let (amount, target, duration) = match first_effect {
        EffectAst::PreventDamage {
            amount,
            target,
            duration,
        } => (Some(amount.clone()), target.clone(), duration.clone()),
        EffectAst::PreventAllDamageToTarget { target, duration } => {
            (None, target.clone(), duration.clone())
        }
        _ => return Ok(None),
    };

    let second_tokens = trim_commas(second);
    let second_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !slice_starts_with(
        &second_words,
        &["for", "each", "1", "damage", "prevented", "this", "way"],
    ) || !slice_contains(&second_words, &"put")
        || !slice_contains(&second_words, &"+1/+1")
        || !slice_contains(&second_words, &"counter")
        || !slice_contains(&second_words, &"on")
    {
        return Ok(None);
    }

    let Some(on_idx) = find_index(&second_words, |word| *word == "on") else {
        return Ok(None);
    };
    let target_words = &second_words[on_idx + 1..];
    let valid_target_tail = matches!(
        target_words,
        ["that", "creature"] | ["it"] | ["that", "permanent"] | ["that", "object"]
    );
    if !valid_target_tail {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::PreventDamageToTargetPutCounters {
        amount,
        target,
        duration,
        counter_type: CounterType::PlusOnePlusOne,
    }]))
}
