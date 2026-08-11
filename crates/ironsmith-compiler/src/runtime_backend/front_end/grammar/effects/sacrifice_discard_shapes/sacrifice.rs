use crate::mana::ManaSymbol;
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, parser_token_word_refs};
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
pub(crate) enum SacrificeUnlessKind {
    None,
    Escaped,
    ManaSpent(ManaSymbol),
    OpponentDamagedThisTurn,
    General,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SacrificeClauseShape<'a> {
    pub(crate) body_tokens: &'a [OwnedLexToken],
    pub(crate) full_body_tokens: &'a [OwnedLexToken],
    pub(crate) unless_token_offset: Option<usize>,
    pub(crate) unless_kind: SacrificeUnlessKind,
    pub(crate) sacrifice_references_it: bool,
    pub(crate) has_graveyard_history: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SacrificeQuantityShape<'a> {
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
pub(crate) enum SacrificeAggregateKind {
    GreatestManaValue,
    GreatestPower,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SacrificeAggregateShape<'a> {
    pub(crate) kind: SacrificeAggregateKind,
    pub(crate) object_tokens: &'a [OwnedLexToken],
    pub(crate) among_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SacrificeTaggedReferenceKind {
    ItOrCard,
    Token,
    OneOfTaggedSet,
    AllOfTaggedSet,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SacrificeObjectShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) tagged_reference: Option<SacrificeTaggedReferenceKind>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SacrificeCountShape<'a> {
    pub(crate) count: u32,
    pub(crate) other: bool,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SacrificeFractionRoundedShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) denominator: u32,
    pub(crate) rounded_up: bool,
}

pub(crate) fn parse_sacrifice_mana_spent_symbol(tokens: &[OwnedLexToken]) -> Option<ManaSymbol> {
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

pub(crate) fn parse_sacrifice_clause_shape(tokens: &[OwnedLexToken]) -> SacrificeClauseShape<'_> {
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

pub(crate) fn parse_sacrifice_quantity_shape(
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

pub(crate) fn parse_sacrifice_fraction_rounded_shape(
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

pub(crate) fn parse_sacrifice_count_shape(tokens: &[OwnedLexToken]) -> SacrificeCountShape<'_> {
    let mut count = 1u32;
    let mut rest = tokens;
    // `one of them` names one member of the previously established object
    // set; its `one` is not an ordinary count prefix. Keep the complete
    // phrase intact so `parse_sacrifice_object_shape` can preserve the set
    // choice semantics instead of degrading it to a bare `them` reference.
    let one_of_tagged_set =
        primitives::parse_prefix(rest, primitives::phrase(&["one", "of", "them"]).void()).is_some();
    if !one_of_tagged_set
        && let Some(prefix) = leaf::parse_leaf_number_prefix_tokens(rest)
        && let Some((value, used)) = prefix.into_fixed()
    {
        count = value;
        rest = &rest[used..];
    }

    let mut other = false;
    if let Some((_, after_another)) =
        primitives::parse_prefix(rest, primitives::kw("another").void())
    {
        other = true;
        rest = after_another;
    }

    if !one_of_tagged_set
        && count == 1
        && let Some(prefix) = leaf::parse_leaf_number_prefix_tokens(rest)
        && let Some((value, used)) = prefix.into_fixed()
    {
        count = value;
        rest = &rest[used..];
    }

    SacrificeCountShape {
        count,
        other,
        filter_tokens: rest,
    }
}

pub(crate) fn parse_sacrifice_aggregate_shape(
    tokens: &[OwnedLexToken],
) -> Option<SacrificeAggregateShape<'_>> {
    let (marker_offset, kind, among_tokens) = primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["with", "the", "greatest", "mana", "value", "among"])
                .value(SacrificeAggregateKind::GreatestManaValue),
            primitives::phrase(&["with", "the", "greatest", "power", "among"])
                .value(SacrificeAggregateKind::GreatestPower),
        ))
    })?;
    Some(SacrificeAggregateShape {
        kind,
        object_tokens: &tokens[..marker_offset],
        among_tokens,
    })
}

