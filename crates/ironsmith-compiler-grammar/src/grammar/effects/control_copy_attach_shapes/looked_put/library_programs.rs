use super::*;

pub fn parse_tagged_on_top_library_shape(tokens: &[OwnedLexToken]) -> Option<TaggedTopPutShape> {
    let rest = rest_head(tokens)?;
    if !primitives::contains_word(rest, "bottom") {
        return None;
    }
    let body = strip_optional_put(tokens);
    let (on_index, _, destination) = primitives::find_prefix(body, || primitives::kw("on"))?;
    let count = parse_count_and_reference(trim_lexed_commas(body.get(..on_index)?))?;
    let (_, destination) = primitives::parse_prefix(
        trim_lexed_commas(destination),
        (
            opt(primitives::kw("the")),
            primitives::kw("top"),
            opt(primitives::kw("of")),
        )
            .void(),
    )?;
    if !(primitives::contains_word(destination, "library")
        || primitives::contains_word(destination, "libraries"))
    {
        return None;
    }
    Some(TaggedTopPutShape {
        count,
        bottom_order: super::super::super::sequence_pairs::parse_bottom_order(tokens)?,
    })
}

pub fn parse_revealed_remainder_shape(tokens: &[OwnedLexToken]) -> Option<RevealedRemainderShape> {
    let is_remainder = [
        "rest", "cards", "revealed", "this", "way", "bottom", "library",
    ]
    .into_iter()
    .all(|word| primitives::contains_word(tokens, word));
    let is_full_collection =
        (permission_shapes::contains_tokens(tokens, &["the", "revealed", "cards"])
            || permission_shapes::contains_tokens(tokens, &["all", "revealed", "cards"])
            || permission_shapes::contains_tokens(tokens, &["all", "the", "revealed", "cards"]))
            && primitives::contains_word(tokens, "bottom")
            && primitives::contains_word(tokens, "library")
            && !primitives::contains_word(tokens, "rest");
    let is_you_revealed_collection = (permission_shapes::contains_tokens(
        tokens,
        &["the", "cards", "you", "revealed", "this", "way"],
    ) || permission_shapes::contains_tokens(
        tokens,
        &["cards", "you", "revealed", "this", "way"],
    )) && primitives::contains_word(tokens, "bottom")
        && primitives::contains_word(tokens, "library")
        && !primitives::contains_word(tokens, "rest");
    if !is_remainder && !is_full_collection && !is_you_revealed_collection {
        return None;
    }
    let surface = if is_you_revealed_collection {
        ironsmith_core::LibraryRemainderSurface::CardsYouRevealedThisWay
    } else if is_remainder
        && permission_shapes::contains_tokens(
            tokens,
            &["rest", "of", "the", "cards", "revealed", "this", "way"],
        )
    {
        ironsmith_core::LibraryRemainderSurface::RestOfCardsRevealedThisWay
    } else {
        ironsmith_core::LibraryRemainderSurface::Rest
    };
    Some(RevealedRemainderShape {
        random_order: primitives::contains_word(tokens, "random"),
        exclude_current_reference: is_remainder,
        surface,
    })
}

pub fn is_reorder_tagged_cards(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "back")
        && primitives::contains_word(tokens, "any")
        && primitives::contains_word(tokens, "order")
        && (primitives::contains_word(tokens, "it") || primitives::contains_word(tokens, "them"))
}
