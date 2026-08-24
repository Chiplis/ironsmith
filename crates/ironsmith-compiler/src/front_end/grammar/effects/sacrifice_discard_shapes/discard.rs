use crate::color::ColorSet;
use crate::effect::Value;
use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use ironsmith_core::ValueSurfaceHint;
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{leaf, primitives};
use super::super::chain_splitting;
use super::common;

const HAND_REFERENCES: &[&[&str]] = &[
    &["hand"],
    &["your", "hand"],
    &["their", "hand"],
    &["that", "players", "hand"],
];
const TAGGED_REFERENCES: &[&[&str]] = &[&["it"], &["that", "card"], &["that", "token"]];
const EQUAL_COUNT_PREFIXES: &[&[&str]] = &[
    &["a", "number", "of", "cards", "equal", "to"],
    &["the", "number", "of", "cards", "equal", "to"],
    &["number", "of", "cards", "equal", "to"],
];
const SAME_MANA_VALUE_REFERENCES: &[&[&str]] = &[
    &["with", "that", "spells", "mana", "value"],
    &["with", "that", "spell's", "mana", "value"],
    &[
        "with", "the", "same", "mana", "value", "as", "that", "spell",
    ],
    &["with", "same", "mana", "value", "as", "that", "spell"],
];
const CHOSEN_COLOR_REFERENCES: &[&[&str]] = &[
    &["of", "that", "color"],
    &["that", "color"],
    &["of", "the", "chosen", "color"],
    &["the", "chosen", "color"],
    &["of", "chosen", "color"],
    &["chosen", "color"],
];

