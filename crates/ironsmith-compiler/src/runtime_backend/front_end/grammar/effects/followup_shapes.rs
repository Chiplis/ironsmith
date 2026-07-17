use super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[path = "followup_shapes/regeneration.rs"]
mod regeneration;
pub(crate) use regeneration::*;
#[path = "followup_shapes/player_and_library.rs"]
mod player_and_library;
pub(crate) use player_and_library::*;
#[path = "followup_shapes/counter_linked_land.rs"]
mod counter_linked_land;
pub(crate) use counter_linked_land::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreateMorePriorTokensShape<'a> {
    pub(crate) predicate_tokens: &'a [OwnedLexToken],
    pub(crate) count: u32,
    pub(crate) instead: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionalFollowupKind {
    WhenMilledThisWay,
    IfNoOneDoes,
    IfYouWin,
    IfYouWinClash,
    IfYouWinFlip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConditionalFollowupShape<'a> {
    pub(crate) kind: ConditionalFollowupKind,
    pub(crate) continuation_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TokenReminderFollowupFacts {
    pub(crate) lifecycle_head: bool,
    pub(crate) delayed_pronoun_lifecycle: bool,
    pub(crate) pronoun_trigger_prefix: bool,
}

fn marker_anywhere<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

#[path = "followup_shapes/turn_skip.rs"]
mod turn_skip;
pub(crate) use turn_skip::*;

pub(crate) fn is_temporary_land_animation_sentence(tokens: &[OwnedLexToken]) -> bool {
    marker_anywhere(
        tokens,
        alt((primitives::kw("become"), primitives::kw("becomes"))),
    ) && marker_anywhere(
        tokens,
        alt((primitives::kw("creature"), primitives::kw("creatures"))),
    ) && marker_anywhere(tokens, primitives::phrase(&["until", "end", "of", "turn"]))
}

fn create_or_put<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("create"), primitives::kw("put")))
        .void()
        .parse_next(input)
}

fn create_more_tail<'a>(input: &mut LexStream<'a>) -> WResult<bool> {
    alt((
        (primitives::kw("instead"), primitives::sentence_end()).value(true),
        (
            primitives::phrase(&["onto", "the", "battlefield"]),
            opt(primitives::kw("instead")),
            primitives::sentence_end(),
        )
            .map(|(_, instead, _)| instead.is_some()),
        primitives::sentence_end().value(false),
    ))
    .parse_next(input)
}

fn parse_create_more_prior_tokens_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CreateMorePriorTokensShape<'a>> {
    let predicate_tokens = repeat_till(1.., any.void(), peek(create_or_put))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    create_or_put.parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["of", "those", "tokens"]).parse_next(input)?;
    let instead = create_more_tail.parse_next(input)?;
    Ok(CreateMorePriorTokensShape {
        predicate_tokens,
        count,
        instead,
    })
}

pub(crate) fn parse_create_more_prior_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CreateMorePriorTokensShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_create_more_prior_tokens_lexed,
        "create more prior tokens",
    )
    .ok()
}

fn conditional_followup_prefix<'a>(input: &mut LexStream<'a>) -> WResult<ConditionalFollowupKind> {
    alt((
        primitives::phrase(&[
            "when", "one", "or", "more", "cards", "are", "milled", "this", "way",
        ])
        .value(ConditionalFollowupKind::WhenMilledThisWay),
        primitives::phrase(&["if", "no", "one", "does"])
            .value(ConditionalFollowupKind::IfNoOneDoes),
        (
            primitives::phrase(&["if", "you", "win"]),
            alt((
                primitives::phrase(&["the", "clash"]),
                primitives::phrase(&["that", "clash"]),
            )),
            peek(primitives::comma()),
        )
            .value(ConditionalFollowupKind::IfYouWinClash),
        (
            primitives::phrase(&["if", "you", "win"]),
            primitives::phrase(&["the", "flip"]),
            peek(primitives::comma()),
        )
            .value(ConditionalFollowupKind::IfYouWinFlip),
        (
            primitives::phrase(&["if", "you", "win"]),
            peek(primitives::comma()),
        )
            .value(ConditionalFollowupKind::IfYouWin),
    ))
    .parse_next(input)
}

fn parse_conditional_followup_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalFollowupShape<'a>> {
    let kind = conditional_followup_prefix.parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::comma()))
        .void()
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let continuation_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ConditionalFollowupShape {
        kind,
        continuation_tokens,
    })
}

pub(crate) fn parse_conditional_followup(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalFollowupShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_conditional_followup_lexed,
        "conditional subject-verb followup",
    )
    .ok()
}

pub(crate) fn is_anaphoric_damage_self_replacement(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    words.starts_with(&["it", "deals"])
        && words
            .windows(3)
            .any(|window| window == ["to", "that", "creature"])
        && words.contains(&"instead")
}

fn lifecycle_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("exile"), primitives::kw("sacrifice")))
        .void()
        .parse_next(input)
}

fn pronoun_trigger_prefix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["when", "it"]),
        primitives::phrase(&["whenever", "it"]),
        primitives::phrase(&["when", "they"]),
        primitives::phrase(&["whenever", "they"]),
    ))
    .parse_next(input)
}

pub(crate) fn token_reminder_followup_facts(
    tokens: &[OwnedLexToken],
) -> TokenReminderFollowupFacts {
    let lifecycle_head = primitives::parse_prefix(tokens, lifecycle_head).is_some();
    let has_pronoun = marker_anywhere(tokens, alt((primitives::kw("it"), primitives::kw("them"))));
    TokenReminderFollowupFacts {
        lifecycle_head,
        delayed_pronoun_lifecycle: lifecycle_head && has_pronoun,
        pronoun_trigger_prefix: primitives::parse_prefix(tokens, pronoun_trigger_prefix).is_some(),
    }
}

#[cfg(test)]
#[path = "followup_shapes/tests.rs"]
mod tests;
