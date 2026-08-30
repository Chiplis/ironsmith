use super::*;

pub fn parse_from_among_them_shape(tokens: &[OwnedLexToken]) -> Option<FromAmongPutShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (reference_index, _, after_reference) = primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["from", "among", "them"]),
            primitives::phrase(&["from", "among", "those", "cards"]),
            primitives::phrase(&["from", "among", "the", "revealed", "cards"]),
            primitives::phrase(&["from", "among", "the", "cards", "revealed", "this", "way"]),
        ))
        .void()
    })?;
    let choice_tokens = strip_optional_put(trim_lexed_commas(tokens.get(..reference_index)?));
    let (count, filter_tokens) =
        if let Some(parsed) = leaf::parse_leaf_choice_count_prefix_tokens(choice_tokens) {
            (
                parsed.count,
                trim_lexed_commas(choice_tokens.get(parsed.consumed..)?),
            )
        } else {
            (ChoiceCount::up_to(1), choice_tokens)
        };
    if filter_tokens.is_empty() {
        return None;
    }
    let after_reference = trim_lexed_commas(after_reference);
    let destination =
        if permission_shapes::prefix_tokens(after_reference, &["onto", "the", "battlefield"])
            || permission_shapes::prefix_tokens(after_reference, &["onto", "battlefield"])
        {
            FromAmongDestinationShape::Battlefield
        } else if primitives::contains_word(after_reference, "hand")
            || primitives::contains_word(after_reference, "hands")
        {
            FromAmongDestinationShape::Hand
        } else {
            FromAmongDestinationShape::Other
        };
    Some(FromAmongPutShape {
        count,
        filter_tokens,
        destination,
        rest_destination: parse_rest_destination(tokens),
    })
}
