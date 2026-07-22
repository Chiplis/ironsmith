use super::*;

pub(super) fn normalized_reminder_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    let words = if let Some(rest) = common::strip_phrase_prefix(words, &["it", "has"])
        .or_else(|| common::strip_phrase_prefix(words, &["they", "have"]))
    {
        rest
    } else {
        words
    };

    // Possessive apostrophes are removed from parser words, so the authored
    // "This token's power ..." surface arrives as "this tokens power ...".
    // The characteristic sentence may be quoted inside the larger token
    // definition rather than beginning the token slice. Normalize that
    // explicit token reference to the same semantic subject as "Its power".
    for subject in [
        &["this", "tokens"][..],
        &["this", "token"],
        &["thiss", "token"],
    ] {
        if let Some(start) = common::phrase_offset(words, subject)
            && let Some(tail) = words.get(start + subject.len()..)
            && common::phrase_present(
                tail,
                &["power", "and", "toughness", "are", "each", "equal", "to"],
            )
        {
            let mut normalized = vec!["its"];
            normalized.extend_from_slice(tail);
            return normalized;
        }
    }

    for prefix in [
        &["when", "it"][..],
        &["whenever", "it"],
        &["when", "they"],
        &["whenever", "they"],
    ] {
        if let Some(rest) = common::strip_phrase_prefix(words, prefix) {
            let mut normalized = vec![words[0], "this", "token"];
            normalized.extend_from_slice(rest);
            return normalized;
        }
    }
    words.to_vec()
}

fn parse_possessive_stat_rhs(words: &[&str], is_power: bool) -> Option<Value> {
    let stat_word = if is_power { "power" } else { "toughness" };
    let owner_words = words.strip_suffix(&[stat_word])?;
    let source_value = || {
        if is_power {
            Value::SourcePower
        } else {
            Value::SourceToughness
        }
    };
    let tagged_value = |tag: &str| {
        let spec = Box::new(ChooseSpec::Tagged(TagKey::from(tag)));
        if is_power {
            Value::PowerOf(spec)
        } else {
            Value::ToughnessOf(spec)
        }
    };

    if [
        &["this"][..],
        &["thiss"],
        &["this", "creature"],
        &["thiss", "creature"],
        &["this", "creatures"],
        &["thiss", "creatures"],
    ]
    .iter()
    .any(|expected| common::phrase_exact(owner_words, expected))
    {
        return Some(source_value());
    }
    if [&["that", "card"][..], &["that", "cards"]]
        .iter()
        .any(|expected| common::phrase_exact(owner_words, expected))
    {
        return Some(tagged_value(
            crate::runtime_backend::token_definition::TOKEN_DYNAMIC_THAT_CARD_TAG,
        ));
    }
    if [
        &["that", "creature"][..],
        &["that", "creatures"],
        &["that", "object"],
        &["that", "objects"],
    ]
    .iter()
    .any(|expected| common::phrase_exact(owner_words, expected))
    {
        return Some(tagged_value(IT_TAG));
    }
    None
}

fn parse_dynamic_rhs(words: &[&str]) -> Option<Value> {
    if let Some(value) =
        parse_possessive_stat_rhs(words, true).or_else(|| parse_possessive_stat_rhs(words, false))
    {
        return Some(value);
    }
    let (value, used) = value_expr::parse_value_expr_words(words)?;
    (used == words.len()).then_some(value)
}

pub(super) fn parse_dynamic_power_toughness(words: &[&str]) -> Option<(Value, Value)> {
    if let Some(start) = common::phrase_offset(words, &["with", "power", "equal", "to"])
        && let Some(power_rhs) =
            common::strip_phrase_prefix(words.get(start..)?, &["with", "power", "equal", "to"])
    {
        let and_idx = common::phrase_offset(power_rhs, &["and"])?;
        let power = parse_dynamic_rhs(power_rhs.get(..and_idx)?)?;
        let toughness_rhs = common::strip_phrase_prefix(
            power_rhs.get(and_idx + 1..)?,
            &["toughness", "equal", "to"],
        )?;
        return Some((power, parse_dynamic_rhs(toughness_rhs)?));
    }

    if let Some(rhs) = common::strip_phrase_prefix(
        words,
        &[
            "its",
            "power",
            "and",
            "toughness",
            "are",
            "each",
            "equal",
            "to",
        ],
    ) {
        let value = parse_dynamic_rhs(rhs)?;
        return Some((value.clone(), value));
    }

    let power_rhs = common::strip_phrase_prefix(words, &["its", "power", "is", "equal", "to"])?;
    let and_idx = common::phrase_offset(power_rhs, &["and"])?;
    let power = parse_dynamic_rhs(power_rhs.get(..and_idx)?)?;
    let toughness_rhs = common::strip_phrase_prefix(
        power_rhs.get(and_idx + 1..)?,
        &["its", "toughness", "is", "equal", "to"],
    )?;
    Some((power, parse_dynamic_rhs(toughness_rhs)?))
}

pub(crate) fn parse_token_dynamic_power_toughness_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(Value, Value)> {
    let raw_words = parser_token_word_refs(tokens);
    let words = normalized_reminder_words(&raw_words);
    parse_dynamic_power_toughness(&words)
}
