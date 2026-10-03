use super::*;

pub(super) fn parse(words: &[&str]) -> Option<(Value, usize)> {
    let mut start = usize::from(words.first() == Some(&"the"));
    if words.get(start) == Some(&"total") {
        start += 1;
    }
    if words.get(start..start + 2) == Some(&["amount", "of"][..]) {
        start += 2;
    }
    if words.get(start..start + 6)
        == Some(&["life", "your", "opponents", "lost", "this", "turn"][..])
    {
        return Some((Value::LifeLostThisTurn(PlayerFilter::Opponent), start + 6));
    }
    if words.get(start) != Some(&"damage") {
        return None;
    }
    start += 1;
    if words.get(start) == Some(&"already") {
        start += 1;
    }
    if words.get(start..start + 6)
        == Some(&["dealt", "to", "your", "opponents", "this", "turn"][..])
    {
        Some((
            Value::DamageDealtToPlayersThisTurn(PlayerFilter::Opponent),
            start + 6,
        ))
    } else {
        None
    }
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
            vec!["the", "life", "your", "opponents", "gained", "this", "turn"],
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
