use crate::color::ColorSet;
use crate::effect::Value;
use crate::runtime_backend::front_end::lexer::{
    OwnedLexToken, parser_token_word_refs, trim_lexed_commas,
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
pub(crate) struct BecomeCreatureDescriptor {
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) colors: Option<ColorSet>,
}

#[derive(Debug, Clone)]
pub(crate) struct BecomeLeadingPtShape<'a> {
    pub(crate) power: Value,
    pub(crate) toughness: Value,
    pub(crate) value_word_count: usize,
    pub(crate) creature_word_index: Option<usize>,
    pub(crate) suffix_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct BecomeLeadingCreaturePrefix {
    pub(crate) supported: bool,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) colors: Option<ColorSet>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BecomeAnimationSuffixShape<'a> {
    Ignored,
    With {
        ability_tokens: &'a [OwnedLexToken],
        grants_all_creature_types: bool,
    },
    Unsupported,
}

#[derive(Debug, Clone)]
pub(crate) enum BecomeSimpleDescriptorShape {
    ColorsAndSubtypes {
        colors: ColorSet,
        subtypes: Vec<Subtype>,
    },
    CardTypes(Vec<CardType>),
    Subtypes {
        subtypes: Vec<Subtype>,
        replace_creature_subtypes: bool,
    },
    None,
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.iter().any(|existing| *existing == item) {
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

pub(crate) fn parse_become_leading_pt_shape<'a>(
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

pub(crate) fn parse_become_leading_creature_prefix(words: &[&str]) -> BecomeLeadingCreaturePrefix {
    let mut card_types = vec![CardType::Creature];
    let mut subtypes = Vec::new();
    let mut colors = ColorSet::new();
    for word in words {
        if matches!(*word, "a" | "an" | "the" | "and") {
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
            continue;
        }
        if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
            if card_type != CardType::Creature {
                push_unique(&mut card_types, card_type);
            }
            continue;
        }
        if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            push_unique(&mut subtypes, subtype);
            continue;
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

pub(crate) fn parse_become_creature_descriptor_words(
    words: &[&str],
) -> Option<BecomeCreatureDescriptor> {
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut colors = ColorSet::new();
    let mut saw_subtype = false;
    for word in words {
        if matches!(*word, "a" | "an" | "the" | "and" | "or") {
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
        } else if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
            push_unique(&mut card_types, card_type);
        } else if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            push_unique(&mut subtypes, subtype);
            saw_subtype = true;
        } else {
            return None;
        }
    }
    if saw_subtype && !card_types.iter().any(|kind| *kind == CardType::Creature) {
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

pub(crate) fn strip_become_addition_tail_words<'a>(words: &'a [&'a str]) -> (&'a [&'a str], bool) {
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

fn still_a_land(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    [
        &["still", "a", "land"][..],
        &["that", "s", "still", "a", "land"],
        &["thats", "still", "a", "land"],
        &["it", "s", "still", "a", "land"],
        &["its", "still", "a", "land"],
    ]
    .iter()
    .any(|expected| permission_shapes::exact_words(&words, expected))
}

fn split_all_creature_types(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    if permission_shapes::exact_tokens(tokens, &["all", "creature", "types"]) {
        return Some(&[]);
    }
    primitives::strip_lexed_suffix_phrase(tokens, &["and", "all", "creature", "types"])
        .map(trim_lexed_commas)
}

pub(crate) fn parse_become_animation_suffix_shape(
    tokens: &[OwnedLexToken],
) -> BecomeAnimationSuffixShape<'_> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty()
        || (!tokens.is_empty() && strip_addition_tail_tokens(tokens).is_empty())
        || still_a_land(tokens)
    {
        return BecomeAnimationSuffixShape::Ignored;
    }
    let Some((_, after_with)) = primitives::parse_prefix(tokens, primitives::kw("with").void())
    else {
        return BecomeAnimationSuffixShape::Unsupported;
    };
    let ability_tokens = strip_addition_tail_tokens(trim_lexed_commas(after_with));
    if let Some(without_family) = split_all_creature_types(ability_tokens) {
        return BecomeAnimationSuffixShape::With {
            ability_tokens: without_family,
            grants_all_creature_types: true,
        };
    }
    BecomeAnimationSuffixShape::With {
        ability_tokens,
        grants_all_creature_types: false,
    }
}

fn creature_subtypes_only(subtypes: &[Subtype]) -> bool {
    let creature_types = Subtype::all_creature_types();
    subtypes
        .iter()
        .all(|subtype| creature_types.iter().any(|candidate| candidate == subtype))
}

pub(crate) fn parse_become_simple_descriptor_words(words: &[&str]) -> BecomeSimpleDescriptorShape {
    let (words, had_addition_tail) = strip_become_addition_tail_words(words);
    if words.is_empty() {
        return BecomeSimpleDescriptorShape::None;
    }

    let mut colors = ColorSet::new();
    let mut subtypes = Vec::new();
    let mut color_or_subtype = true;
    for word in words {
        if *word == "and" {
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
        } else if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
            push_unique(&mut subtypes, subtype);
        } else {
            color_or_subtype = false;
            break;
        }
    }
    if color_or_subtype && !colors.is_empty() && !subtypes.is_empty() {
        return BecomeSimpleDescriptorShape::ColorsAndSubtypes { colors, subtypes };
    }

    let mut card_types = Vec::new();
    if words.iter().all(|word| {
        leaf::parse_leaf_card_type_complete(word)
            .map(|card_type| push_unique(&mut card_types, card_type))
            .is_ok()
    }) && !card_types.is_empty()
    {
        return BecomeSimpleDescriptorShape::CardTypes(card_types);
    }

    let mut subtypes = Vec::new();
    if words.iter().all(|word| {
        leaf::parse_leaf_subtype_flexible_complete(word)
            .map(|subtype| push_unique(&mut subtypes, subtype))
            .is_ok()
    }) && !subtypes.is_empty()
    {
        let replace_creature_subtypes = !had_addition_tail && creature_subtypes_only(&subtypes);
        return BecomeSimpleDescriptorShape::Subtypes {
            subtypes,
            replace_creature_subtypes,
        };
    }
    BecomeSimpleDescriptorShape::None
}

pub(crate) fn parse_become_color_words(words: &[&str]) -> Option<ColorSet> {
    let mut colors = ColorSet::new();
    let mut saw_color = false;
    for word in words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        let color = leaf::parse_leaf_color_complete(word).ok()?;
        colors = colors.union(color);
        saw_color = true;
    }
    saw_color.then_some(colors)
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::front_end::lexer::lex_line;

    use super::*;

    #[test]
    fn parses_typed_simple_descriptors_and_animation_suffixes() {
        assert!(matches!(
            parse_become_simple_descriptor_words(&["blue", "zombie"]),
            BecomeSimpleDescriptorShape::ColorsAndSubtypes { .. }
        ));
        assert!(matches!(
            parse_become_simple_descriptor_words(&["bird"]),
            BecomeSimpleDescriptorShape::Subtypes {
                replace_creature_subtypes: true,
                ..
            }
        ));

        let tokens = lex_line("with flying and all creature types", 0).expect("lex fixture");
        assert!(matches!(
            parse_become_animation_suffix_shape(&tokens),
            BecomeAnimationSuffixShape::With {
                grants_all_creature_types: true,
                ..
            }
        ));
    }
}
