use super::*;

/// Parses the common private-library split where at most one looked-at card
/// is kept on top and the exact complement is randomized onto the bottom.
///
/// The selected set is a singleton, so Magic omits an ordering clause for
/// its top placement.  Keep the ordinary partition grammar strict about
/// explicit library ordering and admit that omission only for this bounded
/// shape.  The bottom library reference may also be elided after the earlier
/// "top of your library" reference (for example, "the rest on the bottom in
/// a random order").
pub(super) fn looked_card_optional_one_top_remainder_bottom(
    input: &mut LexStream<'_>,
) -> WResult<LookedCardPartitionShape> {
    primitives::kw("put").parse_next(input)?;
    let selected_count = looked_partition_count.parse_next(input)?;
    if selected_count != ChoiceCount::up_to(1) {
        return Err(primitives::backtrack_err(
            "looked-card top/bottom partition",
            "an up-to-one selected set",
        ));
    }

    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("top").parse_next(input)?;
    primitives::kw("of").parse_next(input)?;
    looked_partition_library_reference.parse_next(input)?;

    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    alt((primitives::kw("rest"), primitives::kw("other")))
        .void()
        .parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("bottom").parse_next(input)?;
    opt((primitives::kw("of"), looked_partition_library_reference)).parse_next(input)?;
    let remainder_order = looked_partition_order.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(LookedCardPartitionShape {
        selected_count,
        // A chooser order is semantically inert for a set of at most one but
        // gives lowering the typed library-placement order it requires.
        selected_destination: LookedPartitionDestination::LibraryTop(
            LibraryBottomOrderAst::ChooserChooses,
        ),
        remainder_destination: LookedPartitionDestination::LibraryBottom(remainder_order),
    })
}

/// Parses a complete two-way partition of a previously looked-at card set.
///
/// Requiring the sentence to end after both destinations prevents this rule
/// from swallowing longer looked-card procedures. Library placements retain
/// their own order modes so the selected subset and its complement can be
/// ordered independently.
pub fn parse_looked_card_partition_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardPartitionShape> {
    alt((
        looked_card_optional_one_top_remainder_bottom,
        looked_card_partition,
    ))
    .parse(LexStream::new(tokens))
    .ok()
}

pub fn parse_looked_card_into_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardIntoHandShape> {
    let mut input = LexStream::new(tokens);
    let filter_end = seek_sequence_phrase(&mut input, FROM_AMONG).ok()?;
    if filter_end == 0 || is_keyword_bundle_choice_filter(&tokens[..filter_end]) {
        return None;
    }
    sequence_any_phrase(FROM_AMONG)
        .parse_next(&mut input)
        .ok()?;
    let tail_start = tokens.len().saturating_sub(input.len());
    let tail = &tokens[tail_start..];
    if !starts_sequence(tail, &[&["into"]]) || !contains_sequence_word(tail, "hand") {
        return None;
    }
    Some(LookedCardIntoHandShape {
        filter: 0..filter_end,
    })
}

pub fn parse_reveal_top_matching_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<RevealTopMatchingFollowupShape> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    sequence_any_phrase(PUT_ALL).parse_next(&mut input).ok()?;
    let filter_start = initial_len.saturating_sub(input.len());
    seek_sequence_phrase(&mut input, &[&["revealed", "this", "way"]]).ok()?;
    let filter_end = initial_len.saturating_sub(input.len());
    if filter_start >= filter_end {
        return None;
    }
    let filter = &tokens[filter_start..filter_end];
    if is_keyword_bundle_choice_filter(filter) {
        return None;
    }
    sequence_phrase(&["revealed", "this", "way"])
        .parse_next(&mut input)
        .ok()?;
    let tail_start = initial_len.saturating_sub(input.len());
    let tail = &tokens[tail_start..];
    if !contains_sequence_phrase(tail, &[&["into", "your", "hand"]]) {
        return None;
    }
    let bottom_order = parse_bottom_order(tail);
    let graveyard = contains_content_sequence(tail, REST_GRAVEYARD)
        && contains_sequence_word(tail, "graveyard");
    let remainder = if let Some(order) = bottom_order {
        RevealTopRemainder::LibraryBottom(order)
    } else if graveyard {
        RevealTopRemainder::Graveyard
    } else {
        return None;
    };
    Some(RevealTopMatchingFollowupShape {
        filter: filter_start..filter_end,
        chosen_type_reference: contains_sequence_phrase(filter, CHOSEN_TYPE),
        remainder,
    })
}
