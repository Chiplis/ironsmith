use super::*;
use ironsmith_core::{
    DamageHistoryQuery, DamageHistoryRecipients, DamageHistoryReduction, DamageHistorySources,
};

pub(crate) fn object_reference(words: &[&str]) -> Option<(ChooseSpec, usize)> {
    if words.len() >= 2 && this_source_surface_for_words(&words[..2]).is_some() {
        return Some((ChooseSpec::Source, 2));
    }
    if matches!(words.first(), Some(&"it" | &"itself")) {
        return Some((
            ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind().into()),
            1,
        ));
    }
    if matches!(
        words.get(..2),
        Some(["that", "creature" | "permanent" | "object"])
    ) {
        return Some((
            ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind().into()),
            2,
        ));
    }
    None
}

pub(super) fn parse(words: &[&str]) -> Option<(Value, usize)> {
    const OCCURRENCE: &[&str] = &[
        "the",
        "greatest",
        "amount",
        "of",
        "damage",
        "dealt",
        "by",
        "a",
        "source",
        "to",
        "a",
        "permanent",
        "or",
        "player",
        "this",
        "turn",
    ];
    if words.starts_with(OCCURRENCE) {
        return Some((
            Value::DamageHistory(Box::new(DamageHistoryQuery {
                sources: DamageHistorySources::Any,
                recipients: DamageHistoryRecipients::Any,
                combat: None,
                reduction: DamageHistoryReduction::LargestSourceRecipientOccurrence,
            })),
            OCCURRENCE.len(),
        ));
    }

    let mut offset = usize::from(words.first() == Some(&"the"));
    if words.get(offset) == Some(&"total") {
        offset += 1;
    }
    if words.get(offset..offset + 2) == Some(&["amount", "of"][..]) {
        offset += 2;
    }
    let combat = match words.get(offset) {
        Some(&"combat") => {
            offset += 1;
            Some(true)
        }
        Some(&"noncombat") => {
            offset += 1;
            Some(false)
        }
        _ => None,
    };
    if words.get(offset) != Some(&"damage") {
        return None;
    }
    offset += 1;
    if words.get(offset) == Some(&"already") {
        offset += 1;
    }
    if words.get(offset..offset + 2) != Some(&["dealt", "to"][..]) {
        return None;
    }
    // "the damage dealt to you so far this turn by artifacts" (Reverse
    // Polarity): damage to a player is a player-recipient history query.
    let recipients = if words.get(offset + 2) == Some(&"you") {
        offset += 3;
        DamageHistoryRecipients::Players(PlayerFilter::You)
    } else {
        let (recipient, used) = object_reference(words.get(offset + 2..)?)?;
        offset += 2 + used;
        DamageHistoryRecipients::Reference(Box::new(recipient))
    };
    if words.get(offset..offset + 2) == Some(&["so", "far"][..]) {
        offset += 2;
    }
    if words.get(offset..offset + 2) != Some(&["this", "turn"][..]) {
        return None;
    }
    offset += 2;
    let sources = if words.get(offset) == Some(&"by") {
        let tail = words.get(offset + 1..)?;
        match tail {
            ["sources", "they", "controlled"] => {
                offset = words.len();
                let mut filter = ObjectFilter::default();
                filter.controller = Some(PlayerFilter::IteratedPlayer);
                DamageHistorySources::Matching(filter)
            }
            ["sources", "you", "controlled"] => {
                offset = words.len();
                DamageHistorySources::Matching(ObjectFilter::default().you_control())
            }
            ["other", "sources", "named", name @ ..] if !name.is_empty() => {
                offset = words.len();
                DamageHistorySources::Matching(
                    ObjectFilter::default().other().named(name.join(" ")),
                )
            }
            // "by artifacts" / "by creatures": sources of one card type.
            [plural]
                if plural
                    .strip_suffix('s')
                    .and_then(crate::util::parse_card_type)
                    .is_some() =>
            {
                offset = words.len();
                let card_type = plural.strip_suffix('s').and_then(crate::util::parse_card_type)?;
                DamageHistorySources::Matching(ObjectFilter::default().with_type(card_type))
            }
            _ => return None,
        }
    } else {
        DamageHistorySources::Any
    };
    Some((
        Value::DamageHistory(Box::new(DamageHistoryQuery {
            sources,
            recipients,
            combat,
            reduction: DamageHistoryReduction::Total,
        })),
        offset,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quantities_keep_actor_recipient_and_historical_controller_distinct() {
        for (text, source, recipient) in [
            (
                "the damage already dealt to it this turn",
                DamageHistorySources::Any,
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.bind().into()),
            ),
            (
                "the amount of damage dealt to this creature this turn by sources they controlled",
                DamageHistorySources::Matching({
                    let mut f = ObjectFilter::default();
                    f.controller = Some(PlayerFilter::IteratedPlayer);
                    f
                }),
                ChooseSpec::Source,
            ),
            (
                "the amount of damage dealt to this creature this turn by other sources named blazing effigy",
                DamageHistorySources::Matching(
                    ObjectFilter::default().other().named("blazing effigy"),
                ),
                ChooseSpec::Source,
            ),
        ] {
            let words = text.split_whitespace().collect::<Vec<_>>();
            let (value, used) = parse(&words).unwrap();
            assert_eq!(used, words.len());
            assert_eq!(
                value,
                Value::DamageHistory(Box::new(DamageHistoryQuery {
                    sources: source,
                    recipients: DamageHistoryRecipients::Reference(Box::new(recipient)),
                    combat: None,
                    reduction: DamageHistoryReduction::Total,
                }))
            );
        }
        let tokens=crate::lexer::lex_line("3 plus the amount of damage dealt to this creature this turn by other sources named blazing effigy",0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        assert!(
            matches!(value,Value::Add(left,right) if matches!(left.as_ref(),Value::Fixed(3)) && matches!(right.as_ref(),Value::DamageHistory(_)))
        );
    }
    #[test]
    fn unsupported_occurrence_and_unknown_actor_qualifiers_do_not_become_totals() {
        for text in [
            "the greatest amount of damage dealt by sources to a permanent or player this turn",
            "the amount of damage dealt to this creature this turn by sources they own",
            "the amount of damage dealt to this creature last turn",
            "the amount of damage dealt to this creature this turn by other sources named",
        ] {
            assert!(
                parse(&text.split_whitespace().collect::<Vec<_>>()).is_none(),
                "{text}"
            );
        }
    }
    #[test]
    fn greatest_source_recipient_occurrence_is_not_a_source_turn_total() {
        let words =
            "the greatest amount of damage dealt by a source to a permanent or player this turn"
                .split_whitespace()
                .collect::<Vec<_>>();
        let (value, used) = parse(&words).unwrap();
        assert_eq!(used, words.len());
        assert!(
            matches!(value, Value::DamageHistory(query) if query.reduction == DamageHistoryReduction::LargestSourceRecipientOccurrence)
        );
    }
}
