use super::*;

pub(super) fn parse_conditional_destroy_shape(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (if_idx, (), predicate_tokens) =
        primitives::find_prefix(tokens, || primitives::kw("if").void())?;
    let mut target_tokens = trim_lexed_commas(&tokens[..if_idx]);
    while let Some((head, ())) =
        primitives::split_lexed_once_before_suffix(target_tokens, 0, || {
            primitives::kw("instead").void()
        })
    {
        target_tokens = trim_lexed_commas(head);
    }
    Some((target_tokens, trim_lexed_commas(predicate_tokens)))
}
