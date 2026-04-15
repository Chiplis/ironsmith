use super::super::super::grammar::primitives as grammar;
use super::super::{parse_effect_sentence_lexed, parse_restriction_duration, trim_commas};
use crate::cards::builders::{CardTextError, EffectAst};

const THEY_DONT_UNTAP_DURING_PREFIXES: &[&[&str]] = &[
    &["they", "dont", "untap", "during"],
    &["they", "don't", "untap", "during"],
    &["those", "permanents", "dont", "untap", "during"],
    &["those", "permanents", "don't", "untap", "during"],
];

pub(super) fn try_parse(
    first: &[crate::cards::builders::OwnedLexToken],
    second: &[crate::cards::builders::OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::TapAll { filter }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let starts_with_supported_pronoun_clause =
        grammar::words_match_any_prefix(&second_tokens, THEY_DONT_UNTAP_DURING_PREFIXES).is_some();
    let has_source_tapped_duration =
        grammar::words_find_phrase(&second_tokens, &["for", "as", "long", "as"]).is_some()
            && grammar::contains_word(&second_tokens, "remains")
            && grammar::contains_word(&second_tokens, "tapped")
            && (grammar::contains_word(&second_tokens, "this")
                || grammar::contains_word(&second_tokens, "thiss")
                || grammar::contains_word(&second_tokens, "source")
                || grammar::contains_word(&second_tokens, "artifact")
                || grammar::contains_word(&second_tokens, "creature")
                || grammar::contains_word(&second_tokens, "permanent"));
    if !starts_with_supported_pronoun_clause || !has_source_tapped_duration {
        return Ok(None);
    }

    let Some((duration, clause_tokens)) = parse_restriction_duration(&second_tokens)? else {
        return Ok(None);
    };
    let valid_untap_clause =
        grammar::words_match_any_prefix(&clause_tokens, THEY_DONT_UNTAP_DURING_PREFIXES).is_some();
    if !valid_untap_clause {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::TapAll {
            filter: filter.clone(),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::untap(filter.clone()),
            duration,
            condition: Some(crate::ConditionExpr::SourceIsTapped),
        },
    ]))
}
