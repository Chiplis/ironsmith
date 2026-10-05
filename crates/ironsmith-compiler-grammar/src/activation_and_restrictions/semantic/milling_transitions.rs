use super::*;

pub(super) fn parse_milling_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    let (player, start, end, per_player) =
        if let [subject @ .., "is" | "are", "milled"] = words.as_slice() {
            (PlayerFilter::Any, 0, subject.len(), false)
        } else if let Some(verb) = words
            .iter()
            .position(|word| matches!(*word, "mill" | "mills"))
        {
            let Some(player) = parse_trigger_subject_player_filter(&words[..verb]) else {
                return Ok(None);
            };
            (player, verb + 1, words.len(), true)
        } else {
            return Ok(None);
        };
    let subject = &words[start..end];
    let one_or_more = subject.starts_with(&["one", "or", "more"]);
    let start = start + if one_or_more { 3 } else { 0 };
    let subject = &words[start..end];
    if !subject
        .last()
        .is_some_and(|word| matches!(*word, "card" | "cards"))
    {
        return Ok(None);
    }
    let mut filter = if matches!(subject, ["card" | "cards"] | ["a", "card"]) {
        None
    } else {
        let start = trigger_word_token_start(tokens, start).unwrap_or(tokens.len());
        let end = trigger_word_token_start(tokens, end).unwrap_or(tokens.len());
        Some(
            parse_trigger_subject_filter_lexed(&tokens[start..end])?.ok_or_else(|| {
                CardTextError::ParseError("milling requires a complete card filter".into())
            })?,
        )
    };
    if let Some(filter) = &mut filter {
        // A completed mill can end in any public replacement destination.
        filter.zone = None;
    }
    Ok(Some(TriggerSpec::CardsMilled {
        player,
        filter,
        one_or_more,
        per_player,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn milling_preserves_actor_filter_and_quantifier_scope() {
        let parse = |text| {
            parse_milling_trigger(&crate::lexer::lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap()
        };
        let TriggerSpec::CardsMilled {
            player,
            filter,
            one_or_more,
            per_player,
        } = parse("an opponent mills one or more nonland cards")
        else {
            panic!("mill");
        };
        assert_eq!(player, PlayerFilter::Opponent);
        assert!(one_or_more && per_player);
        assert!(
            filter
                .unwrap()
                .excluded_card_types
                .contains(&crate::types::CardType::Land)
        );
        let TriggerSpec::CardsMilled {
            player,
            one_or_more,
            per_player,
            ..
        } = parse("one or more nonland cards are milled")
        else {
            panic!("mill");
        };
        assert_eq!(player, PlayerFilter::Any);
        assert!(one_or_more && !per_player);
        assert!(matches!(
            parse("a player mills a card"),
            TriggerSpec::CardsMilled {
                filter: None,
                one_or_more: false,
                per_player: true,
                ..
            }
        ));
    }
    #[test]
    fn milling_requires_complete_surfaces() {
        for text in [
            "a player mills",
            "unknown mills a card",
            "a player mills a card during your turn",
            "one or more cards are milled this way",
            "this card is put into a graveyard from a library",
        ] {
            assert!(
                parse_milling_trigger(&crate::lexer::lex_line(text, 0).unwrap())
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
    }
}
