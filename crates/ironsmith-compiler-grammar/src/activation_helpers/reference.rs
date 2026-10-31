use super::*;

pub(super) fn parse_add_one_mana_any_color_among_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(surface) = activation_grammar::parse_any_of_source_colors_surface(tokens) {
        // A complete "one mana of any of <source>'s colors" match.
        crate::word_claims::claim_tokens(tokens);
        return Ok(Some(ObjectFilter::source_with_surface(surface)));
    }
    let Some(span) = activation_grammar::parse_any_color_among_span(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter(span.filter_tokens, false)?;
    // "one mana of any color among" before the filter.
    crate::object_filters::claim_outside(tokens, span.filter_tokens);
    Ok(Some(filter))
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
    // The relative "that" and "could produce"/"produced" around the filter.
    crate::object_filters::claim_outside(tokens, filter_tokens);
    // "the sacrificed land" is the object paid for this ability's cost; its
    // last-known abilities decide what it could produce.
    let filter = if TokenWordView::new(filter_tokens).to_word_refs() == ["the", "sacrificed", "land"]
    {
        crate::word_claims::claim_tokens(filter_tokens);
        ObjectFilter::land().match_tagged(
            crate::tag::CompilerReferenceTag::AdditionalCostObject.bind(),
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        )
    } else {
        parse_object_filter(filter_tokens, false)?
    };
    Ok(Some((filter, mana_type_source)))
}
