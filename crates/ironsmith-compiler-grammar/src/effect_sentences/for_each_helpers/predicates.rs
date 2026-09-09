use super::*;

pub fn parse_who_did_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    Ok(tagged_past_action_predicate(
        for_each_shapes::parse_who_tagged_filter_shape(inner_tokens),
        inner_tokens,
    ))
}