/// Parse a filter tail such as "of each of the sacrificed creature's colors."
/// Apostrophe normalization presents the possessive noun as `creatures`.
/// The returned value is presentation metadata; the caller supplies the
/// tagged color-sharing relation that carries runtime semantics.
pub fn parse_additional_cost_object_colors_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::AdditionalCostObjectSurface> {
    let words = parser_token_word_refs(tokens);
    let mut rest = words.as_slice();
    if let Some(after_prefix) = crate::word_primitives::strip_prefix(rest, &["of", "each", "of"]) {
        rest = after_prefix;
    } else if let Some(after_prefix) = crate::word_primitives::strip_prefix(rest, &["of"]) {
        rest = after_prefix;
    } else {
        return None;
    }
    if let Some(after_article) =
        crate::word_primitives::strip_any_prefix(rest, &[&["the"], &["a"], &["an"]])
            .map(|(_, tail)| tail)
    {
        rest = after_article;
    }
    let (action_word, noun_word, colors_word) = match rest {
        [action, noun, colors] => (*action, *noun, *colors),
        _ => return None,
    };
    if !matches!(colors_word, "color" | "colors") {
        return None;
    }
    let action = match action_word {
        "sacrificed" => ironsmith_core::AdditionalCostObjectAction::Sacrificed,
        "exiled" => ironsmith_core::AdditionalCostObjectAction::Exiled,
        _ => return None,
    };
    let kind = match noun_word {
        "creature" | "creatures" => ironsmith_core::SacrificedObjectKind::Creature,
        "artifact" | "artifacts" => ironsmith_core::SacrificedObjectKind::Artifact,
        "enchantment" | "enchantments" => ironsmith_core::SacrificedObjectKind::Enchantment,
        "permanent" | "permanents" => ironsmith_core::SacrificedObjectKind::Permanent,
        _ => return None,
    };
    Some(ironsmith_core::AdditionalCostObjectSurface::new(
        action, kind,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardShapeError {
    MissingCount,
    MissingCardKeyword,
}

#[derive(Debug, Clone)]
pub enum DiscardClauseShape<'a> {
    Hand,
    AllCardsInHand,
    TaggedOne,
    TaggedAll,
    EqualCount {
        count: Value,
        trailing_tokens: &'a [OwnedLexToken],
    },
    Cards(DiscardCardsShape<'a>),
}

#[derive(Debug, Clone)]
pub struct DiscardCardsShape<'a> {
    pub uses_all_count: bool,
    pub count: Value,
    pub any_number: bool,
    pub qualifier_tokens: &'a [OwnedLexToken],
    pub trailing_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardQualifierShape {
    EmptyOrThe,
    ChosenColor,
    Colors(ColorSet),
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardTrailingShape {
    Empty,
    Random,
    ChosenName,
    ChosenColor,
    SameManaValueAsTriggering,
    Colors(ColorSet),
    Other,
}

#[derive(Debug, Clone, Copy)]
pub struct DiscardAlternativeShape<'a> {
    pub discard_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub enum DiscardUnlessShape<'a> {
    None,
    MissingPredicate,
    Predicate(&'a [OwnedLexToken]),
}

fn discard_value_from_choice_count(count: crate::effect::ChoiceCount) -> Option<(Value, bool)> {
    if count.min == 1 && count.max.is_none() {
        return Some((
            Value::Fixed(0).with_surface_hint(ValueSurfaceHint::OneOrMoreChoice),
            true,
        ));
    }
    if count.is_any_number() {
        return Some((Value::Fixed(0), true));
    }
    if count.is_dynamic_x() {
        return Some((Value::X, false));
    }
    if count.min == 0
        && let Some(max) = count.max
    {
        // `up to N` is represented by the bounded value plus the optional
        // selection flag. The runtime uses the value as the maximum, while a
        // zero value continues to mean an unbounded "any number" choice.
        return Some((Value::Fixed(max as i32), true));
    }
    if count.min == count.max? {
        return Some((Value::Fixed(count.min as i32), false));
    }
    None
}

fn equal_count_shape(tokens: &[OwnedLexToken]) -> Option<DiscardClauseShape<'_>> {
    for prefix in EQUAL_COUNT_PREFIXES {
        let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::phrase(prefix).void())
        else {
            continue;
        };
        let (count, used) = crate::util::parse_value(rest)?;
        return Some(DiscardClauseShape::EqualCount {
            count: count.with_surface_hint(ValueSurfaceHint::EqualTo),
            trailing_tokens: rest.get(used..)?,
        });
    }
    None
}

fn is_full_hand_discard(words: &[&str]) -> bool {
    let Some(rest) = words.strip_prefix(&["all"]) else {
        return false;
    };
    let rest = rest.strip_prefix(&["the"]).unwrap_or(rest);
    let Some(rest) = rest
        .strip_prefix(&["cards"])
        .or_else(|| rest.strip_prefix(&["card"]))
    else {
        return false;
    };
    let rest = rest
        .strip_prefix(&["in"])
        .or_else(|| rest.strip_prefix(&["from"]))
        .unwrap_or(rest);
    common::exact_any(rest, HAND_REFERENCES)
}

pub fn parse_discard_clause_shape(
    tokens: &[OwnedLexToken],
) -> Result<DiscardClauseShape<'_>, DiscardShapeError> {
    let words = parser_token_word_refs(tokens);
    if common::exact_any(&words, HAND_REFERENCES) {
        return Ok(DiscardClauseShape::Hand);
    }
    if is_full_hand_discard(&words) {
        return Ok(DiscardClauseShape::AllCardsInHand);
    }
    if common::exact_any(&words, TAGGED_REFERENCES) {
        return Ok(DiscardClauseShape::TaggedOne);
    }
    if common::exact(&words, &["those", "cards"]) {
        return Ok(DiscardClauseShape::TaggedAll);
    }
    if let Some(shape) = equal_count_shape(tokens) {
        return Ok(shape);
    }

    let (uses_all_count, count, any_number, used) =
        if let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::kw("all").void()) {
            (true, Value::Fixed(0), false, tokens.len() - rest.len())
        } else if let Some((choice_count, used)) =
            crate::util::parse_choice_count_token_prefix_consumed(tokens)
            && let Some((count, any_number)) = discard_value_from_choice_count(choice_count)
        {
            (false, count, any_number, used)
        } else if let Some((count, used)) = crate::util::parse_value(tokens) {
            (false, count, false, used)
        } else {
            return Err(DiscardShapeError::MissingCount);
        };

    let rest = tokens
        .get(used..)
        .ok_or(DiscardShapeError::MissingCardKeyword)?;
    let Some((card_offset, _, trailing_tokens)) = primitives::find_prefix(rest, || {
        alt((
            primitives::kw("card").void(),
            primitives::kw("cards").void(),
        ))
    }) else {
        return Err(DiscardShapeError::MissingCardKeyword);
    };
    Ok(DiscardClauseShape::Cards(DiscardCardsShape {
        uses_all_count,
        count,
        any_number,
        qualifier_tokens: &rest[..card_offset],
        trailing_tokens,
    }))
}

