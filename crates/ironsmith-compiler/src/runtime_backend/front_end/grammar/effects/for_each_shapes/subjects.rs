use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::ChoiceCount;
use crate::mana::ManaSymbol;
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
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachSpentManaEffectShape<'a> {
    pub(crate) source_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachManaSymbolSpentEffectShape<'a> {
    pub(crate) symbol: ManaSymbol,
    pub(crate) group_size: u32,
    pub(crate) reference: ironsmith_core::ManaSpentCastReferenceSurface,
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
    // A leading bare "Each" is an ordinary quantified subject (for example,
    // "Each creature that isn't an Insect, Rat, Spider, or Squirrel gets ...").
    // Treat only an explicit "For each ...," prefix as an iterator sentence;
    // otherwise a subtype-list comma can be mistaken for the effect boundary.
    primitives::parse_prefix(
        trim_lexed_commas(tokens),
        primitives::phrase(&["for", "each"]),
    )?;
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
        alt((
            primitives::kw("player"),
            primitives::kw("players"),
            primitives::kw("opponent"),
            primitives::kw("opponents"),
        ))
        .void(),
    )?;
    // Do not mistake the counted-set suffix in an ordinary action such as
    // "target player creates ... for each card ..." for the iterator marker.
    // A qualifier between the player phrase and `each` remains supported.
    let (each_index, effect_tokens) =
        after_player.iter().enumerate().find_map(|(index, token)| {
            if !token.is_word("each")
                || after_player
                    .get(index.saturating_sub(1))
                    .is_some_and(|previous| previous.is_word("for"))
            {
                return None;
            }
            Some((index, trim_lexed_commas(after_player.get(index + 1..)?)))
        })?;
    let target_len = after_count.len().checked_sub(after_player.len())? + each_index;
    let target_tokens = trim_lexed_commas(after_count.get(..target_len)?);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    (!target_tokens.is_empty() && !effect_tokens.is_empty()).then_some(ForEachTargetPlayersShape {
        count,
        target_tokens,
        effect_tokens,
    })
}

#[cfg(test)]
#[path = "subjects/tests.rs"]
mod tests;
