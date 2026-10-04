//! Exact recipient-limited damage amounts. This is a prior instruction metric,
//! not a triggering event amount and not the attempted X amount.
use super::*;
pub(super) fn parse(words: &[&str]) -> Option<(Value, usize)> {
    let prefix = ["the", "damage", "dealt", "but", "not", "more"];
    let mut tail = words.strip_prefix(&prefix)?;
    let mut used = prefix.len();
    if tail.first() == Some(&"life") {
        tail = &tail[1..];
        used += 1;
    }
    tail = tail.strip_prefix(&["than"])?;
    used += 1;
    let paid_color = if let ["the", "amount", "of", symbol, "spent", "on", "x", rest @ ..] = tail {
        let color = Color::from_mana_code_or_name(symbol)?;
        tail = rest;
        used += 7;
        Some(color)
    } else {
        None
    };
    let [
        "the",
        "players" | "player's",
        "life",
        "total",
        "before",
        "the",
        "damage",
        "was",
        "dealt",
        "the",
        "planeswalkers" | "planeswalker's",
        "loyalty",
        "before",
        "the",
        "damage",
        "was",
        "dealt",
        "or",
        "the",
        "creatures" | "creature's",
        "toughness",
        rest @ ..,
    ] = tail
    else {
        return None;
    };
    used += tail.len() - rest.len();
    let amount = Value::PendingPriorEffectMetric(
        ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::Outcome,
            ironsmith_core::EffectMetric::DamageDealtCappedByRecipient,
        )
        .with_action(ironsmith_core::PriorEffectAction::DealtDamage),
    );
    Some((
        if let Some(color) = paid_color {
            Value::Min(Box::new(amount), Box::new(Value::ManaSpentOnX(color)))
        } else {
            amount
        },
        used,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capped_damage_amount_retains_exact_prior_instruction_and_actual_color() {
        for extra in ["", "the amount of {B} spent on X, "] {
            let text = format!(
                "the damage dealt, but not more than {extra}the player's life total before the damage was dealt, the planeswalker's loyalty before the damage was dealt, or the creature's toughness"
            );
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let (value, used) = super::super::parse_value_expr_tokens(&tokens).unwrap();
            assert_eq!(used, tokens.len());
            let debug = format!("{value:?}");
            assert!(debug.contains("DamageDealtCappedByRecipient"));
            assert_eq!(debug.contains("ManaSpentOnX"), !extra.is_empty());
        }
    }
}
