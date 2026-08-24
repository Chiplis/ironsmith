use super::*;

pub(super) fn has_trailing_attack_or_block_restriction(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, (), after_cant)) = primitives::find_prefix(tokens, || {
        alt((
            primitives::kw("cant").void(),
            primitives::kw("cannot").void(),
            primitives::phrase(&["can", "t"]),
        ))
    }) else {
        return false;
    };
    ["attack", "attacks", "block", "blocks"]
        .iter()
        .any(|word| primitives::find_prefix(after_cant, || primitives::kw(word)).is_some())
        && primitives::has_phrase(after_cant, &["this", "turn"])
}
