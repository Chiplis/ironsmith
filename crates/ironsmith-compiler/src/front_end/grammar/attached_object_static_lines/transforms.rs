use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives};
use super::subjects::{
    AttachedSubject, parse_attached_subject_lexed, semantic_finish, semantic_kw, semantic_phrase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedTransformLossKind {
    AllAbilities,
    OtherCardTypes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedTransformSpec<'a> {
    pub subject: AttachedSubject,
    pub subject_tokens: &'a [OwnedLexToken],
    pub descriptor_tokens: &'a [OwnedLexToken],
    pub ability_tokens: Option<&'a [OwnedLexToken]>,
    pub loss: Option<AttachedTransformLossKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedBasePowerToughnessSpec {
    pub power: i32,
    pub toughness: i32,
    pub preserve_other_types: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedBasePtKeywordSplit<'a> {
    pub base_tokens: &'a [OwnedLexToken],
    pub keyword_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransformBoundary {
    With,
    Loss,
    End,
}

pub fn parse_attached_transform_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedTransformSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_attached_transform_lexed,
        "attached object transform",
    )
    .ok()
}

pub fn parse_attached_base_power_toughness_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<AttachedBasePowerToughnessSpec>, CardTextError> {
    if primitives::parse_prefix(
        tokens,
        semantic_phrase(&["base", "power", "and", "toughness"]),
    )
    .is_none()
    {
        return Ok(None);
    }
    primitives::parse_all(
        tokens,
        parse_attached_base_power_toughness_lexed,
        "attached base power/toughness",
    )
    .map(Some)
}

pub fn split_attached_base_pt_keyword_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedBasePtKeywordSplit<'_>> {
    let (marker, _, after_has) = primitives::find_prefix(tokens, || {
        alt((
            (
                semantic_kw("and"),
                opt(alt((semantic_kw("it"), semantic_kw("they")))),
                alt((semantic_kw("has"), semantic_kw("have"))),
            )
                .void(),
            (
                alt((semantic_kw("it"), semantic_kw("they"))),
                alt((semantic_kw("has"), semantic_kw("have"))),
            )
                .void(),
        ))
    })?;
    let has_token = tokens.len().checked_sub(after_has.len())?.checked_sub(1)?;
    let base_tokens = trim_lexed_commas(tokens.get(..marker)?);
    let keyword_tokens = trim_lexed_commas(tokens.get(has_token + 1..)?);
    if base_tokens.is_empty() || keyword_tokens.is_empty() {
        return None;
    }
    Some(AttachedBasePtKeywordSplit {
        base_tokens,
        keyword_tokens,
    })
}

fn parse_attached_transform_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedTransformSpec<'a>> {
    let (subject, subject_tokens) = parse_attached_subject_lexed
        .with_taken()
        .parse_next(input)?;
    alt((semantic_kw("is"), semantic_kw("are"))).parse_next(input)?;
    repeat::<_, _, (), _, _>(
        0..,
        alt((semantic_kw("a"), semantic_kw("an"), semantic_kw("the"))),
    )
    .parse_next(input)?;
    let descriptor_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_transform_boundary))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    let boundary = parse_transform_boundary(input)?;
    let mut ability_tokens = None;
    let mut loss = None;
    match boundary {
        TransformBoundary::End => semantic_finish(input)?,
        TransformBoundary::Loss => {
            loss = Some(parse_transform_loss_tail(input)?);
        }
        TransformBoundary::With => {
            let granted = repeat_till::<_, _, (), _, _, _, _>(
                1..,
                any.void(),
                peek(alt((
                    parse_transform_loss_boundary.value(TransformBoundary::Loss),
                    peek(semantic_finish).value(TransformBoundary::End),
                ))),
            )
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
            let tail = alt((
                parse_transform_loss_boundary.value(TransformBoundary::Loss),
                peek(semantic_finish).value(TransformBoundary::End),
            ))
            .parse_next(input)?;
            ability_tokens = Some(trim_lexed_commas(granted));
            if tail == TransformBoundary::Loss {
                loss = Some(parse_transform_loss_tail(input)?);
            } else {
                semantic_finish(input)?;
            }
        }
    }
    Ok(AttachedTransformSpec {
        subject,
        subject_tokens,
        descriptor_tokens: trim_lexed_commas(descriptor_tokens),
        ability_tokens,
        loss,
    })
}

fn parse_transform_boundary<'a>(input: &mut LexStream<'a>) -> WResult<TransformBoundary> {
    alt((
        semantic_kw("with").value(TransformBoundary::With),
        parse_transform_loss_boundary.value(TransformBoundary::Loss),
        peek(semantic_finish).value(TransformBoundary::End),
    ))
    .parse_next(input)
}

fn parse_transform_loss_boundary<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(alt((
        (semantic_kw("and"), semantic_kw("it")).void(),
        semantic_kw("and"),
        semantic_kw("it"),
        semantic_kw("they"),
    )))
    .parse_next(input)?;
    alt((semantic_kw("lose"), semantic_kw("loses")))
        .void()
        .parse_next(input)
}

fn parse_transform_loss_tail<'a>(input: &mut LexStream<'a>) -> WResult<AttachedTransformLossKind> {
    let kind = alt((
        semantic_phrase(&["all", "other", "abilities"])
            .value(AttachedTransformLossKind::AllAbilities),
        semantic_phrase(&["all", "other", "card", "types", "and", "abilities"])
            .value(AttachedTransformLossKind::AllAbilities),
        semantic_phrase(&["all", "other", "card", "types"])
            .value(AttachedTransformLossKind::OtherCardTypes),
    ))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(kind)
}

fn parse_attached_base_power_toughness_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedBasePowerToughnessSpec> {
    semantic_phrase(&["base", "power", "and", "toughness"]).parse_next(input)?;
    let pt_token = any.parse_next(input)?;
    let (power, toughness) = leaf::parse_leaf_unsigned_pt_complete(pt_token.parser_text())
        .map_err(|_| primitives::backtrack_err("attached base power/toughness", "fixed P/T"))?;
    let preserve_other_types = alt((
        semantic_phrase(&["in", "addition", "to", "its", "other", "types"]).value(true),
        semantic_phrase(&["in", "addition", "to", "their", "other", "types"]).value(true),
        peek(semantic_finish).value(false),
    ))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(AttachedBasePowerToughnessSpec {
        power,
        toughness,
        preserve_other_types,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_attached_transform_and_base_pt() {
        let tokens = lex_line(
            "Enchanted creature is a blue Frog with base power and toughness 3/3 in addition to its other types.",
            0,
        )
        .unwrap();
        let parsed = parse_attached_transform_tokens(&tokens).unwrap();
        assert_eq!(parsed.subject, AttachedSubject::EnchantedCreature);
        let base = parse_attached_base_power_toughness_tokens(parsed.ability_tokens.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!((base.power, base.toughness), (3, 3));
        assert!(base.preserve_other_types);

        let tokens = lex_line("base power and toughness 3/3 and it has flying.", 0).unwrap();
        let split = split_attached_base_pt_keyword_tokens(&tokens).unwrap();
        assert!(
            parse_attached_base_power_toughness_tokens(split.base_tokens)
                .unwrap()
                .is_some()
        );
    }
}
