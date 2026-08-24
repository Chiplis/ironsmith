use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::cards::builders::CardTextError;
use crate::effect::ChoiceCount;
use crate::target::ObjectFilter;
use crate::types::{CardType, Supertype};
use crate::zone::Zone;

use super::super::super::lexer::{LexStream, OwnedLexToken, render_token_slice};
use super::super::{filters, leaf, primitives};
use super::ActivationCostSegmentCst;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SacrificeChosenShape {
    count: ChoiceCount,
    other: bool,
    filter_first: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SacrificeCostShape {
    Source,
    Creature,
    All { filter_first: usize },
    MissingFilter,
    Chosen(SacrificeChosenShape),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnattachChosenShape<'a> {
    count: u32,
    filter_tokens: &'a [OwnedLexToken],
    source_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnattachCostShape<'a> {
    Source {
        reference_tokens: &'a [OwnedLexToken],
    },
    Chosen(UnattachChosenShape<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TapChosenShape<'a> {
    count: u32,
    other: bool,
    filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiscardCostShape {
    Source,
    Hand,
    Named {
        count: u32,
        other: bool,
        name_first: usize,
    },
    Disjunction {
        count: u32,
        other: bool,
        left_first: usize,
        left_end: usize,
        right_first: usize,
    },
    Cards {
        count: u32,
        other: bool,
        card_types: Vec<CardType>,
        supertypes: Vec<Supertype>,
        random: bool,
    },
}

pub fn parse_sacrifice_segment_tokens(
    tokens: &[OwnedLexToken],
    contextual_source_reference: impl Fn(&[&str]) -> Option<crate::target::SourceReferenceSurface>,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let words = primitives::TokenWordView::new(tokens);
    if words.len() > 1
        && let Some(surface) = contextual_source_reference(&words.word_refs()[1..])
    {
        return Ok(ActivationCostSegmentCst::SacrificeSelf {
            surface: Some(surface),
        });
    }
    let shape = primitives::parse_all(tokens, parse_sacrifice_cost_shape_lexed, "sacrifice-cost")
        .map_err(|_| unsupported(tokens, "sacrifice"))?;
    Ok(match shape {
        SacrificeCostShape::Source => ActivationCostSegmentCst::SacrificeSelf { surface: None },
        SacrificeCostShape::Creature => ActivationCostSegmentCst::SacrificeCreature,
        SacrificeCostShape::All { filter_first } => ActivationCostSegmentCst::SacrificeAll {
            filter: filters::parse_object_filter_with_grammar_entrypoint_lexed(
                &tokens[filter_first..],
                false,
            )?,
        },
        SacrificeCostShape::MissingFilter => {
            return Err(CardTextError::ParseError(
                "rewrite sacrifice parser is missing an object filter".to_string(),
            ));
        }
        SacrificeCostShape::Chosen(shape) => ActivationCostSegmentCst::SacrificeChosen {
            count: shape.count,
            filter: filters::parse_object_filter_with_grammar_entrypoint_lexed(
                &tokens[shape.filter_first..],
                shape.other,
            )?,
        },
    })
}

pub fn parse_discard_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let shape = primitives::parse_all(tokens, parse_discard_cost_shape_lexed, "discard-cost")
        .map_err(|_| unsupported(tokens, "discard"))?;
    Ok(match shape {
        DiscardCostShape::Source => ActivationCostSegmentCst::DiscardSource,
        DiscardCostShape::Hand => ActivationCostSegmentCst::DiscardHand,
        DiscardCostShape::Named {
            count,
            other,
            name_first,
        } => ActivationCostSegmentCst::DiscardFiltered {
            count,
            card_types: Vec::new(),
            supertypes: Vec::new(),
            filter: None,
            random: false,
            name: Some(
                render_token_slice(&tokens[name_first..])
                    .trim()
                    .to_ascii_lowercase(),
            ),
            other,
        },
        DiscardCostShape::Disjunction {
            count,
            other,
            left_first,
            left_end,
            right_first,
        } => {
            let left = super::super::filters::parse_object_filter_with_grammar_entrypoint_lexed(
                &tokens[left_first..left_end],
                false,
            )?;
            let right = super::super::filters::parse_object_filter_with_grammar_entrypoint_lexed(
                &tokens[right_first..],
                false,
            )?;
            let mut filter = ObjectFilter {
                zone: Some(Zone::Hand),
                ..ObjectFilter::default()
            };
            filter.any_of = vec![left, right];
            ActivationCostSegmentCst::DiscardFiltered {
                count,
                card_types: Vec::new(),
                supertypes: Vec::new(),
                filter: Some(filter),
                random: false,
                name: None,
                other,
            }
        }
        DiscardCostShape::Cards {
            count,
            card_types,
            supertypes,
            random,
            ..
        } if card_types.is_empty() && supertypes.is_empty() && !random => {
            ActivationCostSegmentCst::DiscardCard(count)
        }
        DiscardCostShape::Cards {
            count,
            other,
            card_types,
            supertypes,
            random,
        } => ActivationCostSegmentCst::DiscardFiltered {
            count,
            card_types,
            supertypes,
            filter: None,
            random,
            name: None,
            other,
        },
    })
}

pub fn parse_unattach_segment_tokens(
    tokens: &[OwnedLexToken],
    contextual_source_reference: impl FnOnce(&[&str]) -> bool,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let shape = primitives::parse_all(tokens, parse_unattach_cost_shape_lexed, "unattach-cost")
        .map_err(|_| unsupported(tokens, "unattach"))?;
    match shape {
        UnattachCostShape::Source { reference_tokens } => {
            let source_words = primitives::TokenWordView::new(reference_tokens).word_refs();
            if !contextual_source_reference(&source_words) {
                return Err(unsupported(tokens, "unattach"));
            }
            Ok(ActivationCostSegmentCst::UnattachChosen {
                count: 1,
                filter: ObjectFilter::source(),
            })
        }
        UnattachCostShape::Chosen(shape) => {
            let source_words = primitives::TokenWordView::new(shape.source_tokens).word_refs();
            if !contextual_source_reference(&source_words) {
                return Err(CardTextError::ParseError(format!(
                    "rewrite unattach parser only supports unattach-from-source costs in '{}'",
                    render_token_slice(tokens).trim().to_ascii_lowercase()
                )));
            }
            let mut filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
                shape.filter_tokens,
                false,
            )?;
            // Equipment is an artifact permanent even when the surface uses only the
            // subtype noun.  Unattach costs necessarily select it on the battlefield.
            if filter
                .subtypes
                .iter()
                .any(|subtype| subtype == &crate::types::Subtype::Equipment)
            {
                if !filter
                    .card_types
                    .iter()
                    .any(|card_type| card_type == &CardType::Artifact)
                {
                    filter.card_types.push(CardType::Artifact);
                }
                filter.zone.get_or_insert(Zone::Battlefield);
            }
            Ok(ActivationCostSegmentCst::UnattachChosen {
                count: shape.count,
                filter,
            })
        }
    }
}

pub fn parse_tap_chosen_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let shape = primitives::parse_all(tokens, parse_tap_chosen_shape_lexed, "tap-chosen-cost")
        .map_err(|_| unsupported(tokens, "tap chosen"))?;
    let (filter_tokens, exclude_declared_combatants) =
        strip_not_declared_as_attacking_or_blocking_suffix(shape.filter_tokens);
    let mut filter =
        filters::parse_object_filter_with_grammar_entrypoint_lexed(filter_tokens, shape.other)?;
    filter.untapped = true;
    if exclude_declared_combatants {
        filter.nonattacking = true;
        filter.nonblocking = true;
    }
    Ok(ActivationCostSegmentCst::TapChosen {
        count: shape.count,
        filter,
    })
}

