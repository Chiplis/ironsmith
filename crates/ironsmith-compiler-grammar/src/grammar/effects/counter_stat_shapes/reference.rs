use super::*;

pub fn parse_possessive_target_stat(tokens: &[OwnedLexToken]) -> Option<PossessiveTargetStatShape> {
    let words = TokenWordView::new(tokens).word_refs();
    let (stat_words, stat) = if permission_shapes::suffix_words(&words, &["mana", "value"]) {
        (2, TargetStatKind::ManaValue)
    } else if permission_shapes::suffix_words(&words, &["toughness"]) {
        (1, TargetStatKind::Toughness)
    } else if permission_shapes::suffix_words(&words, &["power"]) {
        (1, TargetStatKind::Power)
    } else {
        return None;
    };
    let target_word_count = words.len().checked_sub(stat_words)?;
    let target_end = TokenWordView::new(tokens).token_index_after_words(target_word_count)?;
    let mut target_tokens = tokens.get(..target_end)?.to_vec();
    let possessive = target_tokens.last_mut()?;
    let stem = crate::grammar::primitives::probe_shape(
        parse_possessive_stem.parse(possessive.as_word()?),
    )?;
    if !possessive.replace_word(stem) {
        return None;
    }
    Some(PossessiveTargetStatShape {
        target_tokens,
        stat,
    })
}
