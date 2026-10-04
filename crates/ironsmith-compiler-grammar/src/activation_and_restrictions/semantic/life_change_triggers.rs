use super::*;

fn singular_player(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"]
        | ["a" | "any" | "another", "player"]
        | ["player"]
        | ["an", "opponent"]
        | ["opponent"]
        | ["a", "player", "other", "than", "you" | "yourself"]
        | ["enchanted", "player" | "opponent"] => parse_trigger_subject_player_filter(words),
        _ => None,
    }
}
fn life_event(
    player: PlayerFilter,
    gained: bool,
    during_turn: Option<PlayerFilter>,
) -> TriggerSpec {
    match (gained, player, during_turn) {
        (true, PlayerFilter::You, None) => TriggerSpec::YouGainLife,
        (true, PlayerFilter::You, Some(turn)) => TriggerSpec::YouGainLifeDuringTurn(turn),
        (true, player, during_turn) => TriggerSpec::PlayerGainsLife {
            player,
            during_turn,
        },
        (false, player, None) => TriggerSpec::PlayerLosesLife(player),
        (false, player, Some(during_turn)) => TriggerSpec::PlayerLosesLifeDuringTurn {
            player,
            during_turn,
        },
    }
}
pub(super) fn parse_life_change_trigger(tokens: &[OwnedLexToken]) -> Option<TriggerSpec> {
    let words = crate::lexer::token_word_refs(tokens);
    let (words, during_turn) = if let Some(words) = words.strip_suffix(&["during", "your", "turn"])
    {
        (words, Some(PlayerFilter::You))
    } else if let Some(words) = words.strip_suffix(&["during", "their", "turn"]) {
        (words, Some(PlayerFilter::IteratedPlayer))
    } else {
        (words.as_slice(), None)
    };
    let words = words.strip_suffix(&["life"])?;
    if let Some(subject) = words
        .strip_suffix(&["gain", "or", "lose"])
        .or_else(|| words.strip_suffix(&["gains", "or", "loses"]))
    {
        let player = singular_player(subject)?;
        return Some(TriggerSpec::Either(
            Box::new(life_event(player.clone(), true, during_turn.clone())),
            Box::new(life_event(player, false, during_turn)),
        ));
    }
    let (verb, subject) = words.split_last()?;
    let gained = match *verb {
        "gain" | "gains" => true,
        "lose" | "loses" => false,
        _ => return None,
    };
    Some(life_event(singular_player(subject)?, gained, during_turn))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn life_change_subject_union_retains_both_turn_guards() {
        let parse = |text| parse_life_change_trigger(&crate::lexer::lex_line(text, 0).unwrap());
        assert!(matches!(
            parse("an opponent gains life"),
            Some(TriggerSpec::PlayerGainsLife {
                player: PlayerFilter::Opponent,
                during_turn: None
            })
        ));
        let Some(TriggerSpec::Either(gain, loss)) = parse("you gain or lose life during your turn")
        else {
            panic!("missing shared event");
        };
        assert!(matches!(
            *gain,
            TriggerSpec::YouGainLifeDuringTurn(PlayerFilter::You)
        ));
        assert!(matches!(
            *loss,
            TriggerSpec::PlayerLosesLifeDuringTurn {
                player: PlayerFilter::You,
                during_turn: PlayerFilter::You
            }
        ));
        for text in [
            "an unknown opponent gains life",
            "a player gains life and draws a card",
            "you gain or lose life during combat",
            "one or more opponents gain life",
        ] {
            assert!(parse(text).is_none(), "{text}");
        }
    }
}
