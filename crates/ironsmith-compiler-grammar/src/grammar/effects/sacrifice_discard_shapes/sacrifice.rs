use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::mana::ManaSymbol;
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{leaf, primitives};
use super::common;

const TAGGED_REFERENCES: &[&[&str]] = &[
    &["it"],
    &["that", "card"],
    &["that", "creature"],
    &["the", "creature"],
    &["that", "permanent"],
    &["the", "permanent"],
    &["that", "token"],
    &["the", "token"],
];
const ONE_OF_TAGGED_SET_REFERENCES: &[&[&str]] = &[&["one", "of", "them"]];
const ALL_OF_TAGGED_SET_REFERENCES: &[&[&str]] = &[
    &["those", "permanents"],
    &["those", "creatures"],
    &["those", "tokens"],
];
const CHOICE_SUFFIXES: &[&[&str]] = &[
    &["of", "their", "choice"],
    &["of", "your", "choice"],
    &["of", "its", "choice"],
    &["of", "his", "or", "her", "choice"],
];
const ATTACHED_EXCLUSIONS: &[&[&str]] = &[
    &["than", "enchanted", "creature"],
    &["than", "enchanted", "permanent"],
    &["than", "equipped", "creature"],
    &["than", "equipped", "permanent"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SacrificeUnlessKind {
    None,
    Escaped,
    ManaSpent(ManaSymbol),
    OpponentDamagedThisTurn,
    General,
}

#[derive(Debug, Clone, Copy)]
pub struct SacrificeClauseShape<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub full_body_tokens: &'a [OwnedLexToken],
    pub unless_token_offset: Option<usize>,
    pub unless_kind: SacrificeUnlessKind,
    pub sacrifice_references_it: bool,
    pub has_graveyard_history: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum SacrificeQuantityShape<'a> {
    ThatMany {
        filter_tokens: &'a [OwnedLexToken],
    },
    AllOrEach {
        filter_tokens: &'a [OwnedLexToken],
        other: bool,
        each_surface: bool,
    },
    AllExcept {
        filter_tokens: &'a [OwnedLexToken],
        keep_count: u32,
        other: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SacrificeAggregateKind {
    GreatestManaValue,
    GreatestPower,
}

#[derive(Debug, Clone, Copy)]
pub struct SacrificeAggregateShape<'a> {
    pub kind: SacrificeAggregateKind,
    pub object_tokens: &'a [OwnedLexToken],
    pub among_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SacrificeTaggedReferenceKind {
    ItOrCard,
    Token,
    OneOfTaggedSet,
    AllOfTaggedSet,
}

#[derive(Debug, Clone, Copy)]
pub struct SacrificeObjectShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub tagged_reference: Option<SacrificeTaggedReferenceKind>,
}

#[derive(Debug, Clone, Copy)]
pub struct SacrificeCountShape<'a> {
    pub count: u32,
    pub other: bool,
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct SacrificeFractionRoundedShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub denominator: u32,
    pub rounded_up: bool,
}

pub fn parse_sacrifice_mana_spent_symbol(tokens: &[OwnedLexToken]) -> Option<ManaSymbol> {
    let [mana_token, rest @ ..] = tokens else {
        return None;
    };
    let rest_words = parser_token_word_refs(rest);
    if !common::exact_any(
        &rest_words,
        &[
            &["was", "spent", "to", "cast", "it"],
            &["was", "spent", "to", "cast", "this", "spell"],
        ],
    ) {
        return None;
    }
    let symbols = leaf::parse_leaf_mana_symbol_group_complete(mana_token.slice.as_str()).ok()?;
    let [symbol] = symbols.as_slice() else {
        return None;
    };
    Some(*symbol)
}

pub fn parse_sacrifice_clause_shape(tokens: &[OwnedLexToken]) -> SacrificeClauseShape<'_> {
    let full_body_tokens = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("sacrifice").void(),
            primitives::kw("sacrifices").void(),
        )),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens);
    let full_words = parser_token_word_refs(full_body_tokens);
    let Some((unless_token_offset, _, after_unless)) =
        primitives::find_prefix(full_body_tokens, || primitives::kw("unless").void())
    else {
        return SacrificeClauseShape {
            body_tokens: full_body_tokens,
            full_body_tokens,
            unless_token_offset: None,
            unless_kind: SacrificeUnlessKind::None,
            sacrifice_references_it: false,
            has_graveyard_history: common::all_present(
                &full_words,
                &["for", "each", "graveyard", "turn"],
            ),
        };
    };

    let body_tokens = &full_body_tokens[..unless_token_offset];
    let body_words = parser_token_word_refs(body_tokens);
    let after_words = parser_token_word_refs(after_unless);
    let unless_kind = if common::exact(&after_words, &["it", "escaped"]) {
        SacrificeUnlessKind::Escaped
    } else if let Some(symbol) = parse_sacrifice_mana_spent_symbol(after_unless) {
        SacrificeUnlessKind::ManaSpent(symbol)
    } else if common::exact(
        &after_words,
        &["an", "opponent", "was", "dealt", "damage", "this", "turn"],
    ) {
        SacrificeUnlessKind::OpponentDamagedThisTurn
    } else {
        SacrificeUnlessKind::General
    };
    SacrificeClauseShape {
        body_tokens,
        full_body_tokens,
        unless_token_offset: Some(unless_token_offset),
        unless_kind,
        sacrifice_references_it: common::exact_any(&body_words, TAGGED_REFERENCES)
            || common::exact_any(&body_words, ONE_OF_TAGGED_SET_REFERENCES)
            || common::exact_any(&body_words, ALL_OF_TAGGED_SET_REFERENCES),
        has_graveyard_history: common::all_present(
            &body_words,
            &["for", "each", "graveyard", "turn"],
        ),
    }
}

