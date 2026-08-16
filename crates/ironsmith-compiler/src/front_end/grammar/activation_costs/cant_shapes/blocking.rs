use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::color::ColorSet;
use crate::filter::Comparison;
use crate::static_abilities::StaticAbilityId;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockingCantSubject {
    Bare,
    This,
    ThisCreature,
}

/// Fully typed blocking restrictions parsed from a complete `can't be
/// blocked by` or `can't be blocked except by` clause.
///
/// `DisallowedBlockers::filter` always describes creatures that are forbidden
/// from blocking the source. For `except by` surfaces the allowed quality is
/// inverted here, so callers do not need to rediscover which relation was
/// written.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BlockingCantFact {
    MaximumBlockers {
        subject: BlockingCantSubject,
        maximum_blockers: usize,
    },
    PowerThreshold {
        subject: BlockingCantSubject,
        comparison: Comparison,
    },
    DisallowedBlockers {
        subject: BlockingCantSubject,
        filter: ObjectFilter,
    },
    MinimumBlockers {
        subject: BlockingCantSubject,
        minimum_blockers: usize,
    },
}

pub(crate) fn parse_blocking_cant_fact_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BlockingCantFact> {
    primitives::parse_all(tokens, parse_blocking_cant_fact_lexed, "blocking cant fact").ok()
}

#[derive(Debug, Clone, PartialEq)]
enum BlockingCantTail {
    MaximumBlockers(usize),
    PowerThreshold(Comparison),
    DisallowedBlockers(ObjectFilter),
    MinimumBlockers(usize),
}

fn parse_blocking_cant_fact_lexed<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantFact> {
    let subject = parse_blocking_subject.parse_next(input)?;
    parse_cant.parse_next(input)?;
    primitives::phrase(&["be", "blocked"]).parse_next(input)?;
    let tail = alt((
        (primitives::phrase(&["except", "by"]), parse_except_by_tail).map(|(_, tail)| tail),
        (primitives::kw("by"), parse_by_tail).map(|(_, tail)| tail),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(match tail {
        BlockingCantTail::MaximumBlockers(maximum_blockers) => BlockingCantFact::MaximumBlockers {
            subject,
            maximum_blockers,
        },
        BlockingCantTail::PowerThreshold(comparison) => BlockingCantFact::PowerThreshold {
            subject,
            comparison,
        },
        BlockingCantTail::DisallowedBlockers(filter) => {
            BlockingCantFact::DisallowedBlockers { subject, filter }
        }
        BlockingCantTail::MinimumBlockers(minimum_blockers) => BlockingCantFact::MinimumBlockers {
            subject,
            minimum_blockers,
        },
    })
}

fn parse_blocking_subject<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantSubject> {
    Ok(opt(alt((
        primitives::phrase(&["this", "creature"]).value(BlockingCantSubject::ThisCreature),
        primitives::kw("this").value(BlockingCantSubject::This),
    )))
    .parse_next(input)?
    .unwrap_or(BlockingCantSubject::Bare))
}

fn parse_cant<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("can't"),
        primitives::kw("cant"),
        primitives::kw("cannot"),
    ))
    .void()
    .parse_next(input)
}

fn parse_by_tail<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    alt((
        parse_maximum_blockers,
        parse_power_threshold,
        parse_flying_blockers,
        parse_color_blockers,
        parse_wall_blockers,
    ))
    .parse_next(input)
}

fn parse_except_by_tail<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    alt((
        parse_minimum_blockers,
        parse_except_color_blockers,
        parse_except_artifact_blockers,
        parse_except_wall_blockers,
    ))
    .parse_next(input)
}

fn parse_maximum_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    let minimum = parse_minimum_quantity.parse_next(input)?;
    parse_creature_noun.parse_next(input)?;
    let maximum_blockers = minimum.checked_sub(1).ok_or_else(|| {
        primitives::backtrack_err("maximum blockers", "positive blocker quantity")
    })?;
    Ok(BlockingCantTail::MaximumBlockers(maximum_blockers))
}

