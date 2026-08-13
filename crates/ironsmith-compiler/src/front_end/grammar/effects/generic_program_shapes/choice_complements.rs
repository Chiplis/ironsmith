use crate::cards::builders::PlayerAst;
use crate::effect::{ChoiceAggregateConstraint, ChoiceCount};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::Subtype;
use winnow::combinator::opt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::primitives;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AggregateChoiceComplementShape {
    pub(crate) chooser: PlayerAst,
    pub(crate) filter: ObjectFilter,
    pub(crate) count: ChoiceCount,
    pub(crate) constraint: ChoiceAggregateConstraint,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DistinctSlotChoiceComplementShape {
    pub(crate) chooser: PlayerAst,
    pub(crate) filter: ObjectFilter,
    pub(crate) slot_filters: Vec<ObjectFilter>,
    pub(crate) count_per_slot: ChoiceCount,
}

fn controlled_creature_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    filter.controller = Some(PlayerFilter::IteratedPlayer);
    filter
}

fn parse_aggregate_choice_complement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AggregateChoiceComplementShape> {
    primitives::phrase(&[
        "each",
        "player",
        "chooses",
        "any",
        "number",
        "of",
        "creatures",
        "they",
        "control",
        "with",
        "total",
        "power",
    ])
    .parse_next(input)?;
    let maximum = primitives::number_token.parse_next(input)?;
    primitives::phrase(&["or", "less"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "then",
        "sacrifices",
        "all",
        "other",
        "creatures",
        "they",
        "control",
    ])
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(AggregateChoiceComplementShape {
        chooser: PlayerAst::Any,
        filter: controlled_creature_filter(),
        count: ChoiceCount::any_number(),
        constraint: ChoiceAggregateConstraint::total_power_at_most(
            i32::try_from(maximum).unwrap_or(i32::MAX),
        ),
    })
}

pub(crate) fn parse_aggregate_choice_complement_shape(
    tokens: &[OwnedLexToken],
) -> Option<AggregateChoiceComplementShape> {
    primitives::parse_all(
        tokens,
        parse_aggregate_choice_complement_lexed,
        "aggregate-choice-complement",
    )
    .ok()
}

fn parse_party_choice_complement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<DistinctSlotChoiceComplementShape> {
    primitives::phrase(&[
        "each",
        "player",
        "chooses",
        "a",
        "party",
        "from",
        "among",
        "creatures",
        "they",
        "control",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["then", "sacrifices", "the", "rest"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let slot_filters = [
        Subtype::Cleric,
        Subtype::Rogue,
        Subtype::Warrior,
        Subtype::Wizard,
    ]
    .into_iter()
    .map(|subtype| ObjectFilter::creature().with_subtype(subtype))
    .collect();
    Ok(DistinctSlotChoiceComplementShape {
        chooser: PlayerAst::Any,
        filter: controlled_creature_filter(),
        slot_filters,
        count_per_slot: ChoiceCount::up_to(1),
    })
}

pub(crate) fn parse_party_choice_complement_shape(
    tokens: &[OwnedLexToken],
) -> Option<DistinctSlotChoiceComplementShape> {
    primitives::parse_all(
        tokens,
        parse_party_choice_complement_lexed,
        "party-choice-complement",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn parses_aggregate_choice_complement_directly_to_typed_semantics() {
        let tokens = lex_line(
            "Each player chooses any number of creatures they control with total power 4 or less, then sacrifices all other creatures they control.",
            0,
        )
        .expect("aggregate choice fixture");
        let parsed =
            parse_aggregate_choice_complement_shape(&tokens).expect("aggregate choice shape");

        assert_eq!(parsed.chooser, PlayerAst::Any);
        assert!(parsed.count.is_any_number());
        assert_eq!(parsed.filter.controller, Some(PlayerFilter::IteratedPlayer));
        assert_eq!(
            parsed.constraint,
            ChoiceAggregateConstraint::total_power_at_most(4)
        );
    }

    #[test]
    fn parses_party_as_four_optional_distinct_choice_slots() {
        let tokens = lex_line(
            "Each player chooses a party from among creatures they control, then sacrifices the rest.",
            0,
        )
        .expect("party choice fixture");
        let parsed = parse_party_choice_complement_shape(&tokens).expect("party choice shape");

        assert_eq!(parsed.chooser, PlayerAst::Any);
        assert_eq!(parsed.count_per_slot, ChoiceCount::up_to(1));
        assert_eq!(parsed.filter.controller, Some(PlayerFilter::IteratedPlayer));
        assert_eq!(
            parsed
                .slot_filters
                .iter()
                .flat_map(|filter| filter.subtypes.iter().copied())
                .collect::<Vec<_>>(),
            vec![
                Subtype::Cleric,
                Subtype::Rogue,
                Subtype::Warrior,
                Subtype::Wizard,
            ]
        );
    }
}
