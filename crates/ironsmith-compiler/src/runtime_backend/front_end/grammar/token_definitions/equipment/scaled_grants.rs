use super::*;

fn fixed_power_toughness_word(word: &str) -> Option<(i32, i32)> {
    let parsed = leaf::parse_leaf_power_toughness_complete(word).ok()?;
    match (parsed.power, parsed.toughness) {
        (PtValue::Fixed(power), PtValue::Fixed(toughness)) => Some((power, toughness)),
        _ => None,
    }
}

fn parse_scaled_equipment_grant_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<EquipmentScaledPowerToughnessShape> {
    (
        primitives::word_slice_exact("equipped"),
        primitives::word_slice_exact("creature"),
        primitives::word_slice_exact("gets"),
    )
        .parse_next(input)?;
    let Some((power_toughness_word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err(
            "equipment stat grant",
            "power/toughness",
        ));
    };
    let (power, toughness) = fixed_power_toughness_word(power_toughness_word).ok_or_else(|| {
        primitives::backtrack_err("equipment stat grant", "fixed power/toughness")
    })?;
    *input = rest;
    (
        primitives::word_slice_exact("for"),
        primitives::word_slice_exact("each"),
    )
        .parse_next(input)?;
    let descriptor_words = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            primitives::word_slice_exact("counter"),
            primitives::word_slice_exact("counters"),
        )))
        .void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    alt((
        primitives::word_slice_exact("counter"),
        primitives::word_slice_exact("counters"),
    ))
    .parse_next(input)?;
    (
        primitives::word_slice_exact("among"),
        primitives::word_slice_exact("permanents"),
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("control"),
    )
        .parse_next(input)?;
    let mut counter_words = descriptor_words.to_vec();
    counter_words.push("counter");
    let counter_type = filters::parse_counter_type_words(&counter_words)
        .ok_or_else(|| primitives::backtrack_err("equipment stat grant", "known counter type"))?;
    Ok(EquipmentScaledPowerToughnessShape {
        power,
        toughness,
        count: EquipmentGrantCountShape::CountersAmongPermanentsYouControl(counter_type),
    })
}

pub(super) fn scaled_equipment_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EquipmentScaledPowerToughnessShape> {
    let words = parser_token_word_refs(tokens);
    let mut input: primitives::WordSliceInput<'_> = &words;
    let (_, shape) =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parse_scaled_equipment_grant_words)
            .parse_next(&mut input)
            .ok()?;
    Some(shape)
}
