use super::*;

pub fn parse_effect_action_restriction_tail_words(
    words: &[&str],
) -> Option<EffectActionRestrictionTail> {
    matches!(
        words.first().copied(),
        Some(
            "put"
                | "draw"
                | "reveal"
                | "look"
                | "search"
                | "create"
                | "return"
                | "exile"
                | "sacrifice"
                | "discard"
                | "gain"
                | "lose"
        )
    )
    .then_some(EffectActionRestrictionTail)
}
