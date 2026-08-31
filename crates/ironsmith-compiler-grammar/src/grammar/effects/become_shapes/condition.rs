use super::*;

pub(super) fn parse_modifier_words(words: &[&str]) -> Option<(Value, Value, usize)> {
    if let Some(first) = words.first()
        && let Ok((power, toughness)) = leaf::parse_leaf_pt_modifier_values_complete(first)
    {
        return Some((power, toughness, 1));
    }
    let (first, second) = (words.first()?, words.get(1)?);
    let joined = format!("{first}/{second}");
    let (power, toughness) = crate::grammar::primitives::probe_shape(
        leaf::parse_leaf_pt_modifier_values_complete(&joined),
    )?;
    Some((power, toughness, 2))
}
