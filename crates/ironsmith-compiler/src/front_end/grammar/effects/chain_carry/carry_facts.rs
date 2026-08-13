use winnow::combinator::{alt, opt, repeat};
use winnow::prelude::*;

use crate::effect::Until;

use super::{semantic_finish, semantic_kw};
use crate::grammar::{filters, leaf, primitives};
use crate::lexer::{OwnedLexToken, TokenWordView, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestActionShape {
    Destroy,
    Exile,
    Sacrifice,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CarryDurationPrefix<'a> {
    pub(crate) duration: Until,
    pub(crate) rest: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryableSubjectShape {
    Source,
    ExplicitTarget,
    ObjectFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryClauseHead {
    Choose,
    Create,
    Draw,
    Scry,
    Surveil,
    Other,
}

pub(crate) fn parse_rest_action_tokens(tokens: &[OwnedLexToken]) -> Option<RestActionShape> {
    primitives::parse_all(
        tokens,
        (
            opt(semantic_kw("then")),
            alt((
                semantic_kw("destroy").value(RestActionShape::Destroy),
                semantic_kw("exile").value(RestActionShape::Exile),
                alt((semantic_kw("sacrifice"), semantic_kw("sacrifices")))
                    .value(RestActionShape::Sacrifice),
            )),
            semantic_kw("rest"),
            semantic_finish,
        )
            .map(|(_, action, _, _)| action),
        "rest-action chain segment",
    )
    .ok()
}

pub(crate) fn parse_carry_duration_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CarryDurationPrefix<'_>> {
    let (duration, rest) = if let Some(parsed) =
        leaf::parse_leaf_restriction_duration_prefix_tokens(tokens)
    {
        let duration = match parsed.duration {
            leaf::LeafDurationPhrase::UntilEndOfTurn => Until::EndOfTurn,
            leaf::LeafDurationPhrase::UntilYourNextTurn => Until::YourNextTurn,
            leaf::LeafDurationPhrase::UntilYourNextUpkeep => Until::YourNextUpkeep,
            leaf::LeafDurationPhrase::ControllersNextUntapStep => Until::ControllersNextUntapStep,
            _ => return None,
        };
        (duration, parsed.rest)
    } else {
        let parsed = leaf::parse_leaf_conditional_duration_prefix_tokens(tokens)?;
        let duration = match parsed.duration {
            leaf::LeafConditionalDurationKind::YouControlSource => Until::YouStopControllingThis,
            leaf::LeafConditionalDurationKind::SourceRemainsTapped => Until::SourceUntaps,
            leaf::LeafConditionalDurationKind::SourceRemainsOnBattlefield => {
                Until::ThisLeavesTheBattlefield
            }
        };
        (duration, parsed.rest)
    };
    let rest = trim_lexed_commas(rest);
    if super::super::clause_primitive_shapes::parse_trigger_clause_intro_shape(rest).is_some() {
        return None;
    }
    Some(CarryDurationPrefix { duration, rest })
}

pub(crate) fn parse_carryable_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CarryableSubjectShape> {
    if crate::util::is_source_reference_words(
        &TokenWordView::new(tokens).word_refs(),
    ) {
        return Some(CarryableSubjectShape::Source);
    }
    if leaf::parse_leaf_target_head_tokens(tokens)
        .ok()
        .is_some_and(|head| head.prefix.explicit_target_span.is_some())
    {
        return Some(CarryableSubjectShape::ExplicitTarget);
    }
    filters::parse_object_filter_with_grammar_entrypoint_lexed(tokens, false)
        .ok()
        .map(|_| CarryableSubjectShape::ObjectFilter)
}

pub(crate) fn parse_carry_clause_head_tokens(tokens: &[OwnedLexToken]) -> CarryClauseHead {
    primitives::parse_prefix(
        tokens,
        (
            repeat::<_, _, (), _, _>(0.., alt((semantic_kw("then"), semantic_kw("and")))),
            alt((
                semantic_kw("choose").value(CarryClauseHead::Choose),
                semantic_kw("create").value(CarryClauseHead::Create),
                semantic_kw("draw").value(CarryClauseHead::Draw),
                semantic_kw("scry").value(CarryClauseHead::Scry),
                semantic_kw("surveil").value(CarryClauseHead::Surveil),
            )),
        )
            .map(|(_, head)| head),
    )
    .map_or(CarryClauseHead::Other, |(head, _)| head)
}
