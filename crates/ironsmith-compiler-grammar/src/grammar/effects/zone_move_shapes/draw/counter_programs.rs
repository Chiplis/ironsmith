use super::*;

pub fn counter_same_name_graveyard_shape(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trimmed(tokens);
    let Some((_, (), after_graveyard)) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("graveyard"), primitives::kw("graveyards"))).void()
    }) else {
        return false;
    };
    primitives::find_prefix(after_graveyard, || {
        semantic_phrase(&["same", "name", "as", "the", "spell"])
    })
    .is_some()
        || primitives::find_prefix(after_graveyard, || {
            semantic_phrase(&["same", "name", "as", "that", "spell"])
        })
        .is_some()
}
