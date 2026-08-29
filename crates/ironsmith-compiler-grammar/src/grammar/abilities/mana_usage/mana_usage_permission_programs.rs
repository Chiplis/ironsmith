use super::*;

pub(super) fn parse_alternative_cast_spell_with_origin(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let tokens = strip_article(trim_lexed_commas(tokens));
    let words = TokenWordView::new(tokens).word_refs();
    if !matches!(words.first().copied(), Some("spell" | "spells"))
        || words.get(1).copied() != Some("with")
    {
        return None;
    }

    let alternative = leaf::parse_leaf_alternative_cast_prefix_words(words.get(2..)?)?;
    let origin = words.get(2 + alternative.consumed..)?;
    let (zone, owner) = match origin {
        ["from", "a", "graveyard"] | ["from", "graveyard"] => (Zone::Graveyard, None),
        ["from", "your", "graveyard"] => (Zone::Graveyard, Some(PlayerFilter::You)),
        ["from", "a", "hand"] | ["from", "hand"] => (Zone::Hand, None),
        ["from", "your", "hand"] => (Zone::Hand, Some(PlayerFilter::You)),
        ["from", "a", "library"] | ["from", "library"] => (Zone::Library, None),
        ["from", "your", "library"] => (Zone::Library, Some(PlayerFilter::You)),
        ["from", "exile"] => (Zone::Exile, None),
        ["from", "the", "command", "zone"] => (Zone::Command, None),
        ["from", "outside", "the", "game"] => (Zone::OutsideGame, None),
        _ => return None,
    };

    let mut filter = ObjectFilter::default().in_zone(zone);
    filter.owner = owner;
    filter.alternative_cast = Some(alternative.kind);
    Some(filter)
}