fn parse_minimum_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    let minimum_blockers = parse_minimum_quantity.parse_next(input)?;
    parse_creature_noun.parse_next(input)?;
    if minimum_blockers == 0 {
        return Err(primitives::backtrack_err(
            "minimum blockers",
            "positive blocker quantity",
        ));
    }
    Ok(BlockingCantTail::MinimumBlockers(minimum_blockers))
}

fn parse_minimum_quantity<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    alt((
        parse_more_than_quantity,
        parse_at_least_quantity,
        parse_number_or_more_quantity,
    ))
    .parse_next(input)
}

fn parse_more_than_quantity<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    primitives::phrase(&["more", "than"]).parse_next(input)?;
    let count = parse_number_usize.parse_next(input)?;
    count
        .checked_add(1)
        .ok_or_else(|| primitives::backtrack_err("blocker quantity", "representable blocker count"))
}

fn parse_at_least_quantity<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    primitives::phrase(&["at", "least"]).parse_next(input)?;
    parse_number_usize.parse_next(input)
}

fn parse_number_or_more_quantity<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let count = parse_number_usize.parse_next(input)?;
    opt(primitives::phrase(&["or", "more"]))
        .void()
        .parse_next(input)?;
    Ok(count)
}

fn parse_number_usize<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    usize::try_from(count)
        .map_err(|_| primitives::backtrack_err("blocker quantity", "representable blocker count"))
}

fn parse_power_threshold<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    opt(parse_creature_noun).parse_next(input)?;
    primitives::phrase(&["with", "power"]).parse_next(input)?;
    let threshold = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    let threshold = i32::try_from(threshold)
        .map_err(|_| primitives::backtrack_err("blocker power", "signed-safe power threshold"))?;
    primitives::kw("or").parse_next(input)?;
    let comparison = alt((
        primitives::kw("less").value(Comparison::LessThanOrEqual(threshold)),
        alt((primitives::kw("greater"), primitives::kw("more")))
            .value(Comparison::GreaterThanOrEqual(threshold)),
    ))
    .parse_next(input)?;
    Ok(BlockingCantTail::PowerThreshold(comparison))
}

fn parse_flying_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    opt(parse_creature_noun).parse_next(input)?;
    primitives::phrase(&["with", "flying"]).parse_next(input)?;
    Ok(BlockingCantTail::DisallowedBlockers(
        ObjectFilter::creature().with_static_ability(StaticAbilityId::Flying),
    ))
}

fn parse_color_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    let colors = parse_color_set.parse_next(input)?;
    parse_creature_noun.parse_next(input)?;
    Ok(BlockingCantTail::DisallowedBlockers(
        ObjectFilter::creature().with_colors(colors),
    ))
}

fn parse_wall_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    parse_wall_noun.parse_next(input)?;
    Ok(BlockingCantTail::DisallowedBlockers(
        ObjectFilter::creature().with_subtype(Subtype::Wall),
    ))
}

fn parse_except_color_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    let colors = parse_color_set.parse_next(input)?;
    parse_creature_noun.parse_next(input)?;
    Ok(BlockingCantTail::DisallowedBlockers(
        ObjectFilter::creature().without_colors(colors),
    ))
}

fn parse_except_artifact_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    primitives::kw("artifact").parse_next(input)?;
    parse_creature_noun.parse_next(input)?;
    Ok(BlockingCantTail::DisallowedBlockers(
        ObjectFilter::creature().without_type(CardType::Artifact),
    ))
}

fn parse_except_wall_blockers<'a>(input: &mut LexStream<'a>) -> WResult<BlockingCantTail> {
    parse_wall_noun.parse_next(input)?;
    Ok(BlockingCantTail::DisallowedBlockers(
        ObjectFilter::creature().without_subtype(Subtype::Wall),
    ))
}

fn parse_color_set<'a>(input: &mut LexStream<'a>) -> WResult<ColorSet> {
    let word = primitives::word_parser_text.parse_next(input)?;
    leaf::parse_leaf_color_complete(word)
        .map_err(|_| primitives::backtrack_err("blocker color", "Magic color word"))
}

fn parse_creature_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .void()
        .parse_next(input)
}

