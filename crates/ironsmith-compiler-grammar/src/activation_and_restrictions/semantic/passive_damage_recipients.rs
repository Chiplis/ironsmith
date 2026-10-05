//! Passive receipt clauses retain recipient and single-source qualifications.
use super::*;

fn recipient(
    tokens: &[OwnedLexToken],
    combat: Option<bool>,
    minimum: Option<u32>,
    single_source: bool,
) -> Result<TriggerSpec, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words.is_empty() {
        return Err(CardTextError::ParseError("missing damage recipient".into()));
    }
    // A player/object alternative describes separate recipient occurrences,
    // while a union inside one object filter keeps that filter's own syntax.
    if let Some(index) = words.iter().position(|word| *word == "or") {
        if parse_trigger_subject_player_filter(&words[..index]).is_some() {
            let end = trigger_word_token_start(tokens, index).unwrap_or(tokens.len());
            let start = trigger_word_token_start(tokens, index + 1).unwrap_or(tokens.len());
            return Ok(TriggerSpec::Either(
                Box::new(recipient(&tokens[..end], combat, minimum, single_source)?),
                Box::new(recipient(&tokens[start..], combat, minimum, single_source)?),
            ));
        }
    }
    let target = if let Some(player) = parse_trigger_subject_player_filter(&words) {
        ChooseSpec::Player(player)
    } else if let Some(surface) = source_reference_surface_for_trigger_subject(tokens) {
        ChooseSpec::Object(ObjectFilter::source_with_surface(surface))
    } else {
        let filter = parse_trigger_subject_filter_lexed(tokens)?.ok_or_else(|| {
            CardTextError::ParseError("damage recipient is not an authenticated object".into())
        })?;
        ChooseSpec::Object(filter)
    };
    Ok(TriggerSpec::DamageReceived {
        target,
        combat,
        minimum,
        single_source,
    })
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    // "combat damage is dealt to you [or a planeswalker you control]".
    for (prefix, combat) in [
        (&["combat", "damage", "is", "dealt", "to"][..], Some(true)),
        (
            &["noncombat", "damage", "is", "dealt", "to"][..],
            Some(false),
        ),
        (&["damage", "is", "dealt", "to"][..], None),
    ] {
        if words.starts_with(prefix) {
            let start = trigger_word_token_start(tokens, prefix.len()).unwrap_or(tokens.len());
            return recipient(&tokens[start..], combat, None, false).map(Some);
        }
    }
    let Some(index) = words
        .windows(2)
        .position(|window| matches!(window, ["is" | "are", "dealt"]))
    else {
        return Ok(None);
    };
    let end = trigger_word_token_start(tokens, index).unwrap_or(tokens.len());
    let mut tail = &words[index + 2..];
    let mut minimum = None;
    if tail.len() >= 3 && tail[1..3] == ["or", "more"] {
        let Some(value) = parse_named_number(tail[0]) else {
            return Ok(None);
        };
        minimum = Some(value);
        tail = &tail[3..];
    }
    let combat = if tail.first() == Some(&"combat") {
        tail = &tail[1..];
        Some(true)
    } else if tail.first() == Some(&"noncombat") {
        tail = &tail[1..];
        Some(false)
    } else {
        None
    };
    if tail.first() != Some(&"damage") {
        return Ok(None);
    };
    tail = &tail[1..];
    let single_source = tail == ["by", "a", "single", "source"];
    if !tail.is_empty() && !single_source {
        return Ok(None);
    }
    // Unqualified passive player receipts have an existing grouped matcher.
    // That production groups sources and preserves "one or more" recipients.
    if minimum.is_none()
        && !single_source
        && combat == Some(false)
        && parse_trigger_subject_player_filter(&words[..index]).is_some()
    {
        return Ok(None);
    }
    let generic_self = source_reference_surface_for_trigger_subject(&tokens[..end]).is_some()
        && words[..index] != ["this", "creature"]
        && words[..index] != ["this"];
    if minimum.is_none() && !single_source && combat != Some(false) && !generic_self {
        return Ok(None);
    }
    // Keep excess and explicitly qualified damaging-source grammars with
    // their existing owners; this complete head has no implicit source.
    recipient(&tokens[..end], combat, minimum, single_source).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn read(text: &str) -> TriggerSpec {
        parse(&crate::lexer::lex_line(text, 0).unwrap())
            .unwrap()
            .unwrap()
    }
    #[test]
    fn thresholds_combat_and_per_source_qualifiers_are_typed() {
        let TriggerSpec::DamageReceived {
            target,
            combat,
            minimum,
            single_source,
        } = read("this creature is dealt 3 or more damage")
        else {
            panic!("threshold");
        };
        assert!(matches!(target,ChooseSpec::Object(filter) if filter.source));
        assert_eq!(combat, None);
        assert_eq!(minimum, Some(3));
        assert!(!single_source);
        let TriggerSpec::DamageReceived {
            target,
            minimum,
            single_source,
            ..
        } = read("an opponent is dealt 3 or more damage by a single source")
        else {
            panic!("single source");
        };
        assert_eq!(target, ChooseSpec::Player(PlayerFilter::Opponent));
        assert_eq!(minimum, Some(3));
        assert!(single_source);
        let TriggerSpec::DamageReceived { combat, .. } =
            read("this creature is dealt noncombat damage")
        else {
            panic!("noncombat");
        };
        assert_eq!(combat, Some(false));
        assert!(
            matches!(read("this permanent is dealt damage"),TriggerSpec::DamageReceived { target:ChooseSpec::Object(filter), .. } if filter.source)
        );
    }
    #[test]
    fn received_player_object_alternatives_preserve_distinct_recipients() {
        let TriggerSpec::Either(player, object) =
            read("combat damage is dealt to you or a planeswalker you control")
        else {
            panic!("recipient union");
        };
        assert!(matches!(
            *player,
            TriggerSpec::DamageReceived {
                target: ChooseSpec::Player(PlayerFilter::You),
                combat: Some(true),
                ..
            }
        ));
        assert!(
            matches!(*object,TriggerSpec::DamageReceived {target:ChooseSpec::Object(filter),combat:Some(true), .. } if filter.card_types.contains(&crate::types::CardType::Planeswalker) && filter.controller==Some(PlayerFilter::You))
        );
    }
    #[test]
    fn old_plain_heads_and_incomplete_qualifications_keep_their_owners() {
        for text in [
            "this creature is dealt damage",
            "a creature is dealt excess noncombat damage",
            "this creature is dealt damage by an attacking creature",
            "this creature is dealt 3 or more damage tomorrow",
            "damage is dealt to",
        ] {
            assert!(
                !matches!(
                    parse(&crate::lexer::lex_line(text, 0).unwrap()),
                    Ok(Some(_))
                ),
                "{text}"
            );
        }
    }
}
