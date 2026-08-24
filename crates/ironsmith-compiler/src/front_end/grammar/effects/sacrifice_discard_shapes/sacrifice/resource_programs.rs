use super::*;

pub fn parse_sacrifice_count_shape(tokens: &[OwnedLexToken]) -> SacrificeCountShape<'_> {
    let mut count = 1u32;
    let mut rest = tokens;
    // `one of them` names one member of the previously established object
    // set; its `one` is not an ordinary count prefix. Keep the complete
    // phrase intact so `parse_sacrifice_object_shape` can preserve the set
    // choice semantics instead of degrading it to a bare `them` reference.
    let one_of_tagged_set =
        primitives::parse_prefix(rest, primitives::phrase(&["one", "of", "them"]).void()).is_some();
    if !one_of_tagged_set
        && let Some(prefix) = leaf::parse_leaf_number_prefix_tokens(rest)
        && let Some((value, used)) = prefix.into_fixed()
    {
        count = value;
        rest = &rest[used..];
    }

    let mut other = false;
    if let Some((_, after_another)) =
        primitives::parse_prefix(rest, primitives::kw("another").void())
    {
        other = true;
        rest = after_another;
    }

    if !one_of_tagged_set
        && count == 1
        && let Some(prefix) = leaf::parse_leaf_number_prefix_tokens(rest)
        && let Some((value, used)) = prefix.into_fixed()
    {
        count = value;
        rest = &rest[used..];
    }

    SacrificeCountShape {
        count,
        other,
        filter_tokens: rest,
    }
}

pub fn parse_sacrifice_aggregate_shape(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeAggregateShape<'_>> {
    let (marker_offset, kind, among_tokens) = primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["with", "the", "greatest", "mana", "value", "among"])
                .value(SacrificeAggregateKind::GreatestManaValue),
            primitives::phrase(&["with", "the", "greatest", "power", "among"])
                .value(SacrificeAggregateKind::GreatestPower),
        ))
    })?;
    Some(SacrificeAggregateShape {
        kind,
        object_tokens: &tokens[..marker_offset],
        among_tokens,
    })
}

pub fn parse_sacrifice_attached_exclusion(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    ATTACHED_EXCLUSIONS
        .iter()
        .any(|phrase| common::present(&words, phrase))
}
