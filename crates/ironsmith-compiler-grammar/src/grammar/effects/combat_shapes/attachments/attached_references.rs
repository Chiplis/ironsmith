use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedObjectReferenceTag {
    Enchanted,
    Equipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedObjectReferenceKind {
    Equipment,
    Artifact,
    Creature,
    Enchantment,
    Land,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedObjectReferenceShape {
    pub tag: AttachedObjectReferenceTag,
    pub kind: AttachedObjectReferenceKind,
}

fn attached_object_reference<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedObjectReferenceShape> {
    let tag = alt((
        primitives::kw("enchanted").value(AttachedObjectReferenceTag::Enchanted),
        primitives::kw("equipped").value(AttachedObjectReferenceTag::Equipped),
    ))
    .parse_next(input)?;
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    )))
    .parse_next(input)?;
    let kind = alt((
        alt((
            primitives::kw("equipment").value(AttachedObjectReferenceKind::Equipment),
            primitives::kw("equipments").value(AttachedObjectReferenceKind::Equipment),
            primitives::kw("artifact").value(AttachedObjectReferenceKind::Artifact),
            primitives::kw("artifacts").value(AttachedObjectReferenceKind::Artifact),
            primitives::kw("creature").value(AttachedObjectReferenceKind::Creature),
            primitives::kw("creatures").value(AttachedObjectReferenceKind::Creature),
        )),
        alt((
            primitives::kw("enchantment").value(AttachedObjectReferenceKind::Enchantment),
            primitives::kw("enchantments").value(AttachedObjectReferenceKind::Enchantment),
            primitives::kw("land").value(AttachedObjectReferenceKind::Land),
            primitives::kw("lands").value(AttachedObjectReferenceKind::Land),
            primitives::kw("permanent").value(AttachedObjectReferenceKind::Permanent),
            primitives::kw("permanents").value(AttachedObjectReferenceKind::Permanent),
        )),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AttachedObjectReferenceShape { tag, kind })
}

pub fn parse_attached_object_reference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedObjectReferenceShape> {
    crate::grammar::primitives::probe_all(
        tokens,
        attached_object_reference,
        "enchanted or equipped object reference",
    )
}
