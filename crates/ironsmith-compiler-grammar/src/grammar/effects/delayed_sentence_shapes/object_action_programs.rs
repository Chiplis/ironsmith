use super::*;

pub fn parse_copy_twice_shape(tokens: &[OwnedLexToken]) -> Option<CopyTwiceShape> {
    let (_, tail) = primitives::parse_prefix(
        trimmed(tokens),
        semantic_phrase(&["copy", "that", "spell", "or", "ability", "twice"]),
    )?;
    let tail = trimmed(tail);
    if tail.is_empty()
        || primitives::parse_all(
            tail,
            (repeat::<_, _, (), _, _>(0.., semantic_noise), eof).void(),
            "copy twice punctuation tail",
        )
        .is_ok()
    {
        return Some(CopyTwiceShape {
            may_choose_new_targets: false,
        });
    }
    primitives::parse_all(
        tail,
        (
            semantic_phrase(&[
                "you", "may", "choose", "new", "targets", "for", "the", "copies",
            ]),
            repeat::<_, _, (), _, _>(0.., semantic_noise),
            eof,
        )
            .void(),
        "copy twice target tail",
    )
    .ok()
    .map(|()| CopyTwiceShape {
        may_choose_new_targets: true,
    })
}
