use super::*;

/// Public life-total scalars. Keep the complete player scope and the rational
/// half-starting-life threshold as typed data, independently of the consumer.
pub fn parse_life_total_quantity_words(words: &[&str]) -> Option<(Value, usize)> {
    let offset = usize::from(words.first() == Some(&"the"));
    for (phrase, player) in [
        (
            &["your", "starting", "life", "total"][..],
            PlayerFilter::You,
        ),
        (
            &["target", "players", "starting", "life", "total"][..],
            PlayerFilter::target_player(),
        ),
    ] {
        if words
            .get(offset..)
            .is_some_and(|tail| tail.starts_with(phrase))
        {
            return Some((Value::StartingLifeTotal(player), offset + phrase.len()));
        }
    }

    // All qualifying players have the same minimum, so the existing scalar
    // life-total reference does not imply a chooser or collapse tied effects.
    if words.get(offset..offset + 6)
        == Some(&["lowest", "life", "total", "among", "all", "players"][..])
    {
        return Some((Value::LifeTotal(PlayerFilter::LowestLifeTied), offset + 6));
    }
    if matches!(
        words.get(offset..offset + 4),
        Some(["highest" | "greatest", "life", "total", "among"])
    ) {
        let scope = words.get(offset + 4..)?;
        let (players, used) = if scope.starts_with(&["your", "opponents"]) {
            (PlayerFilter::Opponent, 2)
        } else if scope.starts_with(&["all", "players"]) {
            (PlayerFilter::Any, 2)
        } else if scope.starts_with(&["players"]) {
            (PlayerFilter::Any, 1)
        } else {
            return None;
        };
        return Some((Value::MaximumLifeTotal(players), offset + 4 + used));
    }
    if matches!(words.get(offset..offset + 2), Some(["number", "of"])) {
        let players = match words.get(offset + 2) {
            Some(&"opponents") => Some(PlayerFilter::Opponent),
            Some(&"players") => Some(PlayerFilter::Any),
            _ => None,
        };
        const BELOW_HALF: &[&str] = &[
            "whose", "life", "total", "is", "less", "than", "half", "their", "starting", "life",
            "total",
        ];
        if let Some(players) = players
            && words
                .get(offset + 3..)
                .is_some_and(|tail| tail.starts_with(BELOW_HALF))
        {
            return Some((
                Value::CountPlayersBelowHalfStartingLifeTotal(players),
                offset + 3 + BELOW_HALF.len(),
            ));
        }
    }
    if words
        .get(offset..)
        .is_some_and(|tail| tail.starts_with(&["last", "noted", "life", "total"]))
    {
        let used = offset + 4;
        if words.get(used) == Some(&"for") {
            let subject = words.get(used + 1..used + 3)?;
            if this_source_surface_for_words(subject).is_some() {
                return Some((Value::LastNotedLifeTotal, used + 3));
            }
            return None;
        }
        return Some((Value::LastNotedLifeTotal, used));
    }
    const LIFE_DIFFERENCE: &[&str] = &[
        "difference",
        "between",
        "your",
        "life",
        "total",
        "and",
        "target",
        "players",
        "life",
        "total",
    ];
    if words
        .get(offset..)
        .is_some_and(|tail| tail.starts_with(LIFE_DIFFERENCE))
    {
        return Some((
            Value::absolute_difference(
                Value::LifeTotal(PlayerFilter::You),
                Value::LifeTotal(PlayerFilter::target_player()),
            ),
            offset + LIFE_DIFFERENCE.len(),
        ));
    }
    None
}

#[cfg(test)]
mod minimum_life_total_tests {
    use super::*;
    #[test]
    fn all_player_minimum_does_not_guess_an_opponent_scope_or_rounding() {
        let words = "the lowest life total among all players"
            .split_whitespace()
            .collect::<Vec<_>>();
        assert_eq!(
            parse_life_total_quantity_words(&words),
            Some((Value::LifeTotal(PlayerFilter::LowestLifeTied), words.len()))
        );
        assert!(
            parse_life_total_quantity_words(&[
                "the",
                "lowest",
                "life",
                "total",
                "among",
                "your",
                "opponents"
            ])
            .is_none()
        );
    }
}