fn strip_not_declared_as_attacking_or_blocking_suffix(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], bool) {
    const SUFFIXES: &[&[&str]] = &[
        &[
            "not",
            "declared",
            "as",
            "an",
            "attacking",
            "or",
            "blocking",
            "creature",
            "this",
            "combat",
        ],
        &[
            "not",
            "declared",
            "as",
            "attacking",
            "or",
            "blocking",
            "this",
            "combat",
        ],
    ];

    for start in 0..tokens.len() {
        let suffix_words = primitives::TokenWordView::new(&tokens[start..]).word_refs();
        if crate::word_primitives::parse_any_sequence_complete(&suffix_words, SUFFIXES) {
            let filter_tokens = &tokens[..start];
            if !filter_tokens.is_empty() {
                return (filter_tokens, true);
            }
        }
    }
    (tokens, false)
}

fn unsupported(tokens: &[OwnedLexToken], label: &str) -> CardTextError {
    CardTextError::ParseError(format!(
        "rewrite {label} parser does not yet support '{}'",
        render_token_slice(tokens).trim().to_ascii_lowercase()
    ))
}

fn parse_discard_cost_shape_lexed<'a>(input: &mut LexStream<'a>) -> WResult<DiscardCostShape> {
    let initial_len = input.len();
    primitives::kw("discard").parse_next(input)?;
    alt((
        parse_discard_hand,
        parse_discard_source,
        move |input: &mut LexStream<'a>| parse_discard_selected(input, initial_len),
    ))
    .parse_next(input)
}

