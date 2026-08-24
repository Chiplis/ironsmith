use super::*;

pub fn parse_repeated_tagged_mana_payment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RepeatedTaggedManaPayment> {
    let repeats =
        primitives::find_prefix(tokens, || primitives::phrase(&["for", "each"])).is_some();
    let references_tagged_choice = primitives::find_prefix(tokens, || {
        alt((primitives::kw("those"), primitives::kw("them"))).void()
    })
    .is_some()
        || primitives::find_prefix(tokens, || primitives::phrase(&["chosen", "this", "way"]))
            .is_some();
    if !repeats || !references_tagged_choice {
        return None;
    }

    let mut stream = LexStream::new(tokens);
    let pip_groups = primitives::collect_mana_pip_groups
        .parse_next(&mut stream)
        .ok()?;
    Some(RepeatedTaggedManaPayment { pip_groups })
}
