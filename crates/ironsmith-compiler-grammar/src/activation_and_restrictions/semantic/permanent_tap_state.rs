use super::*;

/// Passive state transitions have an object subject, not an action's player.
/// Preserve plural `become` and complete event-time turn qualifications.
pub(super) fn parse_permanent_tap_state_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let all_words = view.to_word_refs();
    let (words, during_your_turn) =
        if let Some(words) = all_words.strip_suffix(&["during", "your", "turn"]) {
            (words, true)
        } else {
            (all_words.as_slice(), false)
        };
    let [
        subject @ ..,
        "become" | "becomes",
        state @ ("tapped" | "untapped"),
    ] = words
    else {
        return Ok(None);
    };
    // "this creature leaves the battlefield or becomes untapped" is a
    // compound trigger whose first event is its own verb phrase, not a
    // subject; leave it to the disjunctive-trigger split.
    if subject.is_empty() || matches!(subject.last(), Some(&"or" | &"and")) {
        return Ok(None);
    }
    let grouped = subject.starts_with(&["one", "or", "more"]);
    if grouped && subject.len() == 3 {
        return Ok(None);
    }
    let start = if grouped {
        trigger_word_token_start(tokens, 3).unwrap_or(tokens.len())
    } else {
        0
    };
    let end = trigger_word_token_start(tokens, subject.len()).unwrap_or(tokens.len());
    let subject_tokens = &tokens[start..end];
    let trigger = match parse_trigger_subject_filter_lexed(subject_tokens)? {
        Some(filter) => {
            if *state == "tapped" {
                if grouped {
                    TriggerSpec::PermanentBecomesTappedOneOrMore(filter)
                } else {
                    TriggerSpec::PermanentBecomesTapped(filter)
                }
            } else {
                TriggerSpec::PermanentBecomesUntapped {
                    filter,
                    one_or_more: grouped,
                }
            }
        }
        None if *state == "tapped" => TriggerSpec::ThisBecomesTapped,
        None => TriggerSpec::ThisBecomesUntapped,
    };
    Ok(Some(if during_your_turn {
        TriggerSpec::ConditionQualified {
            trigger: Box::new(trigger),
            condition: crate::cards::builders::PredicateAst::YourTurn,
            surface: "during your turn".to_owned(),
        }
    } else {
        trigger
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn passive_tap_state_keeps_subject_quantifier_and_turn_qualification() {
        let parse = |text| {
            parse_permanent_tap_state_trigger(&lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap()
        };
        let TriggerSpec::PermanentBecomesTappedOneOrMore(filter) =
            parse("one or more nontoken Merfolk you control become tapped")
        else {
            panic!("grouped tap")
        };
        assert!(filter.nontoken);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        let TriggerSpec::PermanentBecomesUntapped {
            filter,
            one_or_more,
        } = parse("a permanent you control becomes untapped")
        else {
            panic!("filtered untap")
        };
        assert!(!one_or_more);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(matches!(
            parse("this creature become tapped"),
            TriggerSpec::ThisBecomesTapped
        ));
        assert!(matches!(
            parse("this creature become untapped"),
            TriggerSpec::ThisBecomesUntapped
        ));
        assert!(matches!(
            parse("a permanent becomes tapped during your turn"),
            TriggerSpec::ConditionQualified {
                condition: crate::cards::builders::PredicateAst::YourTurn,
                ..
            }
        ));
    }

    #[test]
    fn passive_tap_state_does_not_drop_actor_cost_or_other_qualifiers() {
        for text in [
            "one or more become tapped",
            "you tap an untapped creature an opponent controls",
            "this creature becomes tapped to pay a teamwork cost",
            "a permanent becomes tapped during combat",
        ] {
            assert!(
                parse_permanent_tap_state_trigger(&lex_line(text, 0).unwrap())
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
    }
}

pub(super) fn parse_player_tap_state_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    let (words, during_untap_step) =
        if let Some(words) = words.strip_suffix(&["during", "your", "untap", "step"]) {
            (words, Some(PlayerFilter::You))
        } else {
            (words.as_slice(), None)
        };
    let Some(verb) = words
        .iter()
        .position(|word| matches!(*word, "tap" | "taps" | "untap" | "untaps"))
    else {
        return Ok(None);
    };
    let Some(player) = parse_trigger_subject_player_filter(&words[..verb]) else {
        return Ok(None);
    };
    let tapped = matches!(words[verb], "tap" | "taps");
    let recipient = &words[verb + 1..];
    if recipient.is_empty() {
        return Ok(None);
    }
    let one_or_more = recipient.starts_with(&["one", "or", "more"]);
    let start_word = verb + 1 + if one_or_more { 3 } else { 0 };
    let start = trigger_word_token_start(tokens, start_word).unwrap_or(tokens.len());
    let end = trigger_word_token_start(tokens, words.len()).unwrap_or(tokens.len());
    if start >= end {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(&tokens[start..end], false)?;
    Ok(Some(TriggerSpec::PlayerChangesTapState {
        player,
        filter,
        tapped,
        one_or_more,
        during_untap_step,
    }))
}

#[cfg(test)]
mod actor_tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn actor_tap_filter_keeps_untapped_origin_and_actor_separate_from_recipient_controller() {
        let parsed = parse_player_tap_state_trigger(
            &lex_line(
                "you tap one or more untapped creatures your opponents control",
                0,
            )
            .unwrap(),
        )
        .unwrap()
        .unwrap();
        let TriggerSpec::PlayerChangesTapState {
            player,
            filter,
            tapped,
            one_or_more,
            during_untap_step,
        } = parsed
        else {
            panic!("actor tap state")
        };
        assert_eq!(player, PlayerFilter::You);
        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
        assert!(filter.untapped);
        assert!(tapped && one_or_more);
        assert!(during_untap_step.is_none());
        let parsed = parse_player_tap_state_trigger(
            &lex_line("you untap one or more permanents during your untap step", 0).unwrap(),
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            parsed,
            TriggerSpec::PlayerChangesTapState {
                tapped: false,
                one_or_more: true,
                during_untap_step: Some(PlayerFilter::You),
                ..
            }
        ));
    }
}
