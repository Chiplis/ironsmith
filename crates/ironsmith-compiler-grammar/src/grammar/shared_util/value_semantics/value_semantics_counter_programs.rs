use super::*;

pub fn parse_equal_to_number_of_counters_on_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let mut shape = value_helper_shapes::parse_counter_reference_value_shape(&words)?;
    if let value_helper_shapes::CounterValueReference::Source(Some(
        crate::target::SourceReferenceSurface::ThisPermanentType(surface),
    )) = &mut shape.reference
        && let Some(reference_start) =
            crate::word_primitives::parse_last_sequence_start(&words, &["on"]).map(|idx| idx + 1)
        && let Some(range) = word_view.token_span_for_words(reference_start, word_view.len())
    {
        *surface = render_token_slice(&tokens[range]).trim().to_string();
    }
    Some(counter_reference_shape_value(shape).with_surface_hint(ValueSurfaceHint::EqualTo))
}
