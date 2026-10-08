use super::*;

pub fn parse_return_clause_shape(tokens: &[OwnedLexToken]) -> Option<ReturnClauseShape> {
    let destination_first = primitives::parse_prefix(tokens, primitives::kw("to")).is_some();
    let normalized;
    let tokens = if destination_first {
        normalized = normalize_destination_first(tokens)?;
        normalized.as_slice()
    } else {
        tokens
    };
    let has_unless = marker_anywhere(tokens, primitives::kw("unless"));
    let (split, destination_start) = last_destination_split(tokens)?;
    // "... to its owner's hand, then repeats this process for an artifact":
    // the destination does not absorb a repeated-process tail. Reading only
    // the zone would silently drop the repeated returns.
    if tokens[destination_start..].windows(2).any(|pair| {
        token_is(&pair[0], "then")
            && (token_is(&pair[1], "repeat") || token_is(&pair[1], "repeats"))
    }) {
        return None;
    }
    let (target_tokens, random) = remove_at_random(trim_lexed_commas(&tokens[..split]));
    let destination = parse_destination(trim_lexed_commas(&tokens[destination_start..]))?;
    let target = classify_target(&target_tokens, destination.zone)?;
    Some(ReturnClauseShape {
        target,
        destination,
        destination_first,
        random,
        has_unless,
    })
}
