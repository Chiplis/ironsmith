use crate::color::ColorSet;
use crate::effect::Value;
use crate::lexer::{
    OwnedLexToken, parser_token_word_positions, parser_token_word_refs, trim_lexed_commas,
};
use crate::types::{CardType, Subtype};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{leaf, permission_shapes, primitives};

const ADDITION_TAILS: &[&[&str]] = &[
    &["in", "addition", "to", "its", "other", "types"],
    &["in", "addition", "to", "their", "other", "types"],
    &["in", "addition", "to", "its", "other", "type"],
    &["in", "addition", "to", "their", "other", "type"],
];

#[derive(Debug, Clone)]
pub struct BecomeCreatureDescriptor {
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub colors: Option<ColorSet>,
}

#[derive(Debug, Clone)]
pub struct BecomeLeadingPtShape<'a> {
    pub power: Value,
    pub toughness: Value,
    pub value_word_count: usize,
    pub creature_word_index: Option<usize>,
    pub suffix_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub struct BecomeLeadingCreaturePrefix {
    pub supported: bool,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub colors: Option<ColorSet>,
}

#[derive(Debug, Clone, Copy)]
pub enum BecomeAnimationSuffixShape<'a> {
    Ignored {
        preserve_other_types: bool,
        type_retention_surface: Option<ironsmith_core::TypeRetentionSurface>,
    },
    With {
        ability_tokens: &'a [OwnedLexToken],
        grants_all_creature_types: bool,
        preserve_other_types: bool,
        type_retention_surface: Option<ironsmith_core::TypeRetentionSurface>,
    },
    Unsupported,
}

#[derive(Debug, Clone)]
pub enum BecomeSimpleDescriptorShape {
    ColorsAndSubtypes {
        colors: ColorSet,
        subtypes: Vec<Subtype>,
    },
    CardTypes {
        card_types: Vec<CardType>,
        preserve_other_types: bool,
    },
    Subtypes {
        subtypes: Vec<Subtype>,
        replace_creature_subtypes: bool,
    },
    None,
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}

fn parse_pt_value_words(words: &[&str]) -> Option<(Value, Value, usize)> {
    if let Some(first) = words.first()
        && let Ok((power, toughness)) = leaf::parse_leaf_pt_modifier_values_complete(first)
    {
        return Some((power, toughness, 1));
    }
    let (first, second) = (words.first()?, words.get(1)?);
    let joined = format!("{first}/{second}");
    let (power, toughness) = leaf::parse_leaf_pt_modifier_values_complete(&joined).ok()?;
    Some((power, toughness, 2))
}

pub fn parse_become_leading_pt_shape<'a>(
    words: &[&str],
    body_tokens: &'a [OwnedLexToken],
) -> Option<BecomeLeadingPtShape<'a>> {
    let (power, toughness, value_word_count) = parse_pt_value_words(words)?;
    let creature_word_index = permission_shapes::find_words(words, &["creature"])
        .or_else(|| permission_shapes::find_words(words, &["creatures"]));
    let suffix_tokens = if creature_word_index.is_some() {
        primitives::find_prefix(body_tokens, || {
            alt((
                primitives::kw("creature").void(),
                primitives::kw("creatures").void(),
            ))
        })
        .map(|(_, _, rest)| trim_lexed_commas(rest))
        .unwrap_or_default()
    } else {
        &[]
    };
    Some(BecomeLeadingPtShape {
        power,
        toughness,
        value_word_count,
        creature_word_index,
        suffix_tokens,
    })
}

pub fn parse_become_leading_creature_prefix(words: &[&str]) -> BecomeLeadingCreaturePrefix {
    let mut card_types = vec![CardType::Creature];
    let mut subtypes = Vec::new();
    let mut colors = ColorSet::new();
    let mut index = 0;
    while index < words.len() {
        let word = words[index];
        if matches!(word, "a" | "an" | "the" | "and") {
            index += 1;
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
            index += 1;
            continue;
        }
        if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
            if card_type != CardType::Creature {
                push_unique(&mut card_types, card_type);
            }
            index += 1;
            continue;
        }
        if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            push_unique(&mut subtypes, subtype);
            index += 1;
            continue;
        }
        // A raw hyphenated type is one lexer token, but document/name
        // normalization may reconstruct it as two synthetic words. Recover
        // the same typed subtype before falling back to a bare creature.
        if let Some(next) = words.get(index + 1) {
            let compound = format!("{word}-{next}");
            if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(&compound) {
                push_unique(&mut subtypes, subtype);
                index += 2;
                continue;
            }
        }
        return BecomeLeadingCreaturePrefix {
            supported: false,
            card_types: vec![CardType::Creature],
            subtypes: Vec::new(),
            colors: None,
        };
    }
    BecomeLeadingCreaturePrefix {
        supported: true,
        card_types,
        subtypes,
        colors: (!colors.is_empty()).then_some(colors),
    }
}

