use super::*;

pub(super) fn parse_base_power_grant_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<BasePowerGrantShape<'a>> {
    let initial_len = input.len();
    let _subject_tokens = take_until_have.parse_next(input)?;
    parse_have.parse_next(input)?;
    let has_token = initial_len.saturating_sub(input.len() + 1);
    primitives::phrase(&["base", "power"]).parse_next(input)?;
    let raw = primitives::word_parser_text.parse_next(input)?;
    let power = leaf::parse_number_i32_complete(raw)
        .map_err(|_| primitives::backtrack_err("base power", "fixed signed power"))?;
    primitives::kw("and").parse_next(input)?;
    alt((
        primitives::kw("have"),
        primitives::kw("has"),
        primitives::kw("gain"),
        primitives::kw("gains"),
    ))
    .parse_next(input)?;
    let ability_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    let ability_tokens = trim_lexed_commas(ability_tokens);
    if ability_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "base-power grant",
            "nonempty granted ability",
        ));
    }
    Ok(BasePowerGrantShape {
        has_token,
        power,
        ability_tokens,
    })
}
