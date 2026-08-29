use super::*;

pub(super) fn find_cant_gain_tail(
    tokens: &[OwnedLexToken],
) -> Option<(usize, usize, ironsmith_core::AbilityLossMode)> {
    const CANT_HAVE_OR_GAIN_PHRASES: &[&[&str]] = &[
        &["cant", "have", "or", "gain"],
        &["can't", "have", "or", "gain"],
        &["cannot", "have", "or", "gain"],
        &["can", "t", "have", "or", "gain"],
    ];
    const CANT_GAIN_PHRASES: &[&[&str]] = &[
        &["cant", "gain"],
        &["can't", "gain"],
        &["cannot", "gain"],
        &["can", "t", "gain"],
    ];
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    while let Ok(token) = take_token(&mut input) {
        let and_token = initial_len.saturating_sub(input.len() + 1);
        if !token.is_word("and") {
            continue;
        }
        let after_and = &tokens[and_token + 1..];
        let (rest, loss_mode) = if let Some((_, rest)) =
            primitives::parse_prefix(after_and, primitives::any_phrase(CANT_HAVE_OR_GAIN_PHRASES))
        {
            (rest, ironsmith_core::AbilityLossMode::LoseAndCantHaveOrGain)
        } else if let Some((_, rest)) =
            primitives::parse_prefix(after_and, primitives::any_phrase(CANT_GAIN_PHRASES))
        {
            (rest, ironsmith_core::AbilityLossMode::LoseAndCantGain)
        } else {
            continue;
        };
        let consumed = after_and.len().saturating_sub(rest.len());
        return Some((and_token, and_token + 1 + consumed, loss_mode));
    }
    None
}
