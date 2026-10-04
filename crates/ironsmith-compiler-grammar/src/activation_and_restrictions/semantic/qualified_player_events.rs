use super::*;
fn player(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["enchanted", "player" | "opponent"] => {
            Some(PlayerFilter::TaggedPlayer("enchanted".into()))
        }
        ["you"] | ["a" | "any" | "another", "player"] | ["an", "opponent"] => {
            parse_trigger_subject_player_filter(words)
        }
        _ => None,
    }
}
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<TriggerSpec>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    let Some(verb) = words.iter().enumerate().find_map(|(index, word)| {
        (matches!(*word, "draw" | "draws" | "gain" | "gains")
            && words
                .get(index + 1)
                .is_some_and(|next| matches!(*next, "a" | "your" | "life")))
        .then_some(index)
    }) else {
        return Ok(None);
    };
    let subject = &words[..verb];
    let who = subject
        .windows(2)
        .position(|part| part == ["who", "controls"]);
    let Some(player) = player(who.map_or(subject, |index| &subject[..index])) else {
        return Ok(None);
    };
    let tail = &words[verb + 1..];
    let trigger = if matches!(words[verb], "draw" | "draws") {
        if matches!(
            tail,
            [
                "your", "first", "card", "during", "each", "of", "your", "draw", "steps"
            ]
        ) && player == PlayerFilter::You
        {
            TriggerSpec::PlayerDrawsFirstCardInOwnDrawStep(player.clone())
        } else {
            let Some(rest) = tail.strip_prefix(&["a", "card"]) else {
                return Ok(None);
            };
            if rest.is_empty() {
                // Preserve the legacy ordinary draw route except the newly
                // authenticated enchanted-opponent/qualified subject.
                if who.is_none() && subject != ["enchanted", "opponent"] {
                    return Ok(None);
                }
                TriggerSpec::PlayerDrawsCard(player.clone())
            } else {
                let turn = match rest {
                    ["during", "your", "turn"] => PlayerFilter::You,
                    ["during", "their", "turn"] => PlayerFilter::IteratedPlayer,
                    ["during", "an", "opponents" | "opponent's", "turn"] => PlayerFilter::Opponent,
                    _ => return Ok(None),
                };
                TriggerSpec::PlayerDrawsCardDuringTurn {
                    player: player.clone(),
                    during_turn: turn,
                }
            }
        }
    } else {
        if who.is_none() {
            return Ok(None);
        }
        let during_turn = match tail {
            ["life"] => None,
            ["life", "during", "their", "turn"] => Some(PlayerFilter::IteratedPlayer),
            ["life", "during", "your", "turn"] => Some(PlayerFilter::You),
            _ => return Ok(None),
        };
        TriggerSpec::PlayerGainsLife {
            player: player.clone(),
            during_turn,
        }
    };
    let Some(who) = who else {
        return Ok(Some(trigger));
    };
    let positions = crate::lexer::parser_token_word_positions(tokens);
    let Some(&(start, _)) = positions.get(who + 2) else {
        return Ok(None);
    };
    let end = positions[verb].0;
    if start >= end {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(&tokens[start..end], false)?;
    let surface = format!("that player controls {}", filter.description());
    Ok(Some(TriggerSpec::ConditionQualified {
        trigger: Box::new(trigger),
        condition: crate::cards::builders::PredicateAst::Player(
            crate::cards::builders::PlayerPredicateAst::PlayerControls {
                player: crate::cards::builders::PlayerAst::That,
                filter,
            },
        ),
        surface,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn player_and_event_qualifications_keep_distinct_roles_and_exact_boundaries() {
        fn read(text: &str) -> Result<Option<TriggerSpec>, CardTextError> { parse(&crate::lexer::lex_line(text, 0).unwrap()) }
        assert!(
            matches!(read("enchanted opponent draws a card").unwrap(), Some(TriggerSpec::PlayerDrawsCard(PlayerFilter::TaggedPlayer(tag))) if tag.as_str() == "enchanted")
        );
        assert!(matches!(
            read("you draw your first card during each of your draw steps").unwrap(),
            Some(TriggerSpec::PlayerDrawsFirstCardInOwnDrawStep(
                PlayerFilter::You
            ))
        ));
        assert!(matches!(
            read("you draw a card during an opponent's turn").unwrap(),
            Some(TriggerSpec::PlayerDrawsCardDuringTurn {
                player: PlayerFilter::You,
                during_turn: PlayerFilter::Opponent
            })
        ));
        for verb in ["draws a card", "gains life"] {
            let text = format!(
                "an opponent who controls an artifact named Witness Relic {verb} during their turn"
            );
            let Some(TriggerSpec::ConditionQualified {
                trigger, condition, ..
            }) = read(&text).unwrap()
            else {
                panic!("missing qualified subject");
            };
            assert!(matches!(
                condition,
                crate::cards::builders::PredicateAst::Player(
                    crate::cards::builders::PlayerPredicateAst::PlayerControls {
                        player: crate::cards::builders::PlayerAst::That,
                        ..
                    }
                )
            ));
            assert!(matches!(
                *trigger,
                TriggerSpec::PlayerDrawsCardDuringTurn {
                    player: PlayerFilter::Opponent,
                    during_turn: PlayerFilter::IteratedPlayer
                } | TriggerSpec::PlayerGainsLife {
                    player: PlayerFilter::Opponent,
                    during_turn: Some(PlayerFilter::IteratedPlayer)
                }
            ));
        }
        for text in [
            "an unknown opponent draws a card",
            "you draw your first card during combat",
            "you draw a card during an opponent's turn and discard a card",
            "an opponent who controls draws a card",
            "you draw a card before your turn",
        ] {
            assert!(!matches!(read(text), Ok(Some(_))), "{text}");
        }
    }
}
