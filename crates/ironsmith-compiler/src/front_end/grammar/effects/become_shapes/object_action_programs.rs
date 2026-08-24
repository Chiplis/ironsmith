use super::*;

pub fn parse_become_base_pt_words<'a>(
    words: &'a [&'a str],
) -> Option<BecomePowerToughnessTail<'a>> {
    if let Some(iterated) = parse_become_iterated_mana_value_pt_words(words) {
        return Some(iterated);
    }
    let with = permission_shapes::find_words(words, &["with"])?;
    let tail = words.get(with + 1..)?;
    const HEADS: &[&[&str]] = &[
        &["base", "power", "and", "base", "toughness"],
        &["base", "power", "and", "toughness"],
        &["power", "and", "toughness"],
    ];
    let head = HEADS
        .iter()
        .find(|head| permission_shapes::prefix_words(tail, head))?;
    let value_words = tail.get(head.len()..)?;
    if permission_shapes::prefix_words(value_words, &["each", "equal", "to"]) {
        let expression_words = value_words.get(3..)?;
        let (value, consumed) = parse_become_iterated_counter_value_words(expression_words)
            .map(|value| (value, expression_words.len()))
            .or_else(|| crate::util::parse_value_expr_words(expression_words))?;
        return (consumed == expression_words.len()).then(|| BecomePowerToughnessTail {
            descriptor_words: &words[..with],
            power: value.clone(),
            toughness: value,
        });
    }
    let (power, toughness, consumed) = parse_modifier_words(value_words)?;
    (consumed == value_words.len()).then_some(BecomePowerToughnessTail {
        descriptor_words: &words[..with],
        power,
        toughness,
    })
}
