use super::*;

pub fn parse_aggregate_prefix(words: &[&str]) -> Option<AggregatePrefix> {
    let mut index = usize::from(permission_shapes::prefix_words(words, &["the"]));
    let aggregate = match words.get(index).copied()? {
        "total" => AggregateKind::Total,
        "greatest" => AggregateKind::Greatest,
        _ => return None,
    };
    index += 1;
    let value_kind = if permission_shapes::starts_at_words(words, index, &["mana", "value"]) {
        index += 2;
        AggregateValueKind::ManaValue
    } else {
        let kind = match words.get(index).copied()? {
            "power" => AggregateValueKind::Power,
            "toughness" => AggregateValueKind::Toughness,
            _ => return None,
        };
        index += 1;
        kind
    };
    if words
        .get(index)
        .is_none_or(|word| !matches!(*word, "of" | "among"))
    {
        return None;
    }
    Some(AggregatePrefix {
        aggregate,
        value_kind,
        consumed: index + 1,
    })
}

pub fn starts_equal_to_opponents_you_have(words: &[&str]) -> bool {
    permission_shapes::prefix_words(
        words,
        &[
            "equal",
            "to",
            "the",
            "number",
            "of",
            "opponents",
            "you",
            "have",
        ],
    ) || permission_shapes::prefix_words(
        words,
        &["equal", "to", "number", "of", "opponents", "you", "have"],
    )
}

pub fn starts_or_power_toughness(words: &[&str]) -> bool {
    permission_shapes::prefix_words(words, &["or", "power"])
        || permission_shapes::prefix_words(words, &["or", "toughness"])
}

pub(super) fn has_word(words: &[&str], expected: &str) -> bool {
    permission_shapes::find_words(words, &[expected]).is_some()
}

pub(super) fn has_any(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().any(|word| has_word(words, word))
}
