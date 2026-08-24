use super::*;

pub(super) fn put_chosen_cards_battlefield_or_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&[
            "put",
            "the",
            "chosen",
            "cards",
            "onto",
            "the",
            "battlefield",
            "or",
            "into",
            "your",
            "hand",
        ]),
        primitives::phrase(&[
            "put",
            "those",
            "cards",
            "onto",
            "the",
            "battlefield",
            "or",
            "into",
            "your",
            "hand",
        ]),
    ))
    .void()
    .parse_next(input)
}

/// Captures a leading-if self-replacement whose replacement changes only the
/// destination of the already chosen collection. Keeping the predicate tokens
/// separate lets the ordinary typed predicate parser retain the threshold.
pub fn parse_chosen_cards_destination_replacement_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChosenCardsDestinationReplacementShape<'_>> {
    let clause = trimmed(tokens);
    let ((), after_if) = primitives::parse_prefix(clause, |input: &mut LexStream<'_>| {
        primitives::kw("if").void().parse_next(input)
    })?;
    let (comma_idx, (), after_comma) =
        primitives::find_prefix(after_if, || primitives::comma().void())?;
    let predicate_tokens = trimmed(&after_if[..comma_idx]);
    if predicate_tokens.is_empty() {
        return None;
    }
    let mut replacement = trimmed(after_comma);
    if let Some(((), after_instead)) =
        primitives::parse_prefix(replacement, |input: &mut LexStream<'_>| {
            primitives::kw("instead").void().parse_next(input)
        })
    {
        replacement = trimmed(after_instead);
    }
    let ((), remainder) =
        primitives::parse_prefix(replacement, put_chosen_cards_battlefield_or_hand)?;
    let disposition = parse_chosen_cards_disposition_tail(remainder)?;
    Some(ChosenCardsDestinationReplacementShape {
        predicate_tokens,
        order: disposition.order,
    })
}

pub fn parse_may_reveal_looked_card_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardRevealShape<'_>> {
    let clause = trimmed(tokens);
    let ((), count_surface) = primitives::parse_prefix(clause, may_reveal_looked)?;
    let count_surface = trimmed(count_surface);
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(count_surface)?;
    let filter_surface = trimmed(&count_surface[parsed.consumed..]);
    let (among_idx, (), after_among) = primitives::find_prefix(filter_surface, || from_among_them)?;
    let filter_tokens = trimmed(&filter_surface[..among_idx]);
    let after_among = trimmed(after_among);
    let x_value = if after_among.is_empty() {
        None
    } else {
        let ((), value_tokens) = primitives::parse_prefix(after_among, where_x_prefix)?;
        Some(super::super::looked_card_shapes::parse_where_x_value(
            trimmed(value_tokens),
        )?)
    };
    (!filter_tokens.is_empty()).then_some(LookedCardRevealShape {
        filter_tokens,
        count: parsed.count,
        x_value,
    })
}

pub(super) fn put_revealed_into_hand_then_shuffle<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["put", "the", "revealed", "cards", "into", "your", "hand"]),
        primitives::phrase(&["put", "those", "cards", "into", "your", "hand"]),
        primitives::phrase(&["put", "them", "into", "your", "hand"]),
    ))
    .void()
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["then", "shuffle"])
        .void()
        .parse_next(input)?;
    opt(primitives::phrase(&["your", "library"]))
        .void()
        .parse_next(input)
}

pub fn parse_put_revealed_into_hand_then_shuffle_shape(tokens: &[OwnedLexToken]) -> bool {
    exact_unit(tokens, put_revealed_into_hand_then_shuffle)
}

pub(super) fn put_revealed_onto_battlefield<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&[
        "put",
        "the",
        "revealed",
        "cards",
        "onto",
        "the",
        "battlefield",
    ])
    .void()
    .parse_next(input)
}

pub fn parse_bargained_revealed_battlefield_shape(tokens: &[OwnedLexToken]) -> bool {
    let clause = trimmed(tokens);
    primitives::parse_prefix(clause, bargained).is_some()
        && primitives::find_prefix(clause, || put_revealed_onto_battlefield).is_some()
}

pub(super) fn otherwise_revealed_into_hand<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("otherwise").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["put", "the", "revealed", "cards", "into", "your", "hand"])
        .void()
        .parse_next(input)
}

pub fn parse_otherwise_revealed_hand_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(trimmed(tokens), otherwise_revealed_into_hand).is_some()
}

pub(super) fn may_exile_looked<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["you", "may", "exile"])
        .void()
        .parse_next(input)
}

pub fn parse_may_exile_looked_card_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardFilterShape<'_>> {
    let clause = trimmed(tokens);
    let ((), filter_surface) = primitives::parse_prefix(clause, may_exile_looked)?;
    let filter_surface = trimmed(filter_surface);
    let (among_idx, (), after_among) = primitives::find_prefix(filter_surface, || from_among_them)?;
    if !trimmed(after_among).is_empty() {
        return None;
    }
    let filter_tokens = trimmed(&filter_surface[..among_idx]);
    (!filter_tokens.is_empty()).then_some(LookedCardFilterShape { filter_tokens })
}

pub fn parse_exile_looked_card_and_remainder_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookedCardExileRemainderShape<'_>> {
    let clause = trimmed(tokens);
    let ((), count_surface) = primitives::parse_prefix(clause, |input: &mut LexStream<'_>| {
        primitives::kw("exile").void().parse_next(input)
    })?;
    let count_surface = trimmed(count_surface);
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(count_surface)?;
    let filter_surface = trimmed(&count_surface[parsed.consumed..]);
    let (among_idx, (), after_among) = primitives::find_prefix(filter_surface, || from_among_them)?;
    let filter_tokens = trimmed(&filter_surface[..among_idx]);
    if filter_tokens.is_empty() {
        return None;
    }
    let ((), remainder_tokens) =
        primitives::parse_prefix(trimmed(after_among), |input: &mut LexStream<'_>| {
            primitives::kw("and").void().parse_next(input)
        })?;
    let order = parse_looked_remainder_bottom_shape(trimmed(remainder_tokens))?;
    Some(LookedCardExileRemainderShape {
        filter_tokens,
        count: parsed.count,
        order,
    })
}

pub fn parse_look_exile_split_shape(tokens: &[OwnedLexToken]) -> Option<LookExileSplitShape<'_>> {
    let clause = trimmed(tokens);
    let (exile_idx, (), _) = primitives::find_prefix(clause, || primitives::kw("exile").void())?;
    Some(LookExileSplitShape {
        look_tokens: trimmed(&clause[..exile_idx]),
        exile_tokens: trimmed(&clause[exile_idx..]),
    })
}
