//! "Creatures your opponents control have base toughness 1." (Maha, Its
//! Feathers Night): a layer-7b toughness-only base setting (CR 613.4b).

use super::*;

pub fn parse_base_toughness_only_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some(have_idx) = tokens
        .iter()
        .position(|token| token.is_any_word(&["have", "has"]))
    else {
        return Ok(None);
    };
    let [base, toughness, value] = tokens.get(have_idx + 1..).unwrap_or_default() else {
        return Ok(None);
    };
    if have_idx == 0 || !base.is_word("base") || !toughness.is_word("toughness") {
        return Ok(None);
    }
    let Some(toughness_value) = parse_number_word_i32(value.parser_text()) else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..have_idx];
    if subject_tokens.iter().any(|token| token.is_word("target")) {
        return Ok(None);
    }
    let subject = parse_anthem_subject(subject_tokens)?;
    Ok(Some(StaticAbility::set_base_toughness(
        anthem_subject_filter(&subject),
        toughness_value,
    )))
}
