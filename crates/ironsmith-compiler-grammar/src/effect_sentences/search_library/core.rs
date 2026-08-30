use super::*;

pub fn parse_earthbend_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if words
        .first()
        .is_none_or(|word| *word != SEARCH_EARTHBEND_WORD)
    {
        return Ok(None);
    }

    let count = parse_number(tokens.get(1..).unwrap_or_default())
        .map(|(value, _)| value)
        .or_else(|| words.get(1).and_then(|word| parse_number_word_u32(word)))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing earthbend count (clause: '{}')",
                words.join(" ")
            ))
        })?;

    Ok(Some(EffectAst::subject_verb_earthbend(count)))
}

pub fn parse_enchant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if words.is_empty() || words[0] != SEARCH_ENCHANT_WORD {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = match words.get(1..) {
        Some(["player"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::Any),
        Some(["opponent"]) | Some(["an", "opponent"]) => {
            crate::object::AuraAttachmentFilter::Player(PlayerFilter::Opponent)
        }
        Some(["you"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::You),
        _ => crate::object::AuraAttachmentFilter::Object(parse_object_filter(remaining, false)?),
    };
    Ok(Some(EffectAst::subject_verb_enchant(filter)))
}
