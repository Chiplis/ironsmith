//! Intrinsic transformed casting permissions describe a zone, price and face.
//! They do not confer a named keyword or imply a departure replacement.
use super::*;

pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let prose = render_token_slice(tokens).to_ascii_lowercase();
    let prose = prose.trim().trim_end_matches('.');
    let price = ["you may cast this card transformed from your graveyard by paying ",
        "you may cast this spell transformed from your graveyard by paying "]
        .iter().find_map(|prefix| prose.strip_prefix(prefix))
        .and_then(|rest| rest.strip_suffix(" rather than paying its mana cost"));
    let Some(price) = price else { return Ok(None); };
    // Own just the price independently. A trailing sentence or an unsupported
    // payment is never swallowed by the permission head.
    let price = lex_line(price, 0)?;
    let total_cost = crate::activation_and_restrictions::parse_compiler_activation_cost(&price)?;
    crate::word_claims::claim_tokens(tokens);
    Ok(Some(AlternativeCastingMethod::cast_from_zone_with_total_cost(
        GENERIC_ALTERNATIVE_COST_NAME, crate::zone::Zone::Graveyard,
        total_cost, None, false,
    ).with_zone_cast_instructions(ironsmith_core::ZoneCastInstructions {
        transformed: true, ..Default::default()
    })))
}