fn parse_discard_hand<'a>(input: &mut LexStream<'a>) -> WResult<DiscardCostShape> {
    primitives::phrase(&["your", "hand"]).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(DiscardCostShape::Hand)
}

fn parse_discard_source<'a>(input: &mut LexStream<'a>) -> WResult<DiscardCostShape> {
    primitives::phrase(&["this", "card"]).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(DiscardCostShape::Source)
}

fn parse_discard_selected<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<DiscardCostShape> {
    let count = parse_optional_discard_count(input);
    let other = alt((primitives::kw("other"), primitives::kw("another")))
        .parse_next(input)
        .is_ok();
    parse_indefinite_articles(input);

    let mut named = input.clone();
    if primitives::phrase(&["card", "named"])
        .parse_next(&mut named)
        .is_ok()
    {
        let name_first = initial_len.saturating_sub(named.len());
        let name: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(&mut named)?;
        if name.is_empty() {
            return Err(primitives::backtrack_err("discard name", "card name"));
        }
        *input = named;
        return Ok(DiscardCostShape::Named {
            count,
            other,
            name_first,
        });
    }

    let mut disjunction = input.clone();
    if let Ok((left_first, left_end, right_first)) =
        parse_discard_disjunction(&mut disjunction, initial_len)
    {
        *input = disjunction;
        return Ok(DiscardCostShape::Disjunction {
            count,
            other,
            left_first,
            left_end,
            right_first,
        });
    }

    let (card_types, supertypes) = parse_discard_type_descriptors(input)?;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    let random = primitives::phrase(&["at", "random"])
        .parse_next(input)
        .is_ok();
    eof.parse_next(input)?;
    Ok(DiscardCostShape::Cards {
        count,
        other,
        card_types,
        supertypes,
        random,
    })
}

fn parse_optional_discard_count<'a>(input: &mut LexStream<'a>) -> u32 {
    let mut probe = input.clone();
    if let Ok(count) = leaf::parse_leaf_number_prefix_lexed.parse_next(&mut probe) {
        *input = probe;
        count
    } else {
        1
    }
}

fn parse_indefinite_articles<'a>(input: &mut LexStream<'a>) {
    loop {
        let mut probe = input.clone();
        if alt((primitives::kw("a"), primitives::kw("an")))
            .parse_next(&mut probe)
            .is_err()
        {
            break;
        }
        *input = probe;
    }
}

fn parse_discard_disjunction<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<(usize, usize, usize)> {
    let left_first = initial_len.saturating_sub(input.len());
    let mut left_end = left_first;
    loop {
        let mut boundary = input.clone();
        if primitives::kw("or").parse_next(&mut boundary).is_ok() {
            if left_end == left_first {
                return Err(primitives::backtrack_err(
                    "discard disjunction",
                    "selector before or",
                ));
            }
            let right_first = initial_len.saturating_sub(boundary.len());
            let right: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(&mut boundary)?;
            if right.is_empty() {
                return Err(primitives::backtrack_err(
                    "discard disjunction",
                    "selector after or",
                ));
            }
            *input = boundary;
            return Ok((left_first, left_end, right_first));
        }
        any.parse_next(input)?;
        left_end += 1;
    }
}

