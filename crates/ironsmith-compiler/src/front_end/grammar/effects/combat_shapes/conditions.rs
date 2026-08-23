use crate::filter::PowerToughnessRelation;
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, trim_lexed_commas};
use crate::util::{comparison_to_strict_at_least_threshold, parse_quantity_comparison_prefix};

const CONTROL_PREFIXES: &[&[&str]] = &[&["you", "control"], &["you", "controlled"]];
const CAST_OR_TURN_MARKERS: &[&[&str]] =
    &[&["as", "you", "cast", "this", "spell"], &["this", "turn"]];
const DIFFERENT_POWER_SUFFIXES: &[&[&str]] = &[
    &["with", "different", "powers"],
    &["with", "different", "power"],
];
const TOUGHNESS_GREATER_SUFFIXES: &[&[&str]] = &[
    &[
        "that",
        "each",
        "have",
        "toughness",
        "greater",
        "than",
        "their",
        "power",
    ],
    &[
        "that",
        "each",
        "has",
        "toughness",
        "greater",
        "than",
        "its",
        "power",
    ],
    &["with", "toughness", "greater", "than", "their", "power"],
    &["with", "toughness", "greater", "than", "its", "power"],
    &["with", "power", "less", "than", "their", "toughness"],
    &["with", "power", "less", "than", "its", "toughness"],
];
const POWER_GREATER_SUFFIXES: &[&[&str]] = &[
    &[
        "that",
        "each",
        "have",
        "power",
        "greater",
        "than",
        "their",
        "toughness",
    ],
    &[
        "that",
        "each",
        "has",
        "power",
        "greater",
        "than",
        "its",
        "toughness",
    ],
    &["with", "power", "greater", "than", "their", "toughness"],
    &["with", "power", "greater", "than", "its", "toughness"],
    &["with", "toughness", "less", "than", "their", "power"],
    &["with", "toughness", "less", "than", "its", "power"],
];

#[derive(Debug, Clone, Copy)]
pub struct CombatControlPredicateShape<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub min_count: Option<u32>,
    pub requires_different_powers: bool,
    pub power_toughness_relation: Option<PowerToughnessRelation>,
    pub other: bool,
}

fn strip_relation_suffix(
    tokens: &[OwnedLexToken],
) -> Option<(PowerToughnessRelation, &[OwnedLexToken])> {
    if let Some((_suffix, rest)) =
        primitives::strip_lexed_suffix_phrases(tokens, TOUGHNESS_GREATER_SUFFIXES)
    {
        Some((PowerToughnessRelation::ToughnessGreaterThanPower, rest))
    } else {
        primitives::strip_lexed_suffix_phrases(tokens, POWER_GREATER_SUFFIXES)
            .map(|(_suffix, rest)| (PowerToughnessRelation::PowerGreaterThanToughness, rest))
    }
}

pub fn parse_combat_control_predicate_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatControlPredicateShape<'_>> {
    let (_prefix, mut filter_tokens) =
        primitives::parse_prefix(tokens, primitives::any_phrase(CONTROL_PREFIXES))?;
    let mut min_count = None;
    if let Ok((comparison, used)) =
        parse_quantity_comparison_prefix(filter_tokens, false, false, "control predicate")
    {
        min_count = comparison_to_strict_at_least_threshold(&comparison);
        if min_count.is_some()
            || matches!(
                comparison,
                crate::effect::Comparison::LessThan(_)
                    | crate::effect::Comparison::LessThanOrEqual(_)
            )
        {
            filter_tokens = filter_tokens.get(used..)?;
        }
    }
    if let Some((idx, _marker, _after)) = primitives::find_prefix(filter_tokens, || {
        primitives::any_phrase(CAST_OR_TURN_MARKERS)
    }) {
        filter_tokens = &filter_tokens[..idx];
    }
    filter_tokens = trim_lexed_commas(filter_tokens);

    let mut power_toughness_relation = None;
    if let Some((relation, rest)) = strip_relation_suffix(filter_tokens) {
        power_toughness_relation = Some(relation);
        filter_tokens = trim_lexed_commas(rest);
    }
    let mut requires_different_powers = false;
    if let Some((_suffix, rest)) =
        primitives::strip_lexed_suffix_phrases(filter_tokens, DIFFERENT_POWER_SUFFIXES)
    {
        requires_different_powers = true;
        filter_tokens = trim_lexed_commas(rest);
    }
    if filter_tokens.is_empty() {
        return None;
    }
    let other = primitives::parse_prefix(
        filter_tokens,
        primitives::any_phrase(&[&["another"], &["other"]]),
    )
    .is_some();
    Some(CombatControlPredicateShape {
        filter_tokens,
        min_count,
        requires_different_powers,
        power_toughness_relation,
        other,
    })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{lex_line, parser_token_word_refs};

    use super::*;

    #[test]
    fn parses_control_predicate_suffixes() {
        let tokens = lex_line(
            "you control three or more creatures with different powers this turn",
            0,
        )
        .unwrap();
        let shape = parse_combat_control_predicate_shape_lexed(&tokens).unwrap();
        assert_eq!(shape.min_count, Some(3));
        assert!(shape.requires_different_powers);
        assert_eq!(parser_token_word_refs(shape.filter_tokens), ["creatures"]);
    }
}
