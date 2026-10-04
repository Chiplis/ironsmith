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
    let bearer_len = if rest.starts_with(&["your", "ring"])
        && rest
            .get(2)
            .is_some_and(|word| matches!(*word, "bearer" | "bearers" | "bearer's"))
    {
        Some(3)
    } else if rest.first() == Some(&"your")
        && rest
            .get(1)
            .is_some_and(|word| matches!(*word, "ring-bearer" | "ring-bearers" | "ring-bearer's"))
    {
        Some(2)
    } else {
        None
    };
    if let Some(length) = bearer_len {
        let mut filter = ObjectFilter::default().ring_bearer().you_control();
        filter.zone = Some(crate::zone::Zone::Battlefield);
        // A player has at most one current bearer. The aggregate is the
        // scalar characteristic when present and zero when none exists,
        // without inventing a target or a stale choice/event reference.
        match rest.get(length..) {
            Some(["power", ..]) => return Some((Value::TotalPower(filter), offset + length + 1)),
            Some(["toughness", ..]) => {
                return Some((Value::TotalToughness(filter), offset + length + 1));
            }
            Some(["mana", "value", ..]) => {
                return Some((Value::TotalManaValue(filter), offset + length + 2));
            }
            _ => {}
        }
    }

    // A source-relative attachment host is a live relationship, including
    // during cost determination where there is no resolution tag table.
    // Requiring this exact source to be attached avoids the generic
    // "equipped" state fallback when the source itself is unattached.
    if matches!(rest.first(), Some(&"equipped" | &"enchanted"))
        && rest
            .get(1)
            .is_some_and(|noun| matches!(*noun, "creature" | "creatures" | "creature's"))
    {
        let mut host = ObjectFilter::creature();
        host.with_attached_object = Some(Box::new(ObjectFilter::source()));
        let spec = Box::new(ChooseSpec::All(host).with_surface_hint(
            ChooseSpecSurfaceHint::SourceReference(SourceReferenceSurface::ThisPermanentType(
                format!("{} creature", rest[0]),
            )),
        ));
        match rest.get(2..) {
            Some(["power", ..]) => return Some((Value::PowerOf(spec), offset + 3)),
            Some(["toughness", ..]) => return Some((Value::ToughnessOf(spec), offset + 3)),
            Some(["mana", "value", ..]) => return Some((Value::ManaValueOf(spec), offset + 4)),
            _ => {}
        }
    }

    if rest.starts_with(&["number", "of", "times", "that", "spell", "was", "kicked"]) {
        return Some((
            Value::KicksPaidOf(tagged(Tag::It, "that spell")),
            offset + 7,
        ));
    }
    if rest.len() >= 3 && rest[0] == "target" && matches!(rest[1], "creatures" | "creature's") {
        let spec = Box::new(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature(),
        )));
        match rest.get(2..) {
            Some(["power", ..]) => return Some((Value::PowerOf(spec), offset + 3)),
            Some(["toughness", ..]) => return Some((Value::ToughnessOf(spec), offset + 3)),
            _ => {}
        }
    }
    // A definite possessive retains the referenced object, rather than the
    // resolving spell as damage source. Destroy keeps departure LKI; a live
    // indestructible object is still read from the same tagged identity.
    if offset == 1 && rest.len() >= 2 {
        let noun = rest[0].trim_end_matches("'s").trim_end_matches('s');
        if matches!(
            noun,
            "creature" | "artifact" | "enchantment" | "permanent" | "planeswalker" | "card"
        ) {
            let spec = tagged(Tag::It, &format!("the {noun}"));
            match rest.get(1..) {
                Some(["power", ..]) => return Some((Value::PowerOf(spec), 3)),
                Some(["toughness", ..]) => return Some((Value::ToughnessOf(spec), 3)),
                Some(["mana", "value", ..]) => return Some((Value::ManaValueOf(spec), 4)),
                _ => {}
            }
        }
    }
    // A spell's characteristics are those of the exact prior/event stack
    // object. The spelling must not collapse to the damage recipient or to
    // the spell/ability currently resolving.
    if rest.len() >= 3 && rest[0] == "that" && matches!(rest[1], "spell's" | "spells") {
        let spec = tagged(Tag::It, "that spell");
        match rest.get(2..) {
            Some(["power", ..]) => return Some((Value::PowerOf(spec), offset + 3)),
            Some(["toughness", ..]) => return Some((Value::ToughnessOf(spec), offset + 3)),
            Some(["mana", "value", ..]) => return Some((Value::ManaValueOf(spec), offset + 4)),
            _ => {}
        }
    }
    if rest.starts_with(&["tapped", "creatures", "power"])
        || rest.starts_with(&["tapped", "creature's", "power"])
    {
        return Some((
            Value::PowerOf(Box::new(
                ChooseSpec::Tagged(crate::tag::TagKey::from(
                    crate::tag::PRIOR_TAPPED_OBJECT_QUANTITY_TAG,
                ))
                .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(
                    SourceReferenceSurface::ThisPermanentType("the tapped creature".into()),
                )),
            )),
            offset + 3,
        ));
    }
    for (quantity, length) in [
        (&["power"][..], 1),
        (&["toughness"][..], 1),
        (&["mana", "value"][..], 2),
    ] {
        if rest.starts_with(quantity)
            && rest.get(length..).is_some_and(|tail| {
                tail.starts_with(&["of", "the", "card", "returned", "this", "way"])
            })
        {
            let spec = Box::new(ChooseSpec::Tagged(crate::tag::TagKey::from(
                crate::tag::RETURNED_THIS_WAY_QUANTITY_TAG,
            )));
            let value = match quantity {
                ["power"] => Value::PowerOf(spec),
                ["toughness"] => Value::ToughnessOf(spec),
                _ => Value::ManaValueOf(spec),
            };
            return Some((value, offset + length + 6));
        }
    }
    if words.starts_with(&["its", "base", "power"]) {
        return Some((Value::BasePowerOf(tagged(Tag::It, "it")), 3));
    }
    if words.starts_with(&["its", "loyalty"]) {
        return Some((
            Value::CountersOn(
                tagged(Tag::It, "it"),
                Some(crate::object::CounterType::Loyalty),
            ),
            2,
        ));
    }
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
        && matches!(rest[0], "milled" | "revealed")
        && matches!(rest[1], "cards" | "card's" | "card")
        && rest[2..4] == ["mana", "value"]
    {
        return Some((
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::AffectedObjects,
                    ironsmith_core::EffectMetric::FirstManaValue,
                )
                .with_action(if rest[0] == "milled" {
                    ironsmith_core::PriorEffectAction::Milled
                } else {
                    ironsmith_core::PriorEffectAction::Revealed
                }),
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

#[cfg(test)]
mod damage_reference_tests {
    use super::*;
    #[test]
    fn definite_cost_and_returned_quantities_keep_distinct_identities() {
        for (text, expected_tag) in [
            (
                "the creature's power",
                crate::tag::CompilerReferenceTag::It.as_str(),
            ),
            (
                "the artifact's mana value",
                crate::tag::CompilerReferenceTag::It.as_str(),
            ),
            (
                "the tapped creature's power",
                crate::tag::PRIOR_TAPPED_OBJECT_QUANTITY_TAG,
            ),
            (
                "the power of the card returned this way",
                crate::tag::RETURNED_THIS_WAY_QUANTITY_TAG,
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = super::super::parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len(), "{text}");
            let spec = match value.unhinted() {
                Value::PowerOf(spec) | Value::ManaValueOf(spec) => spec,
                _ => panic!("{value:?}"),
            };
            assert!(
                matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == expected_tag),
                "{text}: {value:?}"
            );
        }
    }
}

#[cfg(test)]
mod attachment_host_tests {
    use super::*;
    #[test]
    fn explicit_attachment_quantity_requires_the_exact_source_attachment_relationship() {
        let tokens = crate::lexer::lex_line("equipped creature's power", 0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        let Value::PowerOf(spec) = value.unhinted() else {
            panic!("{value:?}");
        };
        let ChooseSpec::All(filter) = spec.base() else {
            panic!("{spec:?}");
        };
        assert!(filter.with_attached_object.as_ref().unwrap().source);
        assert!(filter.tagged_constraints.is_empty());
        assert!(matches!(
            filter.card_types.as_slice(),
            [crate::types::CardType::Creature]
        ));
    }
}

#[cfg(test)]
mod spell_quantity_tests {
    use super::*;
    #[test]
    fn referenced_spell_power_and_paid_kicks_are_not_the_resolving_source() {
        for (text, kicks) in [
            ("that spell's power", false),
            ("the number of times that spell was kicked", true),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len());
            let spec = match &value {
                Value::KicksPaidOf(spec) if kicks => spec,
                Value::PowerOf(spec) if !kicks => spec,
                _ => panic!("{value:?}"),
            };
            assert!(matches!(spec.base(),ChooseSpec::Tagged(tag) if tag.as_str()=="__it__"));
            assert_eq!(
                spec.source_reference_surface(),
                Some(&SourceReferenceSurface::ThisPermanentType(
                    "that spell".into()
                ))
            );
        }
    }
}

#[cfg(test)]
mod targeted_characteristic_tests {
    use super::*;
    #[test]
    fn target_possessive_is_an_explicit_choice_not_a_source_or_prior_object() {
        for (text, power) in [
            ("target creature's power", true),
            ("target creature's toughness", false),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len());
            let spec = match value {
                Value::PowerOf(spec) if power => spec,
                Value::ToughnessOf(spec) if !power => spec,
                _ => panic!(),
            };
            assert!(spec.is_target());
            let ChooseSpec::Object(filter) = spec.base() else {
                panic!()
            };
            assert!(
                filter
                    .card_types
                    .contains(&crate::types::CardType::Creature)
            );
            assert!(!filter.source);
        }
        assert!(parse(&["target", "creatures", "mystery"]).is_none());
    }
}

#[cfg(test)]
mod ring_quantity_tests {
    use super::*;
    #[test]
    fn unique_current_bearer_characteristics_are_zero_for_an_empty_designation() {
        for text in [
            "your Ring-bearer's power",
            "your Ring-bearer's toughness",
            "your Ring-bearer's mana value",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len(), "{text}");
            let filter = match value.unhinted() {
                Value::TotalPower(filter)
                | Value::TotalToughness(filter)
                | Value::TotalManaValue(filter) => filter,
                other => panic!("wrong current designation quantity: {other:?}"),
            };
            assert_eq!(filter, &ObjectFilter::your_ring_bearer());
        }
    }
}
