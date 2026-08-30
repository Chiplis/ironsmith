use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::ConditionExpr;
use crate::effect::Until;
use crate::grammar::primitives::WordSliceInput;
use crate::grammar::{filters, leaf, primitives};
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};

#[derive(Clone, Debug, PartialEq)]
pub struct GainAbilityDurationShape {
    pub start: usize,
    pub len: usize,
    pub duration: Until,
    pub condition: Option<ConditionExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeadingGainDurationShape {
    pub consumed_words: usize,
    pub duration: Until,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuotedGainDurationShape {
    pub close_quote_token: usize,
    pub duration: Until,
}

fn until_end_of_turn(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("end"),
        primitives::word_slice_exact("of"),
        primitives::word_slice_exact("turn"),
    )
        .value(Until::EndOfTurn)
        .parse_next(input)
}

fn until_end_of_combat(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("end"),
        primitives::word_slice_exact("of"),
        primitives::word_slice_exact("combat"),
    )
        .value(Until::EndOfCombat)
        .parse_next(input)
}

fn fixed_number(input: &mut WordSliceInput<'_>) -> WResult<u32> {
    let (value, consumed) = leaf::parse_leaf_number_prefix_words(input)
        .and_then(|number| number.into_fixed())
        .ok_or_else(|| primitives::backtrack_err("die result", "fixed number"))?;
    *input = input
        .get(consumed..)
        .ok_or_else(|| primitives::backtrack_err("die result", "available words"))?;
    Ok(value)
}

fn until_end_of_turn_or_any_player_rolls(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    primitives::word_slice_exact("until").parse_next(input)?;
    opt(primitives::word_slice_exact("the")).parse_next(input)?;
    (
        primitives::word_slice_exact("end"),
        primitives::word_slice_exact("of"),
        primitives::word_slice_exact("turn"),
        primitives::word_slice_exact("or"),
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("any"),
        primitives::word_slice_exact("player"),
        alt((
            primitives::word_slice_exact("rolls"),
            primitives::word_slice_exact("roll"),
        )),
        opt(alt((
            primitives::word_slice_exact("a"),
            primitives::word_slice_exact("an"),
        ))),
    )
        .parse_next(input)?;
    let result = fixed_number.parse_next(input)?;
    (
        primitives::word_slice_exact("whichever"),
        primitives::word_slice_exact("comes"),
        primitives::word_slice_exact("first"),
    )
        .parse_next(input)?;
    Ok(Until::EndOfTurnOrAnyPlayerRolls {
        result,
        matching_rolls_observed: 0,
    })
}

fn until_next_turn(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("your"),
        primitives::word_slice_exact("next"),
        primitives::word_slice_exact("turn"),
    )
        .value(Until::YourNextTurn)
        .parse_next(input)
}

fn until_next_upkeep(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("your"),
        primitives::word_slice_exact("next"),
        primitives::word_slice_exact("upkeep"),
    )
        .value(Until::YourNextUpkeep)
        .parse_next(input)
}

fn next_untap_step(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        alt((
            primitives::word_slice_exact("until"),
            primitives::word_slice_exact("during"),
        )),
        primitives::word_slice_exact("your"),
        primitives::word_slice_exact("next"),
        primitives::word_slice_exact("untap"),
        primitives::word_slice_exact("step"),
    )
        .value(Until::YourNextTurn)
        .parse_next(input)
}

fn simple_turn_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((
        until_end_of_turn_or_any_player_rolls,
        until_end_of_combat,
        until_end_of_turn,
        until_next_upkeep,
        until_next_turn,
        next_untap_step,
    ))
    .parse_next(input)
}

fn for_as_long_as(input: &mut WordSliceInput<'_>) -> WResult<()> {
    (
        primitives::word_slice_exact("for"),
        primitives::word_slice_exact("as"),
        primitives::word_slice_exact("long"),
        primitives::word_slice_exact("as"),
    )
        .void()
        .parse_next(input)
}

fn affected_object_counter_duration_lexed(input: &mut LexStream<'_>) -> WResult<Until> {
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        (
            alt((primitives::kw("that"), primitives::kw("this"))),
            alt((
                primitives::kw("artifact"),
                primitives::kw("creature"),
                primitives::kw("enchantment"),
                primitives::kw("land"),
                primitives::kw("permanent"),
            )),
        )
            .void(),
    ))
    .parse_next(input)?;
    primitives::kw("has").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let counter_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::phrase(&["on", "it"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;

    let counter_type =
        filters::parse_counter_type_from_tokens(counter_tokens).ok_or_else(|| {
            primitives::backtrack_err("counter-linked duration", "known counter type")
        })?;
    Ok(Until::ForAsLongAs(
        ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(counter_type),
    ))
}

#[cfg(test)]
#[path = "durations_inline_tests.rs"]
mod tests;

#[path = "durations/ability.rs"]
mod ability_programs;
pub use ability_programs::{
    parse_gain_ability_duration_shape, parse_leading_gain_duration_shape,
    parse_quoted_gain_duration_shape, parse_simple_ability_duration_shape,
};
#[path = "durations/reference.rs"]
mod reference_programs;
pub use reference_programs::parse_source_tapped_gain_duration_shape;
use reference_programs::{source_remains_on_battlefield, source_tapped_duration};
#[path = "durations/object_action.rs"]
mod object_action_programs;
use object_action_programs::you_control;
#[path = "durations/core.rs"]
mod core_programs;
use core_programs::continuous_duration;
#[path = "durations/counter.rs"]
mod counter_programs;
pub use counter_programs::parse_leading_affected_object_counter_duration_shape;
