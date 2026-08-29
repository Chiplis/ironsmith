use super::*;

pub(super) fn parse_optional_object_count(input: &mut LexStream<'_>) -> u32 {
    let mut number = input.clone();
    if let Ok(count) = leaf::parse_leaf_number_prefix_lexed.parse_next(&mut number) {
        *input = number;
        return count;
    }
    let mut article = input.clone();
    if alt((primitives::kw("a"), primitives::kw("an")))
        .parse_next(&mut article)
        .is_ok()
    {
        *input = article;
    }
    1
}