fn parse_discard_type_descriptors<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(Vec<CardType>, Vec<Supertype>)> {
    let mut card_types = Vec::new();
    let mut supertypes = Vec::new();
    loop {
        let mut noun = input.clone();
        if alt((primitives::kw("card"), primitives::kw("cards")))
            .parse_next(&mut noun)
            .is_ok()
        {
            break;
        }
        let word = primitives::word_parser_text.parse_next(input)?;
        if matches!(word, "and" | "or" | "a" | "an") {
            continue;
        }
        if let Ok(supertype) = leaf::parse_leaf_supertype_complete(word) {
            crate::slice_primitives::push_unique(&mut supertypes, supertype);
            continue;
        }
        let card_type = leaf::parse_leaf_card_type_complete(word).map_err(|_| {
            primitives::backtrack_err(
                "discard card descriptor",
                "card type, supertype, or card noun",
            )
        })?;
        crate::slice_primitives::push_unique(&mut card_types, card_type);
    }
    Ok((card_types, supertypes))
}

fn parse_sacrifice_cost_shape_lexed<'a>(input: &mut LexStream<'a>) -> WResult<SacrificeCostShape> {
    let initial_len = input.len();
    primitives::kw("sacrifice").parse_next(input)?;
    alt((
        parse_sacrifice_source,
        parse_sacrifice_creature,
        move |input: &mut LexStream<'a>| parse_sacrifice_all(input, initial_len),
        move |input: &mut LexStream<'a>| parse_sacrifice_chosen(input, initial_len),
    ))
    .parse_next(input)
}

fn parse_sacrifice_all<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<SacrificeCostShape> {
    primitives::kw("all").parse_next(input)?;
    let filter_first = initial_len.saturating_sub(input.len());
    let filter_tokens: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(input)?;
    if filter_tokens.is_empty() {
        return Ok(SacrificeCostShape::MissingFilter);
    }
    Ok(SacrificeCostShape::All { filter_first })
}

fn parse_sacrifice_source<'a>(input: &mut LexStream<'a>) -> WResult<SacrificeCostShape> {
    alt((
        primitives::kw("it").void(),
        (
            primitives::kw("this"),
            opt(alt((
                primitives::kw("creature"),
                primitives::kw("artifact"),
                primitives::kw("aura"),
                primitives::kw("enchantment"),
                primitives::kw("equipment"),
                primitives::kw("fortification"),
                primitives::kw("land"),
                alt((
                    primitives::kw("permanent"),
                    primitives::kw("card"),
                    primitives::kw("token"),
                )),
            ))),
        )
            .void(),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(SacrificeCostShape::Source)
}

fn parse_sacrifice_creature<'a>(input: &mut LexStream<'a>) -> WResult<SacrificeCostShape> {
    primitives::phrase(&["a", "creature"]).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(SacrificeCostShape::Creature)
}

fn parse_sacrifice_chosen<'a>(
    input: &mut LexStream<'a>,
    initial_len: usize,
) -> WResult<SacrificeCostShape> {
    let count = parse_sacrifice_count(input)?;
    let other = alt((primitives::kw("other"), primitives::kw("another")))
        .parse_next(input)
        .is_ok();
    let filter_first = initial_len.saturating_sub(input.len());
    let filter_tokens: Vec<&OwnedLexToken> = repeat(0.., any).parse_next(input)?;
    if filter_tokens.is_empty() {
        return Ok(SacrificeCostShape::MissingFilter);
    }
    Ok(SacrificeCostShape::Chosen(SacrificeChosenShape {
        count,
        other,
        filter_first,
    }))
}

fn parse_sacrifice_count<'a>(input: &mut LexStream<'a>) -> WResult<ChoiceCount> {
    let mut count = input.clone();
    if let Ok(parsed) = leaf::parse_leaf_choice_count_prefix_lexed.parse_next(&mut count) {
        *input = count;
        return Ok(parsed);
    }
    Ok(ChoiceCount::exactly(1))
}

#[cfg(test)]
#[path = "object_segments/tests.rs"]
mod tests;

#[path = "object_segments/reference_programs.rs"]
mod reference_programs;
use reference_programs::parse_optional_object_count;
#[path = "object_segments/choice_programs.rs"]
mod choice_programs;
use choice_programs::{parse_tap_chosen_shape_lexed, parse_unattach_chosen_tail_lexed};
#[path = "object_segments/resource_programs.rs"]
mod resource_programs;
use resource_programs::parse_unattach_cost_shape_lexed;
