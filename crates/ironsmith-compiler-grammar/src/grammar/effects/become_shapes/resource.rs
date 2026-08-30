use super::*;

pub fn parse_become_iterated_mana_value_pt_words<'a>(
    words: &'a [&'a str],
) -> Option<BecomePowerToughnessTail<'a>> {
    const HEADS: &[&[&str]] = &[
        &["base", "power", "and", "base", "toughness"],
        &["base", "power", "and", "toughness"],
        &["power", "and", "toughness"],
    ];
    const VALUE_REFS: &[&[&str]] = &[
        &["its", "mana", "value"],
        &["their", "mana", "value"],
        &["that", "permanent", "s", "mana", "value"],
        &["that", "permanents", "mana", "value"],
        &["that", "object", "s", "mana", "value"],
        &["that", "objects", "mana", "value"],
    ];

    let with = permission_shapes::find_words(words, &["with"])?;
    let tail = words.get(with + 1..)?;
    let head = HEADS
        .iter()
        .find(|head| permission_shapes::prefix_words(tail, head))?;
    let rhs = tail.get(head.len()..)?;
    if !permission_shapes::prefix_words(rhs, &["each", "equal", "to"]) {
        return None;
    }
    let value_words = rhs.get(3..)?;
    if !VALUE_REFS
        .iter()
        .any(|expected| permission_shapes::exact_words(value_words, expected))
    {
        return None;
    }
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    Some(BecomePowerToughnessTail {
        descriptor_words: &words[..with],
        power: value.clone(),
        toughness: value,
    })
}