pub fn parse_sacrifice_quantity_shape(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeQuantityShape<'_>> {
    if let Some((_, rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["that", "many"]).void())
    {
        return Some(SacrificeQuantityShape::ThatMany {
            filter_tokens: rest,
        });
    }
    let (each_surface, mut rest) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::kw("all").value(false),
            primitives::kw("each").value(true),
        )),
    )?;
    let mut other = false;
    if let Some((_, after_other)) = primitives::parse_prefix(
        rest,
        alt((
            primitives::kw("other").void(),
            primitives::kw("another").void(),
        )),
    ) {
        other = true;
        rest = after_other;
    }
    if let Some((except_offset, _, after_except)) =
        primitives::find_prefix(rest, || primitives::phrase(&["except", "for"]).void())
        && except_offset > 0
        && let Some(prefix) = leaf::parse_leaf_number_prefix_tokens(after_except)
        && let Some((keep_count, used)) = prefix.into_fixed()
        && keep_count > 0
        && used == after_except.len()
    {
        return Some(SacrificeQuantityShape::AllExcept {
            filter_tokens: &rest[..except_offset],
            keep_count,
            other,
        });
    }
    Some(SacrificeQuantityShape::AllOrEach {
        filter_tokens: rest,
        other,
        each_surface,
    })
}

pub fn parse_sacrifice_fraction_rounded_shape(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeFractionRoundedShape<'_>> {
    let (denominator, rest) = if let Some((_, rest)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["half", "the"]).void())
    {
        (2, rest)
    } else {
        let (_, after_article) = primitives::parse_prefix(tokens, primitives::kw("a").void())?;
        (1..after_article.len()).find_map(|of_index| {
            let (_, rest) = primitives::parse_prefix(
                &after_article[of_index..],
                primitives::phrase(&["of", "the"]).void(),
            )?;
            let ordinal_words = parser_token_word_refs(&after_article[..of_index]);
            let (denominator, used) = ironsmith_core::parse_ordinal_words(&ordinal_words)?;
            (denominator > 1 && used == ordinal_words.len()).then_some((denominator, rest))
        })?
    };
    let (rounded_up, before_rounding) = if let Some((_, stripped)) =
        primitives::strip_lexed_suffix_phrases(rest, &[&["rounded", "up"]])
    {
        (true, stripped)
    } else if let Some((_, stripped)) =
        primitives::strip_lexed_suffix_phrases(rest, &[&["rounded", "down"]])
    {
        (false, stripped)
    } else {
        return None;
    };
    let object = parse_sacrifice_object_shape(before_rounding);
    (!object.filter_tokens.is_empty()).then_some(SacrificeFractionRoundedShape {
        filter_tokens: object.filter_tokens,
        denominator,
        rounded_up,
    })
}

#[cfg(test)]
#[path = "sacrifice_inline_tests.rs"]
mod tests;

#[path = "sacrifice/resource_programs.rs"]
mod resource_programs;
pub use resource_programs::{
    parse_sacrifice_aggregate_shape, parse_sacrifice_attached_exclusion,
    parse_sacrifice_count_shape,
};
#[path = "sacrifice/reference_programs.rs"]
mod reference_programs;
pub use reference_programs::parse_sacrifice_object_shape;
