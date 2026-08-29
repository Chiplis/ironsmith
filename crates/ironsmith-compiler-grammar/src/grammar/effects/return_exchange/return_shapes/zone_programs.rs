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
    let split = last_destination_split(tokens)?;
    let (target_tokens, random) = remove_at_random(trim_lexed_commas(&tokens[..split]));
    let destination = parse_destination(trim_lexed_commas(&tokens[split + 1..]))?;
    let target = classify_target(&target_tokens, destination.zone)?;
    Some(ReturnClauseShape {
        target,
        destination,
        destination_first,
        random,
        has_unless,
    })
}
