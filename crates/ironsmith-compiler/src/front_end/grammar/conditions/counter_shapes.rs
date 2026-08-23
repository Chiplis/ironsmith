use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::CounterType;
use crate::effect::Comparison;
use crate::target::PlayerFilter;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{filters, primitives, shared_util};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCounterConditionShape {
    pub player: PlayerFilter,
    pub comparison: Comparison,
    pub counter_type: CounterType,
}

pub fn parse_player_counter_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCounterConditionShape> {
    let tokens = trim_lexed_commas(tokens);
    let ((player, ()), quantity_tokens) =
        primitives::parse_prefix(tokens, (parse_player_subject, parse_has))?;
    let quantity = shared_util::value_shapes::parse_quantity_comparison_prefix_tokens(
        quantity_tokens,
        false,
        false,
    )?;
    let counter_tail = quantity_tokens.get(quantity.consumed_tokens..)?;
    let (descriptor_tokens, ()) =
        primitives::split_lexed_once_before_suffix(counter_tail, 0, || {
            (
                alt((primitives::kw("counter"), primitives::kw("counters"))),
                primitives::sentence_end(),
            )
                .void()
        })?;
    let counter_type = if primitives::parse_all(
        descriptor_tokens,
        primitives::kw("poison"),
        "player poison counter type",
    )
    .is_ok()
    {
        CounterType::Poison
    } else {
        filters::parse_counter_type_from_tokens(descriptor_tokens)?
    };
    Some(PlayerCounterConditionShape {
        player,
        comparison: quantity.comparison,
        counter_type,
    })
}

fn parse_player_subject(input: &mut LexStream<'_>) -> WResult<PlayerFilter> {
    alt((
        primitives::kw("you").value(PlayerFilter::You),
        primitives::any_phrase(&[
            &["an", "opponent"],
            &["a", "opponent"],
            &["each", "opponent"],
            &["opponent"],
        ])
        .value(PlayerFilter::Opponent),
        primitives::any_phrase(&[&["a", "player"], &["each", "player"], &["any", "player"]])
            .value(PlayerFilter::Any),
    ))
    .parse_next(input)
}

fn parse_has(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("has"), primitives::kw("have")))
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_player_counter_thresholds() {
        let tokens = lex_line("An opponent has three or more poison counters.", 0).unwrap();
        let shape = parse_player_counter_condition(&tokens).expect("counter threshold");
        assert_eq!(shape.player, PlayerFilter::Opponent);
        assert_eq!(shape.comparison, Comparison::GreaterThanOrEqual(3));
        assert_eq!(shape.counter_type, CounterType::Poison);
    }
}
