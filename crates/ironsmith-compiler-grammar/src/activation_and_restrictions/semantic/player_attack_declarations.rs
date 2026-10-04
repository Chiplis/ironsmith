use super::*;
use ironsmith_core::trigger_model::PlayerAttackGrouping;
fn player(words: &[&str]) -> Option<PlayerFilter> {
    // Singular, complete participants only. The shared subject helper also
    // accepts quantified and prefix-only legacy forms; those must not erase a
    // grouping condition or consume an unknown tail at this event boundary.
    match words {
        ["enchanted", "player" | "opponent"] | ["the", "enchanted", "player"] => {
            Some(PlayerFilter::TaggedPlayer("enchanted".into()))
        }
        ["you"]
        | ["a" | "any" | "another", "player"]
        | ["player"]
        | ["an", "opponent"]
        | ["opponent"]
        | ["a", "player", "other", "than", "you" | "yourself"]
        | ["the", "chosen", "player"]
        | ["chosen" | "that", "player"] => parse_trigger_subject_player_filter(words),
        _ => None,
    }
}
pub(super) fn parse_player_attack_declaration(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let view = ActivationRestrictionCompatWords::new(tokens);
    let words = view.to_word_refs();
    if let Some(subject) = words.strip_suffix(&["is", "attacked"])
        && let Some(defender) = player(subject)
    {
        return Ok(Some(TriggerSpec::PlayerAttackDeclaration {
            attacker: PlayerFilter::Any,
            defender,
            grouping: PlayerAttackGrouping::Defender,
        }));
    }
    let Some(verb) = words
        .iter()
        .position(|word| matches!(*word, "attack" | "attacks"))
    else {
        return Ok(None);
    };
    let Some(attacker) = player(&words[..verb]) else {
        return Ok(None);
    };
    let recipient = &words[verb + 1..];
    let (recipient, grouping) =
        if let Some(rest) = recipient.strip_prefix(&["one", "or", "more", "of"]) {
            (rest, PlayerAttackGrouping::Attacker)
        } else {
            (recipient, PlayerAttackGrouping::Pair)
        };
    let defender = if matches!(recipient, ["your", "opponents"]) {
        Some(PlayerFilter::Opponent)
    } else {
        player(recipient)
    };
    Ok(
        defender.map(|defender| TriggerSpec::PlayerAttackDeclaration {
            attacker,
            defender,
            grouping,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_attack_players_keep_distinct_roles_and_grouping() {
        let parse = |text| {
            parse_player_attack_declaration(&crate::lexer::lex_line(text, 0).unwrap())
                .unwrap()
                .unwrap()
        };
        assert!(
            matches!(parse("enchanted player is attacked"), TriggerSpec::PlayerAttackDeclaration {
            attacker: PlayerFilter::Any, defender: PlayerFilter::TaggedPlayer(tag), grouping: PlayerAttackGrouping::Defender,
        } if tag.as_str() == "enchanted")
        );
        assert!(matches!(
            parse("a player attacks one or more of your opponents"),
            TriggerSpec::PlayerAttackDeclaration {
                attacker: PlayerFilter::Any,
                defender: PlayerFilter::Opponent,
                grouping: PlayerAttackGrouping::Attacker,
            }
        ));
        assert!(matches!(
            parse("an opponent attacks you"),
            TriggerSpec::PlayerAttackDeclaration {
                attacker: PlayerFilter::Opponent,
                defender: PlayerFilter::You,
                grouping: PlayerAttackGrouping::Pair,
            }
        ));
        for text in [
            "unknown player is attacked",
            "one or more opponents attack you",
            "an unknown player on your team attacks you",
            "the player who cast unknown attacks you",
            "a player attacks a planeswalker",
            "a player attacks you during combat",
            "enchanted creature is attacked",
        ] {
            assert!(
                parse_player_attack_declaration(&crate::lexer::lex_line(text, 0).unwrap())
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
    }
}
