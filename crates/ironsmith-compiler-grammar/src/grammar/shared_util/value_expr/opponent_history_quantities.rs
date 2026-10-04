use super::*;

/// A scalar participant (you/the named player) versus an authored total
/// over your opponents. Pronouns stay relative until the subject/iteration
/// binder supplies the correct player; they never imply an opponent sum.
fn participant(words: &[&str], contraction: bool) -> Option<(PlayerFilter, usize)> {
    match words {
        ["you", ..] => Some((PlayerFilter::You, 1)),
        ["youve" | "you've", ..] if contraction => Some((PlayerFilter::You, 1)),
        ["they" | "them", ..] => Some((PlayerFilter::IteratedPlayer, 1)),
        ["theyve" | "they've", ..] if contraction => Some((PlayerFilter::IteratedPlayer, 1)),
        ["that", "player", ..] => Some((PlayerFilter::IteratedPlayer, 2)),
        ["your", "opponents", ..] => Some((PlayerFilter::Opponent, 2)),
        _ => None,
    }
}

pub(super) fn parse(words: &[&str]) -> Option<(Value, usize)> {
    let mut start = usize::from(words.first() == Some(&"the"));
    if words.get(start) == Some(&"total") {
        start += 1;
    }
    if words.get(start..start + 3) == Some(&["number", "of", "cards"][..]) {
        let (player, used) = participant(&words[start + 3..], true)?;
        let mut action = start + 3 + used;
        if matches!(words.get(action), Some(&"have" | &"has")) {
            action += 1;
        }
        if words.get(action..action + 3) == Some(&["drawn", "this", "turn"][..]) {
            return Some((
                Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::CardsDrawn(player)),
                action + 3,
            ));
        }
        if words.get(action..action + 3) == Some(&["drew", "this", "way"][..]) {
            let query = ironsmith_core::PriorEffectMetricQuery::new(
                ironsmith_core::EffectMetricSource::AffectedObjects,
                ironsmith_core::EffectMetric::Count,
            )
            .with_action(ironsmith_core::PriorEffectAction::Drawn)
            .with_player(player);
            return Some((
                Value::PendingPriorEffectMetric(query)
                    .with_surface_hint(ValueSurfaceHint::CardsDrawnThisWay),
                action + 3,
            ));
        }
        return None;
    }
    if words.get(start..start + 2) == Some(&["amount", "of"][..]) {
        start += 2;
    }
    if words.get(start) == Some(&"life") {
        let (player, used) = participant(&words[start + 1..], true)?;
        let mut action = start + 1 + used;
        if matches!(words.get(action), Some(&"have" | &"has")) {
            action += 1;
        }
        if words.get(action + 1..action + 3) != Some(&["this", "turn"][..]) {
            return None;
        }
        let value = match words.get(action) {
            Some(&"lost") => Value::LifeLostThisTurn(player),
            Some(&"gained") => Value::LifeGainedThisTurn(player),
            _ => return None,
        };
        return Some((value, action + 3));
    }
    if words.get(start) != Some(&"damage") {
        return None;
    }
    start += 1;
    if words.get(start) == Some(&"already") {
        start += 1;
    }
    if words.get(start..start + 2) != Some(&["dealt", "to"][..]) {
        return None;
    }
    let (player, used) = participant(&words[start + 2..], false)?;
    let end = start + 2 + used;
    (words.get(end..end + 2) == Some(&["this", "turn"][..]))
        .then_some((Value::DamageDealtToPlayersThisTurn(player), end + 2))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn damage_and_life_loss_are_distinct_typed_turn_totals() {
        for text in [
            "the total amount of life your opponents lost this turn",
            "the total life your opponents lost this turn",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len());
            assert_eq!(value, Value::LifeLostThisTurn(PlayerFilter::Opponent));
        }
        for text in [
            "the damage dealt to your opponents this turn",
            "the damage already dealt to your opponents this turn",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len());
            assert_eq!(
                value,
                Value::DamageDealtToPlayersThisTurn(PlayerFilter::Opponent)
            );
        }
        for words in [
            vec![
                "the",
                "damage",
                "dealt",
                "by",
                "your",
                "opponents",
                "this",
                "turn",
            ],
            vec!["the", "life", "your", "opponents", "gained", "last", "turn"],
        ] {
            assert!(parse(&words).is_none());
        }
    }
    #[test]
    fn total_counts_retain_existing_owned_cross_zone_and_type_or_name_filters() {
        for text in [
            "one plus the total number of instant and sorcery cards you own in exile and in your graveyard",
            "two plus the total number of cards you own in exile and in your graveyard that are Oozes or are named Slime Against Humanity",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len(), "{text}: {value:?}");
            let Value::Add(fixed, count) = value else {
                panic!("{text}: {value:?}");
            };
            assert!(matches!(*fixed, Value::Fixed(1 | 2)));
            let Value::Count(filter) = *count else {
                panic!("{text}: {count:?}");
            };
            assert!(format!("{filter:?}").contains("Graveyard"));
            assert!(format!("{filter:?}").contains("Exile"));
            assert!(format!("{filter:?}").contains("owner: Some(You)"));
        }
    }
}

#[cfg(test)]
mod participant_tests {
    use super::*;
    #[test]
    fn turn_totals_preserve_metric_player_scope_and_contractions() {
        for (text, expected) in [
            (
                "the life you've lost this turn",
                Value::LifeLostThisTurn(PlayerFilter::You),
            ),
            (
                "the life you have lost this turn",
                Value::LifeLostThisTurn(PlayerFilter::You),
            ),
            (
                "the amount of life they lost this turn",
                Value::LifeLostThisTurn(PlayerFilter::IteratedPlayer),
            ),
            (
                "the life that player lost this turn",
                Value::LifeLostThisTurn(PlayerFilter::IteratedPlayer),
            ),
            (
                "the amount of life you gained this turn",
                Value::LifeGainedThisTurn(PlayerFilter::You),
            ),
            (
                "the damage dealt to you this turn",
                Value::DamageDealtToPlayersThisTurn(PlayerFilter::You),
            ),
            (
                "the damage already dealt to that player this turn",
                Value::DamageDealtToPlayersThisTurn(PlayerFilter::IteratedPlayer),
            ),
            (
                "the total life your opponents lost this turn",
                Value::LifeLostThisTurn(PlayerFilter::Opponent),
            ),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len(), "{text}");
            assert_eq!(value, expected, "{text}");
        }
        for text in [
            "the damage dealt by you this turn",
            "the life they lost last turn",
            "the life they paid this turn",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(
                parse(&crate::lexer::parser_token_word_refs(&tokens)).is_none(),
                "{text}"
            );
        }
    }
}

#[cfg(test)]
mod participant_draw_tests {
    use super::*;
    #[test]
    fn draws_this_turn_and_draws_this_way_keep_distinct_typed_sources() {
        for (text, this_way) in [
            ("the number of cards they've drawn this turn", false),
            ("the number of cards that player has drawn this turn", false),
            ("the number of cards they drew this way", true),
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len());
            if this_way {
                let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
                    panic!("{value:?}");
                };
                assert_eq!(query.action, Some(ironsmith_core::PriorEffectAction::Drawn));
                assert_eq!(query.player, Some(PlayerFilter::IteratedPlayer));
                assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
            } else {
                assert_eq!(
                    value,
                    Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::CardsDrawn(
                        PlayerFilter::IteratedPlayer
                    ))
                );
            }
        }
    }
}