pub fn parse_become_creature_descriptor_words(words: &[&str]) -> Option<BecomeCreatureDescriptor> {
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut colors = ColorSet::new();
    let mut saw_subtype = false;
    let mut index = 0;
    while index < words.len() {
        let word = words[index];
        if matches!(word, "a" | "an" | "the" | "and" | "or") {
            index += 1;
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
            index += 1;
        } else if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
            push_unique(&mut card_types, card_type);
            index += 1;
        } else if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            push_unique(&mut subtypes, subtype);
            saw_subtype = true;
            index += 1;
        } else if let Some(next) = words.get(index + 1) {
            let compound = format!("{word}-{next}");
            let subtype = leaf::parse_leaf_subtype_flexible_complete(&compound).ok()?;
            push_unique(&mut subtypes, subtype);
            saw_subtype = true;
            index += 2;
        } else {
            return None;
        }
    }
    if saw_subtype && !card_types.contains(&CardType::Creature) {
        card_types.insert(0, CardType::Creature);
    }
    if card_types.is_empty() && !saw_subtype {
        return None;
    }
    Some(BecomeCreatureDescriptor {
        card_types,
        subtypes,
        colors: (!colors.is_empty()).then_some(colors),
    })
}

pub fn strip_become_addition_tail_words<'a>(words: &'a [&'a str]) -> (&'a [&'a str], bool) {
    for tail in ADDITION_TAILS {
        if permission_shapes::suffix_words(words, tail) {
            return (&words[..words.len().saturating_sub(tail.len())], true);
        }
    }
    (words, false)
}

fn strip_addition_tail_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::strip_lexed_suffix_phrases(tokens, ADDITION_TAILS)
        .map(|(_, head)| head)
        .unwrap_or(tokens)
}

const STILL_A_LAND_TAILS: &[&[&str]] = &[
    &["still", "a", "land"],
    &["that", "s", "still", "a", "land"],
    &["thats", "still", "a", "land"],
    &["it", "s", "still", "a", "land"],
    &["its", "still", "a", "land"],
];

const STILL_A_CARD_TYPE_PREFIXES: &[&[&str]] = &[
    &["that", "s", "still", "a"],
    &["thats", "still", "a"],
    &["it", "s", "still", "a"],
    &["its", "still", "a"],
    &["still", "a"],
];

fn strip_still_a_land_tail_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::strip_lexed_suffix_phrases(tokens, STILL_A_LAND_TAILS)
        .map(|(_, head)| trim_lexed_commas(head))
        .unwrap_or(tokens)
}

fn still_a_land(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    STILL_A_LAND_TAILS
        .iter()
        .any(|expected| permission_shapes::exact_words(&words, expected))
}

fn split_still_a_card_type_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], CardType)> {
    let positions = parser_token_word_positions(tokens);
    let (_, card_type_word) = positions.last()?;
    let card_type = leaf::parse_leaf_card_type_complete(card_type_word).ok()?;
    let words = positions.iter().map(|(_, word)| *word).collect::<Vec<_>>();
    for prefix in STILL_A_CARD_TYPE_PREFIXES {
        let suffix_len = prefix.len() + 1;
        if positions.len() < suffix_len {
            continue;
        }
        let suffix_start = positions.len() - suffix_len;
        let prefix_matches = words[suffix_start..words.len() - 1] == **prefix;
        if prefix_matches {
            let start_token = positions[suffix_start].0;
            return Some((trim_lexed_commas(&tokens[..start_token]), card_type));
        }
    }
    None
}

fn split_all_creature_types(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    if permission_shapes::exact_tokens(tokens, &["all", "creature", "types"]) {
        return Some(&[]);
    }
    primitives::strip_lexed_suffix_phrase(tokens, &["and", "all", "creature", "types"])
        .map(trim_lexed_commas)
}

#[cfg(test)]
#[path = "descriptors_inline_tests.rs"]
mod tests;

#[path = "descriptors/object_action_programs.rs"]
mod object_action_programs;
pub use object_action_programs::{
    parse_become_animation_suffix_shape, parse_become_color_words,
    parse_become_simple_descriptor_words,
};
#[path = "descriptors/core_programs.rs"]
mod core_programs;
use core_programs::creature_subtypes_only;
