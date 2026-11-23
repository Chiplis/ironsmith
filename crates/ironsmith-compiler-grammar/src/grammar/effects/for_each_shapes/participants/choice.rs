use super::*;

pub(super) fn choose_return_unless(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_choose) = primitives::parse_prefix(trim(tokens), primitives::kw("choose"))?;
    let (return_index, _, after_return) = primitives::find_prefix(after_choose, || {
        primitives::phrase(&["then", "return"]).void()
    })?;
    let target_tokens = trim(after_choose.get(..return_index)?);
    if target_tokens.is_empty() {
        return None;
    }
    let (unless_index, _, after_unless) =
        primitives::find_prefix(after_return, || primitives::kw("unless").void())?;
    primitives::parse_prefix(
        after_unless,
        primitives::phrase(&["its", "controller", "has", "you", "draw", "a", "card"]),
    )?;
    // The returned object is the chosen one, back to its owner's hand.
    let returned = trim(after_return.get(..unless_index)?);
    if !matches!(
        crate::lexer::parser_token_word_refs(returned).as_slice(),
        ["that" | "the", "creature" | "permanent", "to", "its", "owners" | "owner's", "hand"]
    ) {
        return None;
    }
    crate::word_claims::claim_tokens(returned);
    Some(target_tokens)
}
