//! Independent entry instruction; no manifest or cloak special-action grant.
use super::*;
use winnow::combinator::{alt, eof, opt, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

pub(crate) struct Entry<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub owner: bool,
    pub optional: bool,
    pub permission: Option<ironsmith_core::DescribedFaceDownCastPermission>,
}

fn shape<'a>(input: &mut crate::lexer::LexStream<'a>) -> winnow::error::ModalResult<(&'a [OwnedLexToken], bool, bool, Option<(&'a [OwnedLexToken], bool)>)> {
    let optional = opt(grammar::phrase(&["you", "may"])).parse_next(input)?.is_some();
    grammar::kw("put").parse_next(input)?;
    let target = repeat_till(1.., any.void(), crate::grammar::primitives::peek(
        (alt((grammar::kw("onto"), grammar::kw("on"))), grammar::phrase(&["the", "battlefield"]))
    )).map(|((), _)| ()).take().parse_next(input)?;
    alt((grammar::kw("onto"), grammar::kw("on"))).parse_next(input)?;
    grammar::phrase(&["the", "battlefield", "face", "down"]).parse_next(input)?;
    let owner = opt(grammar::phrase(&["under", "its", "owner's", "control"])).parse_next(input)?.is_some();
    grammar::phrase(&["as", "a"]).parse_next(input)?;
    grammar::kw("2/2").parse_next(input)?;
    grammar::kw("creature").parse_next(input)?;
    opt(grammar::period()).parse_next(input)?;
    let permission = opt(cast_permission_shape).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok((target, owner, optional, permission))
}

pub(crate) fn read(tokens: &[OwnedLexToken]) -> Result<Option<Entry<'_>>, CardTextError> {
    let Some((target_tokens, owner, optional, permission)) = crate::grammar::primitives::probe_all(tokens, shape, "described-face-down-entry") else { return Ok(None); };
    let permission = permission.map(|(filter, require_printed_mana_cost)| {
        crate::object_filters::parse_object_filter(filter, false).map(|filter| ironsmith_core::DescribedFaceDownCastPermission { filter, require_printed_mana_cost })
    }).transpose()?;
    Ok(Some(Entry { target_tokens, owner, optional, permission }))
}

pub(crate) fn build_entry(target: TargetAst, owner: bool, permission: Option<ironsmith_core::DescribedFaceDownCastPermission>) -> EffectAst {
    let mut effect = EffectAst::subject_verb_manifest_onto_battlefield(
        PlayerAst::You, target, false,
        if owner { ReturnControllerAst::Owner } else { ReturnControllerAst::You }, false,
    );
    if let EffectAst::SubjectVerb(subject) = &mut effect {
        if let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield { described_face_down, no_turn_up_permission, exile_cast_permission, .. }) = &mut subject.action {
            *described_face_down = true;
            *no_turn_up_permission = true;
            *exile_cast_permission = permission;
        }
    }
    effect
}

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(entry) = read(tokens)? else { return Ok(None); };
    let target = parse_target_phrase(entry.target_tokens)?;
    let effects = vec![build_entry(target, entry.owner, entry.permission)];
    Ok(Some(if entry.optional {
        vec![EffectAst::Permissions(PermissionEffectAst::May { effects })]
    } else { effects }))
}

// This is an optional action permission, independent of the preceding entry.
fn cast_permission_shape<'a>(input: &mut crate::lexer::LexStream<'a>) -> winnow::error::ModalResult<(&'a [OwnedLexToken], bool)> {
    grammar::phrase(&["you", "may", "exile", "and", "cast", "it", "face", "up", "any", "time", "you", "have", "priority", "if", "it", "is"]).parse_next(input)?;
    opt(grammar::kw("a")).parse_next(input)?;
    let filter = repeat_till(1.., any.void(), crate::grammar::primitives::peek(alt((grammar::kw("with").void(), grammar::period().void()))))
        .map(|((), _)| ()).take().parse_next(input)?;
    let require_cost = opt(grammar::phrase(&["with", "a", "mana", "cost"])).parse_next(input)?.is_some();
    grammar::period().parse_next(input)?;
    grammar::phrase(&["you", "still", "pay", "its", "costs"]).parse_next(input)?;
    opt(grammar::period()).parse_next(input)?;
    Ok((filter, require_cost))
}
