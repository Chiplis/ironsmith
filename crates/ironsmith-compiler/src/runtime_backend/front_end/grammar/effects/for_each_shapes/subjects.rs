use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::ChoiceCount;
use crate::runtime_backend::front_end::grammar::{leaf, primitives};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachObjectSubjectShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachObjectEffectShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachTargetSubjectShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachTargetPlayersShape<'a> {
    pub(crate) count: ChoiceCount,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachSpentManaEffectShape<'a> {
    pub(crate) source_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachDynamicTargetEffectShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

fn for_each_prefix<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((
        primitives::phrase(&["for", "each"]),
        primitives::kw("each").void(),
    ))
    .void()
    .parse_next(input)
}

fn participant_prefix<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    alt((
        alt((primitives::kw("player"), primitives::kw("players"))).void(),
        alt((primitives::kw("opponent"), primitives::kw("opponents"))).void(),
        (
            alt((primitives::kw("other"), primitives::kw("target"))),
            alt((
                primitives::kw("player"),
                primitives::kw("players"),
                primitives::kw("opponent"),
                primitives::kw("opponents"),
            )),
        )
            .void(),
    ))
    .void()
    .parse_next(input)
}

fn attached_creature_tail<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> winnow::error::ModalResult<()> {
    primitives::phrase(&["attached", "to"]).parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::kw("creature").parse_next(input)?;
    Ok(())
}

fn normalized_filter_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let tokens = trim_lexed_commas(tokens);
    let attached = primitives::find_prefix(tokens, || attached_creature_tail)
        .map(|(index, _, _)| index)
        .unwrap_or(tokens.len());
    trim_lexed_commas(tokens.get(..attached).unwrap_or_default())
}

pub(crate) fn parse_for_each_object_subject_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachObjectSubjectShape<'_>> {
    let (_, rest) = primitives::parse_prefix(trim_lexed_commas(tokens), for_each_prefix)?;
    let rest = primitives::parse_prefix(rest, opt(primitives::kw("of")).void())
        .map(|(_, rest)| rest)
        .unwrap_or(rest);
    let filter_tokens = normalized_filter_tokens(rest);
    if filter_tokens.is_empty()
        || primitives::parse_prefix(filter_tokens, participant_prefix).is_some()
        || super::super::chain_splitting::find_chain_verb_tokens(filter_tokens).is_some()
    {
        return None;
    }
    Some(ForEachObjectSubjectShape { filter_tokens })
}

pub(crate) fn parse_for_each_object_effect_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachObjectEffectShape<'_>> {
    let (subject_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let subject = parse_for_each_object_subject_shape(subject_tokens)?;
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!effect_tokens.is_empty()).then_some(ForEachObjectEffectShape {
        filter_tokens: subject.filter_tokens,
        effect_tokens,
    })
}

#[path = "subjects/iterated_effects.rs"]
mod iterated_effects;
pub(crate) use iterated_effects::*;

pub(crate) fn parse_for_each_target_subject_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachTargetSubjectShape<'_>> {
    let (_, rest) = primitives::parse_prefix(trim_lexed_commas(tokens), for_each_prefix)?;
    let target_tokens = primitives::parse_prefix(rest, opt(primitives::kw("of")).void())
        .map(|(_, rest)| trim_lexed_commas(rest))
        .unwrap_or_else(|| trim_lexed_commas(rest));
    (!target_tokens.is_empty()).then_some(ForEachTargetSubjectShape { target_tokens })
}

pub(crate) fn parse_for_each_target_players_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachTargetPlayersShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let tokens = primitives::parse_prefix(tokens, opt(primitives::kw("then")).void())
        .map(|(_, rest)| trim_lexed_commas(rest))
        .unwrap_or(tokens);
    let parsed_count = leaf::parse_leaf_choice_count_prefix_tokens(tokens);
    let (count, after_count) = parsed_count
        .as_ref()
        .and_then(|parsed| {
            let rest = tokens.get(parsed.consumed..)?;
            primitives::parse_prefix(rest, primitives::kw("target"))?;
            Some((parsed.count.clone(), rest))
        })
        .unwrap_or_else(|| (ChoiceCount::exactly(1), tokens));
    let (_, after_target) = primitives::parse_prefix(after_count, primitives::kw("target"))?;
    let (_, after_player) = primitives::parse_prefix(
        after_target,
        alt((primitives::kw("player"), primitives::kw("players"))).void(),
    )?;
    let (_, effect_tokens) = primitives::parse_prefix(after_player, primitives::kw("each"))?;
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!effect_tokens.is_empty()).then_some(ForEachTargetPlayersShape {
        count,
        effect_tokens,
    })
}

#[cfg(test)]
#[path = "subjects/tests.rs"]
mod tests;
