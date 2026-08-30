use super::*;

pub fn parse_target_only_shape(tokens: &[OwnedLexToken]) -> Option<TargetOnlyShape<'_>> {
    primitives::parse_prefix(tokens, primitives::kw("target"))?;
    if super::super::parse_clause_subject_verb_shape(tokens).is_some() {
        return None;
    }
    let restriction_like = [
        "blocked", "except", "unless", "attack", "attacks", "block", "blocks",
    ]
    .into_iter()
    .any(|word| primitives::find_prefix(tokens, || primitives::kw(word)).is_some());
    Some(TargetOnlyShape {
        target_tokens: tokens,
        restriction_like,
    })
}
