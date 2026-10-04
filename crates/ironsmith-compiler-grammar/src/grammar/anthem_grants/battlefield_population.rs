//! Complete live-battlefield population gates. These clauses quantify objects,
//! rather than describing the source or an attached/iterated recipient.
use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::color::Color;
use crate::lexer::{LexStream, OwnedLexToken};
use crate::object::CounterType;
use super::super::{filters, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldPopulationCondition<'a> {
    MostCommonColorIncludingTies(Color),
    AnyPlayerControls(&'a [OwnedLexToken]),
    NoOpponentControls(&'a [OwnedLexToken]),
    CreatureHasCounter(CounterType),
}

fn color(input: &mut LexStream<'_>) -> WResult<Color> {
    alt((
        primitives::kw("white").value(Color::White),
        primitives::kw("blue").value(Color::Blue),
        primitives::kw("black").value(Color::Black),
        primitives::kw("red").value(Color::Red),
        primitives::kw("green").value(Color::Green),
    )).parse_next(input)
}

fn most_common_color(input: &mut LexStream<'_>) -> WResult<Color> {
    let result = color.parse_next(input)?;
    primitives::phrase(&["is", "the", "most", "common", "color", "among", "all", "permanents", "or", "is", "tied", "for", "most", "common"]).parse_next(input)?;
    Ok(result)
}

pub fn parse_battlefield_population_condition(tokens: &[OwnedLexToken]) -> Option<BattlefieldPopulationCondition<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if let Some(color) = primitives::probe_all(tokens, most_common_color, "most common permanent color including ties") {
        return Some(BattlefieldPopulationCondition::MostCommonColorIncludingTies(color));
    }
    if let Some(((), filter)) = primitives::parse_prefix(tokens, primitives::phrase(&["any", "player", "controls"])) {
        return (!filter.is_empty()).then_some(BattlefieldPopulationCondition::AnyPlayerControls(filter));
    }
    if let Some(((), filter)) = primitives::parse_prefix(tokens, primitives::phrase(&["no", "opponent", "controls"])) {
        return (!filter.is_empty()).then_some(BattlefieldPopulationCondition::NoOpponentControls(filter));
    }
    let ((), rest) = primitives::parse_prefix(tokens, primitives::phrase(&["a", "creature", "has", "a"]))?;
    let (counter_tokens, holder) = primitives::split_lexed_once_on_separator(rest, || primitives::kw("counter").void())?;
    primitives::probe_all(holder, primitives::phrase(&["on", "it"]), "counter holder")?;
    Some(BattlefieldPopulationCondition::CreatureHasCounter(filters::parse_counter_type_from_tokens(counter_tokens)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Value, ValueComparisonOperator};
    use crate::host::PredicateAst;
    use crate::keyword_static::parse_static_condition_clause;
    use crate::lexer::lex_line;
    use crate::filter::{ObjectFilter, PlayerFilter};
    use crate::color::ColorSet;

    #[test]
    fn population_readings_keep_global_scope_opponent_negation_and_counter_kind() {
        for (text, filter, operator, threshold) in [
            ("any player controls a black permanent", ObjectFilter::permanent().with_colors(ColorSet::BLACK), ValueComparisonOperator::GreaterThanOrEqual, 1),
            ("no opponent controls a creature", ObjectFilter::creature().controlled_by(PlayerFilter::Opponent), ValueComparisonOperator::Equal, 0),
            ("no opponent controls a white or blue creature", ObjectFilter::creature().with_colors(ColorSet::WHITE.with(Color::Blue)).controlled_by(PlayerFilter::Opponent), ValueComparisonOperator::Equal, 0),
            ("a creature has a -1/-1 counter on it", ObjectFilter::creature().with_counter_type(CounterType::MinusOneMinusOne), ValueComparisonOperator::GreaterThanOrEqual, 1),
        ] {
            let parsed = parse_static_condition_clause(&lex_line(text, 0).unwrap()).unwrap();
            let PredicateAst::ValueComparison { left: Value::Count(actual), operator: actual_operator, right: Value::Fixed(actual_threshold) } = parsed else { panic!("{text}: {parsed:?}"); };
            // Surface annotations need not compare equal; the semantic fields do.
            assert_eq!(actual.zone, filter.zone, "{text}");
            if filter.card_types.is_empty() {
                assert!(actual.card_types.is_empty() || actual.has_all_permanent_card_types(), "{text}");
            } else {
                assert_eq!(actual.card_types, filter.card_types, "{text}");
            }
            assert_eq!(actual.colors, filter.colors, "{text}");
            assert_eq!(actual.controller, filter.controller, "{text}");
            assert_eq!(actual.with_counter, filter.with_counter, "{text}");
            assert_eq!((actual_operator, actual_threshold), (operator, threshold));
        }
    }

    #[test]
    fn common_color_shape_requires_complete_tie_clause_and_retains_all_five_colors() {
        for (word, color) in [("white", Color::White), ("blue", Color::Blue), ("black", Color::Black), ("red", Color::Red), ("green", Color::Green)] {
            let text = format!("{word} is the most common color among all permanents or is tied for most common");
            let tokens = lex_line(&text, 0).unwrap();
            assert_eq!(parse_battlefield_population_condition(&tokens), Some(BattlefieldPopulationCondition::MostCommonColorIncludingTies(color)));
            assert!(matches!(parse_static_condition_clause(&tokens).unwrap(), PredicateAst::And(_, _)));
        }
        for text in [
            "black is the most common color among all permanents",
            "black is the most common color among all permanents or is tied for most common this turn",
            "a creature has a -1/-1 counter on it and you control a Forest",
            "a creature has a -1/-1 counter on another creature",
        ] {
            assert!(parse_battlefield_population_condition(&lex_line(text, 0).unwrap()).is_none(), "{text}");
        }
        for text in ["any player controls a creature card in a graveyard", "no opponent controls a creature you control"] {
            assert!(parse_static_condition_clause(&lex_line(text, 0).unwrap()).is_err(), "{text}");
        }
    }
}