fn parse_wall_noun<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("wall"), primitives::kw("walls")))
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse(raw: &str) -> Option<BlockingCantFact> {
        let tokens = lex_line(raw, 0).expect("lex blocking cant fixture");
        parse_blocking_cant_fact_tokens(&tokens)
    }

    #[test]
    fn parses_blocker_count_and_power_facts() {
        let cases = [
            (
                "Can't be blocked by more than one creature.",
                BlockingCantFact::MaximumBlockers {
                    subject: BlockingCantSubject::Bare,
                    maximum_blockers: 1,
                },
            ),
            (
                "This creature can't be blocked by three or more creatures.",
                BlockingCantFact::MaximumBlockers {
                    subject: BlockingCantSubject::ThisCreature,
                    maximum_blockers: 2,
                },
            ),
            (
                "This can't be blocked by creatures with power two or less.",
                BlockingCantFact::PowerThreshold {
                    subject: BlockingCantSubject::This,
                    comparison: Comparison::LessThanOrEqual(2),
                },
            ),
            (
                "This creature cannot be blocked by creatures with power 3 or more.",
                BlockingCantFact::PowerThreshold {
                    subject: BlockingCantSubject::ThisCreature,
                    comparison: Comparison::GreaterThanOrEqual(3),
                },
            ),
            (
                "This can't be blocked except by two or more creatures.",
                BlockingCantFact::MinimumBlockers {
                    subject: BlockingCantSubject::This,
                    minimum_blockers: 2,
                },
            ),
            (
                "This creature can't be blocked except by more than two creatures.",
                BlockingCantFact::MinimumBlockers {
                    subject: BlockingCantSubject::ThisCreature,
                    minimum_blockers: 3,
                },
            ),
        ];

        for (raw, expected) in cases {
            assert_eq!(parse(raw), Some(expected), "fixture: {raw}");
        }
    }

    #[test]
    fn parses_by_filter_facts() {
        let cases = [
            (
                "Can't be blocked by creatures with flying.",
                BlockingCantSubject::Bare,
                ObjectFilter::creature().with_static_ability(StaticAbilityId::Flying),
            ),
            (
                "This can't be blocked by red creatures.",
                BlockingCantSubject::This,
                ObjectFilter::creature().with_colors(ColorSet::RED),
            ),
            (
                "This creature can't be blocked by Walls.",
                BlockingCantSubject::ThisCreature,
                ObjectFilter::creature().with_subtype(Subtype::Wall),
            ),
        ];

        for (raw, subject, filter) in cases {
            assert_eq!(
                parse(raw),
                Some(BlockingCantFact::DisallowedBlockers { subject, filter }),
                "fixture: {raw}"
            );
        }
    }

    #[test]
    fn parses_except_by_filters_as_disallowed_blockers() {
        let cases = [
            (
                "Can't be blocked except by blue creatures.",
                BlockingCantSubject::Bare,
                ObjectFilter::creature().without_colors(ColorSet::BLUE),
            ),
            (
                "This can't be blocked except by artifact creatures.",
                BlockingCantSubject::This,
                ObjectFilter::creature().without_type(CardType::Artifact),
            ),
            (
                "This creature can't be blocked except by Walls.",
                BlockingCantSubject::ThisCreature,
                ObjectFilter::creature().without_subtype(Subtype::Wall),
            ),
        ];

        for (raw, subject, filter) in cases {
            assert_eq!(
                parse(raw),
                Some(BlockingCantFact::DisallowedBlockers { subject, filter }),
                "fixture: {raw}"
            );
        }
    }

    #[test]
    fn rejects_incomplete_or_extended_blocking_surfaces() {
        for raw in [
            "Can be blocked by more than one creature.",
            "This token can't be blocked by more than one creature.",
            "This creature can't be blocked by one.",
            "This creature can't be blocked by creatures with power two or equal.",
            "This creature can't be blocked by creatures with flying or reach.",
            "This creature can't be blocked by purple creatures.",
            "This creature can't be blocked except blue creatures.",
            "This creature can't be blocked except by artifact.",
            "This creature can't be blocked except by zero creatures.",
            "This creature can't be blocked by Walls this turn.",
        ] {
            assert_eq!(parse(raw), None, "near miss: {raw}");
        }
    }
}
