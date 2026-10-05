//! Numeric characteristics of a named prior/event object, not a fresh choice.
use super::*;

fn tagged(tag: crate::tag::CompilerReferenceTag, surface: &str) -> Box<ChooseSpec> {
    Box::new(ChooseSpec::Tagged(tag.key()).with_surface_hint(
        ChooseSpecSurfaceHint::SourceReference(SourceReferenceSurface::ThisPermanentType(
            surface.into(),
        )),
    ))
}

pub(super) fn parse(words: &[&str]) -> Option<(Value, usize)> {
    use crate::tag::CompilerReferenceTag as Tag;
    let offset = usize::from(words.first() == Some(&"the"));
    let rest = &words[offset..];
    // The demonstrative is retained for reference resolution: "that spell"
    // cannot become a created token or a set of destroyed permanents.
    if rest.starts_with(&["mana", "value", "of", "that", "spell"]) {
        return Some((
            Value::ManaValueOf(tagged(Tag::It, "that spell")),
            offset + 5,
        ));
    }
    if rest.starts_with(&["power", "of", "the", "creature", "that", "died"]) {
        return Some((
            Value::PowerOf(tagged(Tag::It, "the creature that died")),
            offset + 6,
        ));
    }
    if rest.starts_with(&["number", "of", "counters", "it", "had", "on", "it"]) {
        return Some((Value::CountersOn(tagged(Tag::It, "it"), None), offset + 7));
    }
    if rest.starts_with(&["amount", "of", "mana", "spent", "to", "cast", "it"]) {
        return Some((Value::ManaSpentToCast(tagged(Tag::It, "it")), offset + 7));
    }
    if rest.len() >= 4
        && rest[0] == "that"
        && matches!(rest[1], "sagas" | "saga's")
        && rest[2..4] == ["mana", "value"]
    {
        return Some((Value::ManaValueOf(tagged(Tag::It, "that Saga")), offset + 4));
    }
    // A milled card is the result of the prior mill, not whichever object a
    // subsequent instruction happened to name. Its result memory survives a
    // replacement destination without borrowing an unrelated later object.
    if rest.len() >= 4
        && rest[0] == "milled"
        && matches!(rest[1], "cards" | "card's" | "card")
        && rest[2..4] == ["mana", "value"]
    {
        return Some((
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::AffectedObjects,
                    ironsmith_core::EffectMetric::FirstManaValue,
                )
                .with_action(ironsmith_core::PriorEffectAction::Milled),
            ),
            offset + 4,
        ));
    }
    if rest.len() >= 4
        && rest[0..2] == ["exiled", "creature"]
        && matches!(rest[2], "cards" | "card's" | "card")
    {
        let spec = tagged(Tag::SourceExiled, "the exiled creature card");
        return match rest[3] {
            "power" => Some((Value::PowerOf(spec), offset + 4)),
            "toughness" => Some((Value::ToughnessOf(spec), offset + 4)),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_references_keep_quantities_and_semantic_antecedents() {
        for text in [
            "the mana value of that spell",
            "that Saga's mana value",
            "the power of the creature that died",
            "the number of counters it had on it",
            "the amount of mana spent to cast it",
            "the milled card's mana value",
            "the exiled creature card's power",
            "the exiled creature card's toughness",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len(), "{text}: {value:?}");
            assert!(
                !matches!(value, Value::X | Value::Fixed(_) | Value::Count(_)),
                "{text}: {value:?}"
            );
        }
        let (value, _) = parse(&["the", "milled", "cards", "mana", "value"]).unwrap();
        assert!(matches!(value, Value::PendingPriorEffectMetric(query)
            if query.action == Some(ironsmith_core::PriorEffectAction::Milled)
                && query.metric == ironsmith_core::EffectMetric::FirstManaValue));
        for words in [
            vec!["that", "sagas", "power"],
            vec!["the", "number", "of", "counters", "it", "will", "have"],
            vec!["the", "milled", "cards", "damage"],
        ] {
            assert!(parse(&words).is_none());
        }
    }
}