pub(crate) fn parse_sacrifice_object_shape(tokens: &[OwnedLexToken]) -> SacrificeObjectShape<'_> {
    let filter_tokens = primitives::strip_lexed_suffix_phrases(tokens, CHOICE_SUFFIXES)
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    let words = parser_token_word_refs(filter_tokens);
    let tagged_reference = if common::exact(&words, &["that", "token"]) {
        Some(SacrificeTaggedReferenceKind::Token)
    } else if common::exact_any(&words, ONE_OF_TAGGED_SET_REFERENCES) {
        Some(SacrificeTaggedReferenceKind::OneOfTaggedSet)
    } else if common::exact_any(&words, ALL_OF_TAGGED_SET_REFERENCES) {
        Some(SacrificeTaggedReferenceKind::AllOfTaggedSet)
    } else if common::exact_any(&words, TAGGED_REFERENCES) {
        Some(SacrificeTaggedReferenceKind::ItOrCard)
    } else {
        None
    };
    SacrificeObjectShape {
        filter_tokens,
        tagged_reference,
    }
}

pub(crate) fn parse_sacrifice_attached_exclusion(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    ATTACHED_EXCLUSIONS
        .iter()
        .any(|phrase| common::present(&words, phrase))
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::front_end::lexer::{lex_line, parser_token_word_refs};

    use super::*;

    #[test]
    fn sacrifice_clause_classifies_unless_and_strips_verb() {
        let tokens = lex_line("Sacrifice that token, unless {R} was spent to cast it", 0).unwrap();
        let shape = parse_sacrifice_clause_shape(&tokens);
        assert_eq!(parser_token_word_refs(shape.body_tokens), ["that", "token"]);
        assert_eq!(
            shape.unless_kind,
            SacrificeUnlessKind::ManaSpent(ManaSymbol::Red)
        );
        assert!(shape.sacrifice_references_it);
        assert_eq!(
            shape
                .full_body_tokens
                .get(shape.unless_token_offset.unwrap())
                .and_then(OwnedLexToken::as_word),
            Some("unless")
        );
    }

    #[test]
    fn sacrifice_object_shape_strips_choice_and_preserves_token_reference() {
        let tokens = lex_line("that token of their choice", 0).unwrap();
        let shape = parse_sacrifice_object_shape(&tokens);
        assert_eq!(
            shape.tagged_reference,
            Some(SacrificeTaggedReferenceKind::Token)
        );
        assert_eq!(
            parser_token_word_refs(shape.filter_tokens),
            ["that", "token"]
        );
    }

    #[test]
    fn sacrifice_all_or_each_preserves_only_the_authored_each_surface() {
        for (text, expected_each) in [
            ("each other creature you control", true),
            ("all other creatures you control", false),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let Some(SacrificeQuantityShape::AllOrEach {
                each_surface,
                other,
                ..
            }) = parse_sacrifice_quantity_shape(&tokens)
            else {
                panic!("expected all/each sacrifice quantity: {text}");
            };
            assert_eq!(each_surface, expected_each, "{text}");
            assert!(other, "{text}");
        }
    }

    #[test]
    fn sacrifice_object_shape_preserves_definite_object_references() {
        for text in [
            "that creature",
            "the creature",
            "that permanent",
            "the permanent",
            "that token",
            "the token",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let shape = parse_sacrifice_object_shape(&tokens);
            assert!(
                shape.tagged_reference.is_some(),
                "{text} should remain a reference to the established object"
            );
        }
    }

    #[test]
    fn sacrifice_object_shape_distinguishes_one_member_of_tagged_set() {
        let tokens = lex_line("one of them", 0).unwrap();
        let shape = parse_sacrifice_object_shape(&tokens);

        assert_eq!(
            shape.tagged_reference,
            Some(SacrificeTaggedReferenceKind::OneOfTaggedSet)
        );
        assert_eq!(
            parser_token_word_refs(shape.filter_tokens),
            ["one", "of", "them"]
        );
    }

    #[test]
    fn sacrifice_object_shape_preserves_all_of_plural_result_set() {
        for text in ["those permanents", "those creatures", "those tokens"] {
            let tokens = lex_line(text, 0).unwrap();
            let shape = parse_sacrifice_object_shape(&tokens);

            assert_eq!(
                shape.tagged_reference,
                Some(SacrificeTaggedReferenceKind::AllOfTaggedSet),
                "{text}"
            );
            assert_eq!(
                parser_token_word_refs(shape.filter_tokens),
                text.split_whitespace().collect::<Vec<_>>(),
                "{text}"
            );
        }
    }

    #[test]
    fn aggregate_shape_returns_typed_axis_and_sides() {
        let tokens = lex_line(
            "a creature with the greatest power among creatures you control",
            0,
        )
        .unwrap();
        let shape = parse_sacrifice_aggregate_shape(&tokens).unwrap();
        assert_eq!(shape.kind, SacrificeAggregateKind::GreatestPower);
        assert_eq!(
            parser_token_word_refs(shape.object_tokens),
            ["a", "creature"]
        );
        assert_eq!(
            parser_token_word_refs(shape.among_tokens),
            ["creatures", "you", "control"]
        );
    }

    #[test]
    fn sacrifice_count_shape_returns_count_other_and_filter() {
        let tokens = lex_line("two another creatures", 0).unwrap();
        let shape = parse_sacrifice_count_shape(&tokens);
        assert_eq!(shape.count, 2);
        assert!(shape.other);
        assert_eq!(parser_token_word_refs(shape.filter_tokens), ["creatures"]);
    }

    #[test]
    fn sacrifice_fraction_rounded_shape_preserves_denominator_and_controlled_filter() {
        let tokens = lex_line(
            "half the creatures they control of their choice, rounded up",
            0,
        )
        .unwrap();
        let shape = parse_sacrifice_fraction_rounded_shape(&tokens).unwrap();
        assert_eq!(shape.denominator, 2);
        assert_eq!(
            parser_token_word_refs(shape.filter_tokens),
            ["creatures", "they", "control"]
        );

        let tokens = lex_line(
            "a tenth of the creatures they control of their choice, rounded up",
            0,
        )
        .unwrap();
        let shape = parse_sacrifice_fraction_rounded_shape(&tokens).unwrap();
        assert_eq!(shape.denominator, 10);
        assert!(shape.rounded_up);
        assert_eq!(
            parser_token_word_refs(shape.filter_tokens),
            ["creatures", "they", "control"]
        );
    }

    #[test]
    fn sacrifice_fraction_shape_requires_a_rounding_surface_and_valid_unit_fraction() {
        for text in [
            "a tenth of the creatures they control of their choice",
            "a first of the creatures they control of their choice, rounded up",
            "a tenth creatures they control of their choice, rounded up",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(
                parse_sacrifice_fraction_rounded_shape(&tokens).is_none(),
                "near miss must not claim {text:?}"
            );
        }
    }

    #[test]
    fn sacrifice_all_except_shape_preserves_filter_and_keep_count() {
        let tokens = lex_line("all lands they control except for three", 0).unwrap();
        let Some(SacrificeQuantityShape::AllExcept {
            filter_tokens,
            keep_count,
            other,
        }) = parse_sacrifice_quantity_shape(&tokens)
        else {
            panic!("expected typed all-except quantity");
        };
        assert_eq!(keep_count, 3);
        assert!(!other);
        assert_eq!(
            parser_token_word_refs(filter_tokens),
            ["lands", "they", "control"]
        );

        for text in [
            "all lands they control except for zero",
            "all lands they control except for",
            "lands they control except for three",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(
                !matches!(
                    parse_sacrifice_quantity_shape(&tokens),
                    Some(SacrificeQuantityShape::AllExcept { .. })
                ),
                "near miss must not claim {text:?}"
            );
        }
    }
}
