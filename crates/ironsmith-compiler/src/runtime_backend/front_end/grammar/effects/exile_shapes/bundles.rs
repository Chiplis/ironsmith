use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::{permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachedObjectExileShape {
    pub(crate) target: Vec<OwnedLexToken>,
    pub(crate) attachment_filter: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SameNameExileOwnerShape {
    TargetPlayer,
    TargetOpponent,
    ThatPlayer,
    You,
    TheirOrHisHer,
    FromSubject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SameNameHandGraveyardExileShape {
    pub(crate) owner: SameNameExileOwnerShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExileFaceDownSuffixShape<'a> {
    pub(crate) core: &'a [OwnedLexToken],
    pub(crate) face_down: bool,
}

pub(crate) fn parse_attached_object_exile_shape(
    tokens: &[OwnedLexToken],
) -> Option<AttachedObjectExileShape> {
    let (target, attached) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("and").void())?;
    let attached = primitives::strip_lexed_prefix_phrase(attached, &["all"])?;
    let (attachment_filter, attachment_target) =
        primitives::split_lexed_once_on_separator(attached, || {
            primitives::phrase(&["attached", "to"]).void()
        })?;
    if target.is_empty()
        || attachment_filter.is_empty()
        || !(permission_shapes::exact_tokens(attachment_target, &["it"])
            || permission_shapes::exact_tokens(attachment_target, &["them"]))
    {
        return None;
    }
    Some(AttachedObjectExileShape {
        target: target.to_vec(),
        attachment_filter: attachment_filter.to_vec(),
    })
}

fn same_name_target_owner(input: &mut LexStream<'_>) -> WResult<SameNameExileOwnerShape> {
    alt((
        primitives::phrase(&["target", "opponent's"])
            .value(SameNameExileOwnerShape::TargetOpponent),
        primitives::phrase(&["target", "opponents"]).value(SameNameExileOwnerShape::TargetOpponent),
        primitives::phrase(&["target", "opponent"]).value(SameNameExileOwnerShape::TargetOpponent),
        primitives::phrase(&["target", "player's"]).value(SameNameExileOwnerShape::TargetPlayer),
        primitives::phrase(&["target", "players"]).value(SameNameExileOwnerShape::TargetPlayer),
        primitives::phrase(&["target", "player"]).value(SameNameExileOwnerShape::TargetPlayer),
    ))
    .parse_next(input)
}

fn same_name_other_owner(input: &mut LexStream<'_>) -> WResult<SameNameExileOwnerShape> {
    alt((
        primitives::phrase(&["that", "player's"]).value(SameNameExileOwnerShape::ThatPlayer),
        primitives::phrase(&["that", "players"]).value(SameNameExileOwnerShape::ThatPlayer),
        primitives::phrase(&["that", "player"]).value(SameNameExileOwnerShape::ThatPlayer),
        primitives::phrase(&["his", "or", "her"]).value(SameNameExileOwnerShape::TheirOrHisHer),
        primitives::kw("their").value(SameNameExileOwnerShape::TheirOrHisHer),
        primitives::kw("your").value(SameNameExileOwnerShape::You),
    ))
    .parse_next(input)
}

fn same_name_owner(input: &mut LexStream<'_>) -> WResult<SameNameExileOwnerShape> {
    alt((same_name_target_owner, same_name_other_owner)).parse_next(input)
}

fn hand_or_graveyard(input: &mut LexStream<'_>) -> WResult<Zone> {
    alt((
        primitives::kw("hand").value(Zone::Hand),
        primitives::kw("hands").value(Zone::Hand),
        primitives::kw("graveyard").value(Zone::Graveyard),
        primitives::kw("graveyards").value(Zone::Graveyard),
    ))
    .parse_next(input)
}

pub(crate) fn parse_same_name_hand_graveyard_exile_shape(
    tokens: &[OwnedLexToken],
) -> Option<SameNameHandGraveyardExileShape> {
    primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["all", "cards"]),
            primitives::phrase(&["all", "card"]),
        )),
    )?;
    if !permission_shapes::contains_tokens(tokens, &["with", "that", "name"]) {
        return None;
    }
    let (_, after_from) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("from").void())?;
    let ((owner, first_zone), remainder) =
        primitives::parse_prefix(after_from, (opt(same_name_owner), hand_or_graveyard))?;
    let has_hand = first_zone == Zone::Hand
        || permission_shapes::contains_tokens(remainder, &["hand"])
        || permission_shapes::contains_tokens(remainder, &["hands"]);
    let has_graveyard = first_zone == Zone::Graveyard
        || permission_shapes::contains_tokens(remainder, &["graveyard"])
        || permission_shapes::contains_tokens(remainder, &["graveyards"]);
    if !has_hand || !has_graveyard {
        return None;
    }
    Some(SameNameHandGraveyardExileShape {
        owner: owner.unwrap_or(SameNameExileOwnerShape::FromSubject),
    })
}

pub(crate) fn parse_exile_face_down_suffix_shape(
    tokens: &[OwnedLexToken],
) -> ExileFaceDownSuffixShape<'_> {
    let before_instead =
        primitives::strip_lexed_suffix_phrase(tokens, &["instead"]).unwrap_or(tokens);
    for suffix in [&["face", "down"][..], &["face-down"][..], &["facedown"][..]] {
        if let Some(core) = primitives::strip_lexed_suffix_phrase(before_instead, suffix) {
            return ExileFaceDownSuffixShape {
                core,
                face_down: true,
            };
        }
    }
    ExileFaceDownSuffixShape {
        core: tokens,
        face_down: false,
    }
}

pub(crate) fn strip_exile_all_or_each_shape(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, alt((primitives::kw("all"), primitives::kw("each"))))
        .map(|(_, rest)| rest)
}

pub(crate) fn starts_exile_multi_target_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::kw("target")).is_some()
        || (primitives::parse_prefix(tokens, primitives::phrase(&["up", "to"])).is_some()
            && permission_shapes::contains_tokens(tokens, &["target"]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_attached_same_name_and_face_down_shapes() {
        let attached =
            parse_attached_object_exile_shape(&lex("target creature and all Auras attached to it"))
                .unwrap();
        assert!(permission_shapes::exact_tokens(
            &attached.attachment_filter,
            &["auras"]
        ));

        let plural = parse_attached_object_exile_shape(&lex(
            "any number of target creatures and all Auras attached to them",
        ))
        .unwrap();
        assert!(permission_shapes::exact_tokens(
            &plural.attachment_filter,
            &["auras"]
        ));

        let same_name = parse_same_name_hand_graveyard_exile_shape(&lex(
            "all cards with that name from target player's hand and graveyard",
        ))
        .unwrap();
        assert_eq!(same_name.owner, SameNameExileOwnerShape::TargetPlayer);

        let tokens = lex("target creature face down, instead,");
        let suffix = parse_exile_face_down_suffix_shape(&tokens);
        assert!(suffix.face_down);
        assert!(permission_shapes::exact_tokens(
            suffix.core,
            &["target", "creature"]
        ));
    }
}
