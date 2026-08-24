use super::*;

pub fn starts_with_all_or_each(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        trim_lexed_commas(tokens),
        alt((primitives::kw("all"), primitives::kw("each"))).void(),
    )
    .is_some()
}

pub fn contains_from_it(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::contains_tokens(tokens, &["from", "it"])
}

pub fn contains_among_them(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "among") && primitives::contains_word(tokens, "them")
}

pub fn contains_permanent(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "permanent")
}

pub fn contains_sticker(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "sticker")
}
