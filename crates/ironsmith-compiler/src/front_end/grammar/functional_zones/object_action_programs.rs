use super::*;

pub fn parse_static_functional_zones_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<Zone>> {
    if has_any_phrase(tokens, SOURCE_NOT_ON_BATTLEFIELD_PHRASES) {
        return Some(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if has_any_phrase(tokens, STATIC_LIBRARY_SEARCH_ZONE_PHRASES)
        && has_phrase(tokens, FROM_YOUR_LIBRARY_PHRASE)
    {
        return Some(vec![Zone::Library]);
    }
    if has_any_phrase(tokens, CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PHRASES) {
        return Some(vec![Zone::Graveyard]);
    }
    if has_any_phrase(tokens, CAST_OR_PLAY_SELF_FROM_EXILE_PHRASES) {
        return Some(vec![Zone::Exile]);
    }
    if has_phrase(tokens, CAUSES_YOU_TO_DISCARD_THIS_CARD_PHRASE)
        && has_phrase(tokens, INSTEAD_OF_PUTTING_IT_INTO_YOUR_GRAVEYARD_PHRASE)
    {
        return Some(vec![Zone::Hand]);
    }

    let zones = STATIC_ZONE_HINT_PHRASES
        .iter()
        .filter(|(phrase, _)| has_phrase(tokens, phrase))
        .map(|(_, zone)| *zone)
        .collect::<Vec<_>>();
    (!zones.is_empty()).then_some(zones)
}
