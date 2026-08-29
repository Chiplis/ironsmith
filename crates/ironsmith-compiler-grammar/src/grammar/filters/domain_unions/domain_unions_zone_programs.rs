use super::*;

pub(super) fn possessive_zone_owner(token: &OwnedLexToken) -> Option<PlayerFilter> {
    match token.as_word()? {
        "your" => Some(PlayerFilter::You),
        "their" => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

pub(super) fn parse_in_zone_at(
    tokens: &[OwnedLexToken],
    start: usize,
) -> Option<(Zone, Option<PlayerFilter>, usize)> {
    if !tokens.get(start)?.is_word("in") {
        return None;
    }
    let first = tokens.get(start + 1)?;
    if let Some(zone) = first.as_word().and_then(crate::util::parse_zone_word) {
        return Some((zone, None, start + 2));
    }
    let owner = possessive_zone_owner(first)?;
    let zone = tokens
        .get(start + 2)?
        .as_word()
        .and_then(crate::util::parse_zone_word)?;
    Some((zone, Some(owner), start + 3))
}
