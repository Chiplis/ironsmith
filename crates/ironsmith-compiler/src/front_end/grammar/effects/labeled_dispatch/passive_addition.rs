use crate::color::ColorSet;
use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::types::{CardType, Subtype};
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::super::{leaf, primitives};
use super::common;

const COLOR_AND_TYPE_TAILS: &[&[&str]] = &[
    &[
        "in", "addition", "to", "its", "other", "colors", "and", "types",
    ],
    &[
        "in", "addition", "to", "their", "other", "colors", "and", "types",
    ],
    &[
        "in", "addition", "to", "its", "other", "colors", "and", "creature", "types",
    ],
    &[
        "in", "addition", "to", "their", "other", "colors", "and", "creature", "types",
    ],
];
const TYPE_TAILS: &[&[&str]] = &[
    &["in", "addition", "to", "its", "other", "types"],
    &["in", "addition", "to", "their", "other", "types"],
    &["in", "addition", "to", "its", "other", "creature", "types"],
    &[
        "in", "addition", "to", "their", "other", "creature", "types",
    ],
];
const TAGGED_SUBJECTS: &[&[&str]] = &[
    &["it"],
    &["they"],
    &["them"],
    &["that"],
    &["that", "card"],
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "object"],
    &["those"],
    &["those", "cards"],
    &["those", "creatures"],
    &["those", "permanents"],
    &["those", "objects"],
    &["each", "of", "them"],
    &["each", "of", "those"],
    &["each", "of", "those", "cards"],
    &["each", "of", "those", "creatures"],
    &["each", "of", "those", "permanents"],
    &["each", "of", "those", "objects"],
    &["all", "of", "them"],
    &["all", "of", "those"],
    &["all", "of", "those", "cards"],
    &["all", "of", "those", "creatures"],
    &["all", "of", "those", "permanents"],
    &["all", "of", "those", "objects"],
];

#[derive(Debug, Clone)]
pub(crate) struct PassiveColorTypeAdditionShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) tagged_subject: bool,
    pub(crate) colors: ColorSet,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) adds_colors: bool,
}

fn strip_addition_tail(tokens: &[OwnedLexToken]) -> Option<(bool, &[OwnedLexToken])> {
    if let Some((_, rest)) = primitives::strip_lexed_suffix_phrases(tokens, COLOR_AND_TYPE_TAILS) {
        return Some((true, rest));
    }
    primitives::strip_lexed_suffix_phrases(tokens, TYPE_TAILS).map(|(_, rest)| (false, rest))
}

pub(crate) fn parse_passive_color_type_addition_shape(
    tokens: &[OwnedLexToken],
) -> Option<PassiveColorTypeAdditionShape<'_>> {
    let (adds_colors, body_tokens) = strip_addition_tail(tokens)?;
    let (is_offset, _, descriptor_tokens) = primitives::find_prefix(body_tokens, || {
        alt((primitives::kw("is").void(), primitives::kw("are").void()))
    })?;
    let subject_tokens = &body_tokens[..is_offset];
    let descriptor_words = parser_token_word_refs(descriptor_tokens);
    if subject_tokens.is_empty() || descriptor_words.is_empty() {
        return None;
    }
    // Compound static predicates have dedicated typed parsers. Absorbing an
    // earlier modifier into this subject turns semantics such as base P/T or
    // an anthem into an accidental object-filter constraint.
    if parser_token_word_refs(subject_tokens)
        .iter()
        .any(|word| matches!(*word, "get" | "gets" | "has" | "have"))
    {
        return None;
    }

    let mut colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for word in descriptor_words {
        if word == "and" || leaf::parse_leaf_article_complete(word).is_ok() {
            continue;
        }
        if let Ok(color) = leaf::parse_leaf_color_complete(word) {
            colors = colors.union(color);
            continue;
        }
        if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
            if !card_types.contains(&card_type) {
                card_types.push(card_type);
            }
            continue;
        }
        if let Ok(subtype) = leaf::parse_leaf_subtype_complete(word) {
            if !subtypes.contains(&subtype) {
                subtypes.push(subtype);
            }
            continue;
        }
        return None;
    }

    let subject_words = parser_token_word_refs(subject_tokens);
    Some(PassiveColorTypeAdditionShape {
        subject_tokens,
        tagged_subject: common::exact_any(&subject_words, TAGGED_SUBJECTS),
        colors,
        card_types,
        subtypes,
        adds_colors,
    })
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex_line;

    use super::*;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_typed_passive_color_and_type_additions() {
        let tokens =
            lex("It is a blue Zombie artifact in addition to its other colors and creature types.");
        let shape = parse_passive_color_type_addition_shape(&tokens).expect("passive shape");
        assert!(shape.tagged_subject);
        assert!(shape.adds_colors);
        assert_eq!(shape.colors, ColorSet::BLUE);
        assert_eq!(shape.card_types, [CardType::Artifact]);
        assert_eq!(shape.subtypes, [Subtype::Zombie]);
    }
}
