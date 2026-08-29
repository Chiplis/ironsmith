use super::*;

pub(super) fn poison_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<(u32, &[OwnedLexToken])>, CardTextError> {
    let Some((_, after_has)) =
        primitives::parse_prefix(trim(tokens), primitives::phrase(&["who", "has"]))
    else {
        return Ok(None);
    };
    let Some((count, used)) = parse_greater_than_or_equal_quantity_prefix(
        after_has,
        false,
        false,
        "for-each poison-counter predicate",
    )?
    else {
        return Ok(None);
    };
    let Some(after_count) = after_has.get(used..) else {
        return Ok(None);
    };
    let Some((_, rest)) = primitives::parse_prefix(
        after_count,
        alt((
            primitives::phrase(&["poison", "counter"]),
            primitives::phrase(&["poison", "counters"]),
        ))
        .void(),
    ) else {
        return Ok(None);
    };
    Ok(Some((count, trim(rest))))
}
