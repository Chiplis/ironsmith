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
    // Locating "in": claimed only once a zone follows.
    if !crate::word_claims::probe_only(|| tokens.get(start).is_some_and(|token| token.is_word("in"))) {
        return None;
    }
    let first = tokens.get(start + 1)?;
    if let Some(zone) = first.as_word().and_then(crate::util::parse_zone_word) {
        crate::word_claims::claim_tokens(&tokens[start..start + 1]);
        return Some((zone, None, start + 2));
    }
    let owner = possessive_zone_owner(first)?;
    let zone = tokens
        .get(start + 2)?
        .as_word()
        .and_then(crate::util::parse_zone_word)?;
    crate::word_claims::claim_tokens(&tokens[start..start + 1]);
    Some((zone, Some(owner), start + 3))
}
