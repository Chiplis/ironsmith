//! "Devour [quality] N" (CR 702.82c) and "Devour X, where X is the number of
//! creatures devoured this way" (Thromok the Insatiable). Plain "Devour N"
//! stays a keyword action; these variants carry a sacrifice quality or a
//! devoured-count multiplier that the keyword action cannot express.

use super::*;

const DEVOURED_COUNT_TAIL: &[&str] = &[
    "x", "where", "x", "is", "the", "number", "of", "creatures", "devoured", "this", "way",
];

pub fn parse_devour_quality_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("devour")) {
        return Ok(None);
    }
    let body = &tokens[1..];
    let body_words = parser_token_word_refs(body);
    let multiplier_is_devoured_count =
        crate::word_primitives::parse_sequence_complete(&body_words, DEVOURED_COUNT_TAIL);
    let (multiplier, quality) = if multiplier_is_devoured_count {
        (1, None)
    } else {
        // "Devour <quality> <N>": at least one quality word and a final
        // count; the bare "Devour N" keyword is not this rule's.
        let Some((count_token, quality_tokens)) = body.split_last() else {
            return Ok(None);
        };
        if quality_tokens.is_empty() {
            return Ok(None);
        }
        let Some(multiplier) = parse_number_word_i32(count_token.parser_text())
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
        else {
            return Ok(None);
        };
        let quality = parse_object_filter_lexed(quality_tokens, false)?;
        if quality.card_types.is_empty() && quality.subtypes.is_empty() {
            return Ok(None);
        }
        (multiplier, Some(quality))
    };
    Ok(Some(StaticAbilityAst::KeywordAction(
        KeywordAction::DevourVariant {
            multiplier,
            quality,
            multiplier_is_devoured_count,
            presentation_multiplier: multiplier,
        },
    )))
}
