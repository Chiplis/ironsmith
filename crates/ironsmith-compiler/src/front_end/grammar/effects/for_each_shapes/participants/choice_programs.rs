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
    let (_, _, after_unless) =
        primitives::find_prefix(after_return, || primitives::kw("unless").void())?;
    primitives::parse_prefix(
        after_unless,
        primitives::phrase(&["its", "controller", "has", "you", "draw", "a", "card"]),
    )?;
    Some(target_tokens)
}
