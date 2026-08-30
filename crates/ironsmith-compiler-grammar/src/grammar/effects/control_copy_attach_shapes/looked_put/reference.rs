use super::*;

pub fn parse_all_exiled_into_hand_filter(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = trim_lexed_commas(tokens);
    let (_, after_put) = primitives::parse_prefix(tokens, primitives::kw("put").void())?;
    let (_, _) = primitives::parse_prefix(
        after_put,
        alt((primitives::kw("all"), primitives::kw("each"))).void(),
    )?;
    let (into_index, _, destination) =
        primitives::find_prefix(after_put, || primitives::kw("into"))?;
    let filter = trim_lexed_commas(after_put.get(..into_index)?);
    if !primitives::contains_word(filter, "exiled")
        || !(primitives::contains_word(filter, "card")
            || primitives::contains_word(filter, "cards"))
        || !(primitives::contains_word(destination, "hand")
            || primitives::contains_word(destination, "hands"))
    {
        return None;
    }
    Some(filter)
}
