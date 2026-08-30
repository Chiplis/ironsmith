use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::ChoiceCount;
use crate::grammar::{leaf, primitives};
use crate::lexer::{OwnedLexToken, trim_lexed_commas};
use crate::mana::ManaSymbol;

#[derive(Debug, Clone, Copy)]
pub struct ForEachObjectSubjectShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct ForEachObjectEffectShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct ForEachTargetSubjectShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct ForEachTargetPlayersShape<'a> {
    pub count: ChoiceCount,
    pub target_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct ForEachSpentManaEffectShape<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub reference: ironsmith_core::ManaSpentCastReferenceSurface,
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct ForEachManaSymbolSpentEffectShape<'a> {
    pub symbol: ManaSymbol,
    pub group_size: u32,
    pub reference: ironsmith_core::ManaSpentCastReferenceSurface,
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct ForEachDynamicTargetEffectShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub effect_tokens: &'a [OwnedLexToken],
}

fn for_each_prefix<'a>(input: &mut crate::lexer::LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        primitives::phrase(&["for", "each"]),
        primitives::kw("each").void(),
    ))
    .void()
    .parse_next(input)
}

fn participant_prefix<'a>(
    input: &mut crate::lexer::LexStream<'a>,
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
    input: &mut crate::lexer::LexStream<'a>,
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

#[path = "subjects/iterated_effects.rs"]
mod iterated_effects;
pub use iterated_effects::*;

#[cfg(test)]
#[path = "subjects/tests.rs"]
mod tests;

#[path = "subjects/reference.rs"]
mod reference_programs;
use reference_programs::contains_effect_verb_outside_filter_zone;
pub use reference_programs::{
    parse_for_each_object_effect_shape, parse_for_each_object_subject_shape,
    parse_for_each_target_players_shape, parse_for_each_target_subject_shape,
};