fn non_article_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
        .into_iter()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect()
}

fn chosen_color_reference(tokens: &[OwnedLexToken]) -> bool {
    let raw_words = parser_token_word_refs(tokens);
    let words = non_article_words(tokens);
    common::exact_any(&raw_words, CHOSEN_COLOR_REFERENCES)
        || common::exact_any(&words, CHOSEN_COLOR_REFERENCES)
}

fn color_set(tokens: &[OwnedLexToken]) -> Option<ColorSet> {
    let words = non_article_words(tokens);
    if words.is_empty() {
        return None;
    }
    let mut colors = ColorSet::new();
    let mut saw_color = false;
    for word in words {
        if word == "or" {
            continue;
        }
        let color = leaf::parse_leaf_color_complete(word).ok()?;
        colors = colors.union(color);
        saw_color = true;
    }
    saw_color.then_some(colors)
}

pub fn parse_discard_qualifier_shape(tokens: &[OwnedLexToken]) -> DiscardQualifierShape {
    let words = parser_token_word_refs(tokens);
    if words.is_empty() || common::exact(&words, &["the"]) {
        DiscardQualifierShape::EmptyOrThe
    } else if chosen_color_reference(tokens) {
        DiscardQualifierShape::ChosenColor
    } else if let Some(colors) = color_set(tokens) {
        DiscardQualifierShape::Colors(colors)
    } else {
        DiscardQualifierShape::Other
    }
}

pub fn parse_discard_trailing_shape(tokens: &[OwnedLexToken]) -> DiscardTrailingShape {
    let words = parser_token_word_refs(tokens);
    if words.is_empty() {
        DiscardTrailingShape::Empty
    } else if common::exact(&words, &["at", "random"]) {
        DiscardTrailingShape::Random
    } else if common::exact(&words, &["with", "that", "name"]) {
        DiscardTrailingShape::ChosenName
    } else if chosen_color_reference(tokens) {
        DiscardTrailingShape::ChosenColor
    } else if common::exact_any(&words, SAME_MANA_VALUE_REFERENCES) {
        DiscardTrailingShape::SameManaValueAsTriggering
    } else if let Some(colors) = color_set(tokens) {
        DiscardTrailingShape::Colors(colors)
    } else {
        DiscardTrailingShape::Other
    }
}

pub fn parse_discard_alternative_shape(
    tokens: &[OwnedLexToken],
) -> Option<DiscardAlternativeShape<'_>> {
    let mut search_tokens = tokens;
    let mut search_offset = 0usize;
    while let Some((relative_offset, _, after_or)) =
        primitives::find_prefix(search_tokens, || primitives::kw("or").void())
    {
        let marker_offset = search_offset + relative_offset;
        let alternative_tokens = crate::util::trim_edge_punctuation_tokens(after_or);
        let starts_new_action = chain_splitting::find_chain_verb_tokens(alternative_tokens)
            .is_some_and(|found| found.word_index == 0)
            || chain_splitting::has_extended_effect_head_tokens(alternative_tokens);
        if !alternative_tokens.is_empty() && starts_new_action {
            return Some(DiscardAlternativeShape {
                discard_tokens: &tokens[..marker_offset],
            });
        }

        let consumed = search_tokens.len().saturating_sub(after_or.len());
        search_offset += consumed;
        search_tokens = after_or;
    }
    None
}

pub fn parse_discard_unless_shape(tokens: &[OwnedLexToken]) -> DiscardUnlessShape<'_> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some((_, predicate_tokens)) =
        primitives::parse_prefix(tokens, primitives::kw("unless").void())
    else {
        return DiscardUnlessShape::None;
    };
    let predicate_tokens = crate::util::trim_edge_punctuation_tokens(predicate_tokens);
    if predicate_tokens.is_empty() {
        DiscardUnlessShape::MissingPredicate
    } else {
        DiscardUnlessShape::Predicate(predicate_tokens)
    }
}

#[cfg(test)]
#[path = "discard_inline_tests.rs"]
mod tests;
