use crate::color::ColorSet;
use crate::lexer::{
    OwnedLexToken, parser_token_word_positions, parser_token_word_refs, render_token_slice,
    trim_lexed_commas,
};
use crate::target::SourceReferenceSurface;
use crate::types::{CardType, Subtype, Supertype};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{leaf, permission_shapes, primitives};

const COPY_NAME_PREFIXES: &[&[&str]] = &[
    &["its", "name", "is"],
    &["it", "s", "name", "is"],
    &["his", "name", "is"],
    &["her", "name", "is"],
];
const COPY_PRESERVE_TAILS: &[&[&str]] = &[
    &["and", "it", "has", "this", "ability"],
    &["and", "this", "ability"],
];
const COPY_LEGENDARY_TAILS: &[&[&str]] = &[
    &[
        "and",
        "its",
        "legendary",
        "in",
        "addition",
        "to",
        "its",
        "other",
        "types",
    ],
    &[
        "and",
        "it",
        "s",
        "legendary",
        "in",
        "addition",
        "to",
        "its",
        "other",
        "types",
    ],
    &[
        "and",
        "it",
        "is",
        "legendary",
        "in",
        "addition",
        "to",
        "its",
        "other",
        "types",
    ],
];
const COLOR_CHOICES: &[&[&str]] = &[
    &["color", "of", "your", "choice"],
    &["color", "or", "colors", "of", "your", "choice"],
    &["colors", "of", "your", "choice"],
];
const SOURCE_POWER_TOUGHNESS: &[&[&str]] = &[
    &["this", "power", "and", "toughness"],
    &["thiss", "power", "and", "toughness"],
    &["source", "power", "and", "toughness"],
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BecomeCopyExceptionShape {
    pub preserve_source_abilities: bool,
    pub name_override: Option<String>,
    pub name_override_surface: Option<SourceReferenceSurface>,
    pub add_supertypes: Vec<Supertype>,
    pub remove_supertypes: Vec<Supertype>,
    pub add_colors: ColorSet,
    pub add_card_types: Vec<CardType>,
    pub set_card_types: Vec<CardType>,
    pub add_subtypes: Vec<Subtype>,
    pub set_subtypes: Vec<Subtype>,
    pub granted_ability_tokens: Option<Vec<OwnedLexToken>>,
    pub set_base_power_toughness: Option<(i32, i32)>,
    pub surface: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BecomeRestShape {
    pub rest_tokens: Vec<OwnedLexToken>,
    pub body_tokens: Vec<OwnedLexToken>,
    pub copy_exception: Option<BecomeCopyExceptionShape>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BecomeExactKind {
    Monarch,
    BasicLandTypeChoice,
    BasicLandType(Subtype),
    ColorChoice { allow_multiple: bool },
    CreatureTypeChoice,
    Colorless,
    Saddled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BecomeCopySourceShape<'a> {
    NotCopy,
    Missing,
    Source(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy)]
pub struct BecomeAuraShape {
    pub attachment_you_control: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct BecomeBodySurfaceShape<'a> {
    pub body_tokens: &'a [OwnedLexToken],
    pub exact_kind: Option<BecomeExactKind>,
    pub copy_source: BecomeCopySourceShape<'a>,
    pub aura: Option<BecomeAuraShape>,
    pub equal_to_source_power_toughness: bool,
}

fn split_last_except(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let mut search_tokens = tokens;
    let mut search_offset = 0usize;
    let mut last_offset = None;
    while let Some((relative, _, after_except)) =
        primitives::find_prefix(search_tokens, || primitives::kw("except").void())
    {
        let marker_offset = search_offset + relative;
        last_offset = Some(marker_offset);
        let consumed = search_tokens.len().saturating_sub(after_except.len());
        search_offset += consumed;
        search_tokens = after_except;
    }
    let marker_offset = last_offset?;
    Some((
        trim_lexed_commas(&tokens[..marker_offset]),
        trim_lexed_commas(&tokens[marker_offset + 1..]),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyExceptionFollowupKind {
    Copula,
    Has,
}

fn find_word_phrase_token_span(
    tokens: &[OwnedLexToken],
    phrase: &[&str],
) -> Option<(usize, usize)> {
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let start_word = crate::word_primitives::parse_sequence_start(&words, phrase)?;
    let span = view.token_span_for_words(start_word, start_word + phrase.len())?;
    Some((span.start, span.end))
}

fn find_copy_exception_followup(
    tokens: &[OwnedLexToken],
    include_bare_copula: bool,
) -> Option<(usize, usize, CopyExceptionFollowupKind)> {
    const HAS_PHRASES: &[&[&str]] = &[
        &["and", "he", "has"],
        &["and", "she", "has"],
        &["and", "it", "has"],
        &["and", "has"],
    ];
    const COPULA_PHRASES: &[&[&str]] = &[
        &["and", "hes"],
        &["and", "shes"],
        &["and", "its"],
        &["hes"],
        &["shes"],
        &["its"],
    ];

    let mut matches = Vec::new();
    for phrase in HAS_PHRASES {
        if let Some((start, end)) = find_word_phrase_token_span(tokens, phrase) {
            matches.push((start, end, CopyExceptionFollowupKind::Has));
        }
    }
    for phrase in COPULA_PHRASES {
        if !include_bare_copula {
            continue;
        }
        if let Some((start, end)) = find_word_phrase_token_span(tokens, phrase) {
            matches.push((start, end, CopyExceptionFollowupKind::Copula));
        }
    }
    matches
        .into_iter()
        .min_by_key(|(start, end, _)| (*start, *end))
}

fn parse_fixed_power_toughness(word: &str) -> Option<(i32, i32)> {
    let (power, toughness) = word.split_once('/')?;
    Some((power.parse().ok()?, toughness.parse().ok()?))
}

#[cfg(test)]
#[path = "surface_inline_tests.rs"]
mod tests;

#[path = "surface/combat_programs.rs"]
mod combat_programs;
pub use combat_programs::parse_become_attack_color;
#[path = "surface/object_action_programs.rs"]
mod object_action_programs;
use object_action_programs::parse_structured_become_copy_exception_shape;
pub use object_action_programs::{
    parse_become_body_surface_shape, parse_become_copy_exception_shape, parse_become_rest_shape,
};
#[path = "surface/core_programs.rs"]
mod core_programs;
use core_programs::basic_land_type;
