use super::*;

pub fn parse_draw_equal_shape(tokens: &[OwnedLexToken]) -> Option<DrawEqualShape<'_>> {
    let tokens = trimmed(tokens);
    let ((), value_tokens) =
        primitives::parse_prefix(tokens, primitives::phrase(&["equal", "to"]).void())?;
    let value_tokens = trimmed(value_tokens);
    if exact_phrase(
        value_tokens,
        &[
            "the",
            "greatest",
            "number",
            "of",
            "cards",
            "a",
            "player",
            "discarded",
            "this",
            "way",
        ],
    ) {
        return Some(DrawEqualShape::GreatestCardsDiscardedThisWay);
    }
    for (prefix, stat) in [
        (&["power", "of"][..], DrawEqualStat::Power),
        (&["the", "power", "of"][..], DrawEqualStat::Power),
        (&["toughness", "of"][..], DrawEqualStat::Toughness),
        (&["the", "toughness", "of"][..], DrawEqualStat::Toughness),
        (&["mana", "value", "of"][..], DrawEqualStat::ManaValue),
        (
            &["the", "mana", "value", "of"][..],
            DrawEqualStat::ManaValue,
        ),
    ] {
        if let Some(((), target_tokens)) =
            primitives::parse_prefix(value_tokens, semantic_phrase(prefix).void())
        {
            let target_tokens = trimmed(target_tokens);
            if !target_tokens.is_empty() {
                return Some(DrawEqualShape::StatOfTarget {
                    stat,
                    target_tokens,
                });
            }
        }
    }
    Some(DrawEqualShape::Fallback {
        references_this_way: primitives::find_prefix(value_tokens, || {
            semantic_phrase(&["this", "way"])
        })
        .is_some(),
    })
}
