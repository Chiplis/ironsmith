use super::*;

pub fn is_revealed_land_creature_split_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, &[&["put"]])
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "land",
                "cards",
                "revealed",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
                "tapped",
            ]],
        )
        && contains_sequence_phrase(
            tokens,
            &[&[
                "put", "all", "creature", "cards", "revealed", "this", "way", "into", "your",
                "hand",
            ]],
        )
}

pub fn parse_looked_remainder_shape(tokens: &[OwnedLexToken]) -> Option<LookedRemainderShape> {
    let tail = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        sequence_phrase(&["then"]).parse_next(input)
    })
    .map(|(_, tail)| tail)
    .unwrap_or(tokens);
    // Multi-sentence looked-card procedures commonly restate the library
    // owner before the final disposition ("That player puts the rest ..."
    // or "They put the rest ..."). The enclosing sequence parser already
    // carries the original player structurally, so this grammar only needs to
    // remove the anaphoric actor before recognizing the remainder action.
    let tail = match tail {
        [first, second, rest @ ..]
            if first.parser_text() == "that"
                && matches!(second.parser_text(), "player" | "opponent") =>
        {
            rest
        }
        [first, rest @ ..] if first.parser_text() == "they" => rest,
        _ => tail,
    };
    let explicit_rest = contains_sequence_word(tail, "rest");
    let exact_revealed_complement =
        is_explicit_revealed_cards_not_put_onto_battlefield_complement(tail);
    if !starts_sequence(tail, &[&["put"], &["puts"]])
        || (!explicit_rest && !exact_revealed_complement)
    {
        return None;
    }
    if contains_sequence_word(tail, "bottom") && contains_sequence_word(tail, "library") {
        return parse_consult_remainder_order_tokens(tail).map(LookedRemainderShape::LibraryBottom);
    }
    contains_sequence_word(tail, "graveyard").then_some(LookedRemainderShape::Graveyard)
}

/// Whether the authored remainder explicitly names the revealed-card
/// complement rather than using the shorter "the rest" surface.
pub fn is_explicit_revealed_cards_not_put_onto_battlefield_complement(
    tokens: &[OwnedLexToken],
) -> bool {
    contains_sequence_phrase(tokens, &[&["all", "cards", "revealed", "this", "way"]])
        && (contains_sequence_phrase(tokens, &[&["werent", "put", "onto", "the", "battlefield"]])
            || contains_sequence_phrase(
                tokens,
                &[&["weren't", "put", "onto", "the", "battlefield"]],
            )
            || contains_sequence_phrase(
                tokens,
                &[&["were", "not", "put", "onto", "the", "battlefield"]],
            ))
}

/// Retains the authored wording for a looked/revealed-set complement while
/// leaving its execution semantics unchanged.
pub fn looked_remainder_surface(
    tokens: &[OwnedLexToken],
) -> ironsmith_core::LibraryRemainderSurface {
    if is_explicit_revealed_cards_not_put_onto_battlefield_complement(tokens) {
        return ironsmith_core::LibraryRemainderSurface::RevealedCardsNotPutOntoBattlefield;
    }
    if contains_sequence_phrase(
        tokens,
        &[
            &[
                "the", "rest", "of", "the", "cards", "revealed", "this", "way",
            ],
            &["the", "rest", "of", "cards", "revealed", "this", "way"],
        ],
    ) {
        return ironsmith_core::LibraryRemainderSurface::RestOfCardsRevealedThisWay;
    }
    ironsmith_core::LibraryRemainderSurface::Rest
}

/// Recognizes an optional deployment whose legal candidate must share a name
/// with some permanent on the battlefield.  The comparison set is modeled by
/// the sequence parser rather than treating the trailing `if` as a gate on an
/// arbitrary card from the looked pool.
pub fn is_looked_same_name_permanent_battlefield_action(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(
        tokens,
        &[
            &["you", "may", "put", "one", "of", "those", "cards"],
            &["you", "may", "put", "one", "of", "them"],
        ],
    ) && contains_sequence_phrase(tokens, BATTLEFIELD)
        && contains_sequence_phrase(
            tokens,
            &[
                &[
                    "if",
                    "it",
                    "has",
                    "the",
                    "same",
                    "name",
                    "as",
                    "a",
                    "permanent",
                ],
                &[
                    "if",
                    "that",
                    "card",
                    "has",
                    "the",
                    "same",
                    "name",
                    "as",
                    "a",
                    "permanent",
                ],
            ],
        )
}

pub fn parse_any_number_revealed_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<AnyNumberRevealedChoiceShape> {
    let (_, tail) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        sequence_phrase(&["choose"]).parse_next(input)
    })?;
    let (count, after_count) =
        primitives::parse_prefix(tail, leaf::parse_leaf_choice_count_prefix_lexed)?;
    if count != ChoiceCount::any_number() {
        return None;
    }
    let mut input = LexStream::new(after_count);
    let filter_end = crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        seek_sequence_phrase(input, &[&["revealed", "this", "way"]])
    })?;
    crate::grammar::primitives::take_leaf(
        &mut input,
        sequence_phrase(&["revealed", "this", "way"]),
    )?;
    crate::grammar::primitives::take_leaf(&mut input, finish_sequence_words)?;
    let filter_start =
        tokens.len().saturating_sub(tail.len()) + tail.len().saturating_sub(after_count.len());
    (filter_end > 0).then_some(AnyNumberRevealedChoiceShape {
        count,
        filter: filter_start..filter_start + filter_end,
    })
}

pub fn is_land_nonland_split_bottom_shape(tokens: &[OwnedLexToken]) -> bool {
    starts_sequence(tokens, &[&["put"]])
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "nonland",
                "cards",
                "chosen",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
            ]],
        )
        && contains_sequence_phrase(
            tokens,
            &[&[
                "all",
                "land",
                "cards",
                "chosen",
                "this",
                "way",
                "onto",
                "the",
                "battlefield",
                "tapped",
            ]],
        )
        && parse_looked_remainder_shape(tokens)
            == Some(LookedRemainderShape::LibraryBottom(
                LibraryBottomOrderAst::Random,
            ))
}

pub fn parse_reveal_one_gain_mana_value_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<RevealOneGainManaValueShape> {
    let mut input = LexStream::new(first);
    let view_end = crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        seek_sequence_phrase(input, &[&["and", "put"]])
    })?;
    crate::grammar::primitives::take_leaf(&mut input, sequence_phrase(&["and"]))?;
    let tail_start = first.len().saturating_sub(input.len());
    if !starts_sequence(&first[tail_start..], PUT_ONE_INTO_HAND)
        || !starts_sequence(second, &[&["you", "gain", "life"]])
        || !contains_sequence_phrase(second, &[&["mana", "value"]])
        || (!contains_sequence_word(second, "card")
            && !contains_sequence_word(second, "cards")
            && !contains_sequence_word(second, "card's"))
        || !starts_sequence(third, &[&["put"], &["puts"]])
        || !contains_sequence_word(third, "other")
        || !contains_sequence_word(third, "revealed")
        || !contains_sequence_word(third, "graveyard")
    {
        return None;
    }
    Some(RevealOneGainManaValueShape { view: 0..view_end })
}
