use super::*;

pub(super) fn parse_mana_value_limit(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut input = LexStream::new(tokens);
    seek_sequence_phrase(&mut input, &[&["mana", "value"]]).ok()?;
    sequence_phrase(&["mana", "value"])
        .parse_next(&mut input)
        .ok()?;
    let value = leaf::parse_leaf_number_prefix_lexed
        .parse_next(&mut input)
        .ok()?;
    sequence_phrase(&["or", "less"])
        .parse_next(&mut input)
        .ok()?;
    Some(value)
}
