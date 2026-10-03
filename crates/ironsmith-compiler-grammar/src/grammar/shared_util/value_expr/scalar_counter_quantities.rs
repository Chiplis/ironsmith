use super::*;

pub(super) fn parse(words: &[&str]) -> Option<(Value, usize)> {
    let offset = usize::from(words.first() == Some(&"the"));
    let rest = &words[offset..];
    if rest.starts_with(&["amount", "of", "e", "you", "have"])
        || rest.starts_with(&["amount", "of", "energy", "you", "have"])
    {
        return Some((
            Value::PlayerCounters(PlayerFilter::You, crate::object::CounterType::Energy),
            offset + 5,
        ));
    }
    let count_offset = if rest.starts_with(&["total", "number", "of"]) {
        3
    } else if rest.starts_with(&["number", "of"]) {
        2
    } else {
        return None;
    };
    let counter = rest[count_offset..]
        .iter()
        .position(|word| matches!(*word, "counter" | "counters"))?
        + count_offset;
    if !(1..=2).contains(&(counter - count_offset)) {
        return None;
    }
    let kind = parse_counter_type_words(&rest[count_offset..=counter])?;
    let tail = &rest[counter + 1..];
    let (player, used) = if tail.starts_with(&["among", "players"]) {
        (PlayerFilter::Any, 2)
    } else if tail.starts_with(&["among", "all", "players"]) {
        (PlayerFilter::Any, 3)
    } else if tail.starts_with(&["among", "your", "opponents"]) {
        (PlayerFilter::Opponent, 3)
    } else {
        return None;
    };
    Some((
        Value::PlayerCounters(player, kind),
        offset + counter + 1 + used,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn energy_symbols_and_player_counter_totals_keep_their_domains() {
        for text in [
            "the amount of {E} you have",
            "the amount of energy you have",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len());
            assert_eq!(
                value,
                Value::PlayerCounters(PlayerFilter::You, crate::object::CounterType::Energy)
            );
        }
        let tokens =
            crate::lexer::lex_line("the total number of rad counters among players", 0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        assert_eq!(
            value,
            Value::PlayerCounters(PlayerFilter::Any, crate::object::CounterType::Rad)
        );
        assert!(
            parse(&[
                "the",
                "number",
                "of",
                "rad",
                "counters",
                "among",
                "creatures"
            ])
            .is_none()
        );
    }
    #[test]
    fn integer_scalars_compose_source_characteristics_and_keep_x_legacy_form() {
        let tokens = crate::lexer::lex_line("three times this creature's power", 0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        assert!(
            matches!(value, Value::Scaled(value, 3) if matches!(value.unhinted(), Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Source)))
        );
        assert_eq!(
            parse_value_expr_words(&["five", "times", "x"]),
            Some((Value::XTimes(5), 3))
        );
        let tokens = crate::lexer::lex_line("this planeswalker's loyalty", 0).unwrap();
        let (value, used) = parse_value_expr_tokens(&tokens).unwrap();
        assert_eq!(used, tokens.len());
        assert!(
            matches!(value, Value::CountersOn(spec, Some(crate::object::CounterType::Loyalty)) if matches!(spec.base(), ChooseSpec::Source))
        );
    }
}
