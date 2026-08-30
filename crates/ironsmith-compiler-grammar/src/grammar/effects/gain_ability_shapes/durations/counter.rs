use super::*;

/// Parse an authored leading duration such as
/// `for as long as that permanent has a charge counter on it, ...`.
///
/// The surrounding gain-ability parser uses the returned word count to start
/// verb recognition after the condition, so the condition's own `has` cannot
/// be mistaken for the grant verb.
pub fn parse_leading_affected_object_counter_duration_shape(
    tokens: &[OwnedLexToken],
) -> Option<LeadingGainDurationShape> {
    let (duration, rest) =
        primitives::parse_prefix(tokens, affected_object_counter_duration_lexed)?;
    if rest.is_empty() {
        return None;
    }
    let consumed_tokens = tokens.len().checked_sub(rest.len())?;
    Some(LeadingGainDurationShape {
        consumed_words: TokenWordView::new(&tokens[..consumed_tokens]).len(),
        duration,
    })
}
