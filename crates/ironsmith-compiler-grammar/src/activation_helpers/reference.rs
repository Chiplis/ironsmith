use super::*;

pub(super) fn parse_add_one_mana_any_color_among_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(span) = activation_grammar::parse_any_color_among_span(tokens) else {
        return Ok(None);
    };
    Ok(Some(parse_object_filter(span.filter_tokens, false)?))
}

pub fn parse_land_could_produce_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, crate::effects::ManaTypeSource)>, CardTextError> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let Some(shape) = activation_grammar::parse_land_could_produce_shape(tokens) else {
        return Ok(None);
    };
    let (filter_tokens, mana_type_source) = match shape {
        activation_grammar::LandCouldProduceShape::CouldProduceFilter(filter_tokens) => (
            filter_tokens,
            crate::effects::ManaTypeSource::MatchingLandsCouldProduce,
        ),
        activation_grammar::LandCouldProduceShape::TriggeringEventProducedFilter(filter_tokens) => {
            (
                filter_tokens,
                crate::effects::ManaTypeSource::TriggeringEventProduced,
            )
        }
        activation_grammar::LandCouldProduceShape::UnsupportedTrailing => {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (tail: '{}')",
                words.join(" ")
            )));
        }
    };
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing land filter in mana clause (tail: '{}')",
            words.join(" ")
        )));
    }
    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some((filter, mana_type_source)))
}
