use super::*;

pub fn target_names_unowned_shared_zone(tokens: &[OwnedLexToken]) -> bool {
    [
        &["from", "a", "graveyard"][..],
        &["from", "any", "graveyard"][..],
        &["from", "a", "library"][..],
        &["from", "any", "library"][..],
    ]
    .iter()
    .any(|phrase| permission_shapes::contains_tokens(tokens, phrase))
}
