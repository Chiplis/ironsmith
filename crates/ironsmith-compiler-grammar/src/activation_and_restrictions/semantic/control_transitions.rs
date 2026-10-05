use super::*;
use ironsmith_core::trigger_model::{ControlChangeDirection, ControlChangeTrigger};

pub(super) fn parse_control_transition_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    let Some(verb) = words
        .windows(3)
        .position(|part| matches!(part, ["gain" | "gains" | "lose" | "loses", "control", "of"]))
    else {
        return Ok(None);
    };
    let Some(player) = parse_trigger_subject_player_filter(&words[..verb]) else {
        return Ok(None);
    };
    let gained = matches!(words[verb], "gain" | "gains");
    let from_index = words[verb + 3..]
        .iter()
        .position(|word| *word == "from")
        .map(|index| verb + 3 + index);
    if !gained && from_index.is_some() {
        return Ok(None);
    }
    let from = if let Some(index) = from_index {
        let tail = &words[index + 1..];
        Some(if tail == ["another", "player"] {
            PlayerFilter::NotYou
        } else if let Some(player) = parse_trigger_subject_player_filter(tail) {
            player
        } else {
            return Ok(None);
        })
    } else {
        None
    };
    let start = trigger_word_token_start(tokens, verb + 3).unwrap_or(tokens.len());
    let end = from_index
        .and_then(|index| trigger_word_token_start(tokens, index))
        .unwrap_or(tokens.len());
    if start >= end {
        return Ok(None);
    }
    let subject = &tokens[start..end];
    let filter = if let Some(surface) = source_reference_surface_for_trigger_subject(subject) {
        ObjectFilter::source_with_surface(surface)
    } else if subject
        .first()
        .is_some_and(|token| token.is_word("that") || token.is_word("those"))
        && subject.len() > 1
    {
        parse_object_filter_lexed(&subject[1..], false)?.match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        )
    } else if let Some(filter) = parse_trigger_subject_filter_lexed(subject)? {
        filter
    } else {
        return Ok(None);
    };
    Ok(Some(TriggerSpec::ControlChanged(ControlChangeTrigger {
        filter,
        change: if gained {
            ControlChangeDirection::Gained { player, from }
        } else {
            ControlChangeDirection::Lost { player }
        },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(text: &str) -> Option<TriggerSpec> {
        parse_control_transition_trigger(&crate::lexer::lex_line(text, 0).unwrap()).unwrap()
    }
    #[test]
    fn old_and_new_players_remain_distinct_from_the_permanent_subject() {
        let Some(TriggerSpec::ControlChanged(trigger)) =
            parse("an opponent gains control of a permanent from you")
        else {
            panic!("control trigger");
        };
        assert_eq!(
            trigger.change,
            ControlChangeDirection::Gained {
                player: PlayerFilter::Opponent,
                from: Some(PlayerFilter::You)
            }
        );
        assert!(!trigger.filter.source);
        let Some(TriggerSpec::ControlChanged(trigger)) =
            parse("you gain control of this enchantment from another player")
        else {
            panic!("source gain");
        };
        assert!(trigger.filter.source);
        assert_eq!(
            trigger.change,
            ControlChangeDirection::Gained {
                player: PlayerFilter::You,
                from: Some(PlayerFilter::NotYou)
            }
        );
        let Some(TriggerSpec::ControlChanged(trigger)) = parse("you lose control of this artifact")
        else {
            panic!("source loss");
        };
        assert!(trigger.filter.source);
        assert_eq!(
            trigger.change,
            ControlChangeDirection::Lost {
                player: PlayerFilter::You
            }
        );
    }
    #[test]
    fn incomplete_unknown_and_unconsumed_control_clauses_fail_closed() {
        for text in [
            "you gain control of",
            "other gains control of this artifact",
            "you lose control of target",
            "you gain control of this artifact from",
            "you lose control of this creature from you",
        ] {
            let parsed =
                parse_control_transition_trigger(&crate::lexer::lex_line(text, 0).unwrap());
            assert!(!matches!(parsed, Ok(Some(_))), "{text}");
        }
    }
}
