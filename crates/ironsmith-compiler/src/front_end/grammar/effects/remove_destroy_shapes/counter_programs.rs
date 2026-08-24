use super::*;

pub fn parse_destroy_counter_constraint_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestroyCounterConstraintShape<'_>> {
    let (with_idx, (), tail) = primitives::find_prefix(tokens, || primitives::kw("with").void())?;
    let base_tokens = trim_lexed_commas(&tokens[..with_idx]);
    if base_tokens.is_empty() {
        return None;
    }
    if let Some(((), constraint_tokens)) =
        primitives::parse_prefix(tail, primitives::kw("no").void())
    {
        return Some(DestroyCounterConstraintShape {
            base_tokens,
            constraint_tokens: trim_lexed_commas(constraint_tokens),
            kind: DestroyCounterConstraintKind::Without,
        });
    }
    Some(DestroyCounterConstraintShape {
        base_tokens,
        constraint_tokens: trim_lexed_commas(tail),
        kind: DestroyCounterConstraintKind::With,
    })
}
