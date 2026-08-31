use super::*;

pub(super) fn parse_mana_value_limit(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut input = LexStream::new(tokens);
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        seek_sequence_phrase(input, &[&["mana", "value"]])
    })?;
    crate::grammar::primitives::take_leaf(&mut input, sequence_phrase(&["mana", "value"]))?;
    let value =
        crate::grammar::primitives::take_leaf(&mut input, leaf::parse_leaf_number_prefix_lexed)?;
    crate::grammar::primitives::take_leaf(&mut input, sequence_phrase(&["or", "less"]))?;
    Some(value)
}
