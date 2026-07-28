use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::{leaf, permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView,
};

use super::super::{seek_sequence_phrase, sequence_any_phrase, sequence_phrase};
use super::{ConsultManaValueConditionShape, parse_consult_mana_value_condition_shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConsultCastTimingShape {
    Immediate,
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConsultCastCostShape {
    Normal,
    WithoutPayingManaCost,
    PayLifeEqualToManaValue,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConsultCastShape {
    pub(crate) caster: Vec<OwnedLexToken>,
    pub(crate) allow_land: bool,
    pub(crate) timing: ConsultCastTimingShape,
    pub(crate) cost: ConsultCastCostShape,
    pub(crate) mana_value_condition: Option<ConsultManaValueConditionShape>,
    pub(crate) surface: ironsmith_core::GrantPlayTaggedSurface,
}

const PAY_LIFE_MANA_VALUE_CLAUSE: &[&str] = &[
    "by", "paying", "life", "equal", "to", "the", "spell's", "mana", "value", "rather", "than",
    "paying", "its", "mana", "cost",
];
const WITHOUT_PAYING_MANA_COST: &[&str] = &["without", "paying", "its", "mana", "cost"];

fn trim_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0;
    let mut end = tokens.len();
    while start < end && tokens[start].kind == TokenKind::Comma {
        start += 1;
    }
    while end > start && tokens[end - 1].kind == TokenKind::Comma {
        end -= 1;
    }
    &tokens[start..end]
}

fn consult_cast_action(
    input: &mut LexStream<'_>,
) -> WResult<(bool, ironsmith_core::GrantPlayTaggedObjectSurface)> {
    alt((
        sequence_phrase(&["cast", "that", "card"]).value((
            false,
            ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard,
        )),
        sequence_phrase(&["cast", "it"])
            .value((false, ironsmith_core::GrantPlayTaggedObjectSurface::It)),
        sequence_any_phrase(&[
            &["cast", "that", "exiled", "card"],
            &["cast", "the", "exiled", "card"],
        ])
        .value((
            false,
            ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard,
        )),
        sequence_phrase(&["play", "that", "card"])
            .value((true, ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard)),
        sequence_phrase(&["play", "it"])
            .value((true, ironsmith_core::GrantPlayTaggedObjectSurface::It)),
    ))
    .parse_next(input)
}

pub(crate) fn parse_consult_cast_shape(tokens: &[OwnedLexToken]) -> Option<ConsultCastShape> {
    let mut clause = trim_commas(tokens);
    let mut timing = ConsultCastTimingShape::Immediate;
    let mut leading_duration = false;
    if let Some(parsed) = leaf::parse_leaf_turn_duration_prefix_tokens(clause) {
        match parsed.duration {
            leaf::LeafTurnDurationPhrase::UntilEndOfTurn => {
                clause = trim_commas(parsed.rest);
                timing = ConsultCastTimingShape::UntilEndOfTurn;
                leading_duration = true;
            }
            leaf::LeafTurnDurationPhrase::UntilYourNextTurnEnd => {
                clause = trim_commas(parsed.rest);
                timing = ConsultCastTimingShape::UntilYourNextTurnEnd;
                leading_duration = true;
            }
            leaf::LeafTurnDurationPhrase::ThisTurn
            | leaf::LeafTurnDurationPhrase::UntilYourNextTurn => {}
        }
    }

    let mut may_input = LexStream::new(clause);
    let may_start = seek_sequence_phrase(&mut may_input, &[&["may"]]).ok()?;
    if may_start == 0 {
        return None;
    }
    sequence_phrase(&["may"]).parse_next(&mut may_input).ok()?;
    let after_may = clause.len().saturating_sub(may_input.len());
    let caster = trim_commas(&clause[..may_start]);
    if caster.is_empty() {
        return None;
    }
    let ((allow_land, object_surface), remainder) =
        primitives::parse_prefix(&clause[after_may..], consult_cast_action)?;
    let remainder = trim_commas(remainder);
    let surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(leading_duration)
        .with_object(object_surface);

    if permission_shapes::exact_tokens(remainder, &["this", "turn"]) {
        return Some(ConsultCastShape {
            caster: caster.to_vec(),
            allow_land,
            timing: ConsultCastTimingShape::UntilEndOfTurn,
            cost: ConsultCastCostShape::Normal,
            mana_value_condition: None,
            surface,
        });
    }
    if permission_shapes::exact_tokens(remainder, PAY_LIFE_MANA_VALUE_CLAUSE) {
        return Some(ConsultCastShape {
            caster: caster.to_vec(),
            allow_land,
            timing,
            cost: ConsultCastCostShape::PayLifeEqualToManaValue,
            mana_value_condition: None,
            surface,
        });
    }

    let (_, condition_tokens) =
        primitives::parse_prefix(remainder, sequence_any_phrase(&[WITHOUT_PAYING_MANA_COST]))?;
    let condition_tokens = trim_commas(condition_tokens);
    let mana_value_condition = if TokenWordView::new(condition_tokens).is_empty() {
        None
    } else {
        Some(parse_consult_mana_value_condition_shape(condition_tokens)?)
    };
    Some(ConsultCastShape {
        caster: caster.to_vec(),
        allow_land,
        timing,
        cost: ConsultCastCostShape::WithoutPayingManaCost,
        mana_value_condition,
        surface,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Value, ValueComparisonOperator};
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_consult_cast_timing_cost_and_condition() {
        let normal = parse_consult_cast_shape(&lex("You may play that card this turn")).unwrap();
        assert!(normal.allow_land);
        assert_eq!(normal.timing, ConsultCastTimingShape::UntilEndOfTurn);
        assert_eq!(normal.cost, ConsultCastCostShape::Normal);
        assert!(!normal.surface.leading_duration);
        assert_eq!(
            normal.surface.object,
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard)
        );

        let conditioned = parse_consult_cast_shape(&lex(
            "Until the end of your next turn, you may cast it without paying its mana cost if its mana value is less than 4",
        ))
        .unwrap();
        assert_eq!(
            conditioned.timing,
            ConsultCastTimingShape::UntilYourNextTurnEnd
        );
        assert_eq!(
            conditioned.cost,
            ConsultCastCostShape::WithoutPayingManaCost
        );
        assert!(conditioned.surface.leading_duration);
        assert_eq!(
            conditioned.surface.object,
            Some(ironsmith_core::GrantPlayTaggedObjectSurface::It)
        );
        let condition = conditioned.mana_value_condition.unwrap();
        assert_eq!(condition.operator, ValueComparisonOperator::LessThan);
        assert_eq!(condition.right, Value::Fixed(4));
    }
}
