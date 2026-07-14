use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Value;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::runtime_backend::grammar::shared_util::value_semantics::{
    parse_value_prefix_lexed, parse_where_x_greatest_commander_mana_value,
};
use crate::runtime_backend::util::parse_number_or_x_value_lexed;
use ironsmith_core::{EffectMetric, EffectMetricSource, EventValueSpec, ValueSurfaceHint};

use super::super::super::{permission_shapes, primitives};

const MEMORY_ACTION_WORDS: &[&str] = &[
    "chosen",
    "destroyed",
    "discarded",
    "exiled",
    "milled",
    "revealed",
    "sacrificed",
    "searched",
];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TopCardsViewShape {
    pub(crate) revealed: bool,
    pub(crate) count: Value,
}

fn top_cards_view_head(input: &mut LexStream<'_>) -> WResult<bool> {
    alt((
        primitives::phrase(&["look", "at", "the", "top"]).value(false),
        primitives::phrase(&["look", "at", "top"]).value(false),
        primitives::phrase(&["reveal", "the", "top"]).value(true),
        primitives::phrase(&["reveal", "top"]).value(true),
    ))
    .parse_next(input)
}

fn card_of_your_library(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["card", "of", "your", "library"]),
        primitives::phrase(&["cards", "of", "your", "library"]),
    ))
    .void()
    .parse_next(input)
}

fn that_many_cards_from_top_head(input: &mut LexStream<'_>) -> WResult<bool> {
    alt((
        primitives::phrase(&[
            "look", "at", "that", "many", "cards", "from", "the", "top", "of", "your", "library",
        ])
        .value(false),
        primitives::phrase(&[
            "reveal", "that", "many", "cards", "from", "the", "top", "of", "your", "library",
        ])
        .value(true),
    ))
    .parse_next(input)
}

fn prior_effect_count_head(input: &mut LexStream<'_>) -> WResult<()> {
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of"])
        .void()
        .parse_next(input)
}

fn contains_memory_action(tokens: &[OwnedLexToken]) -> bool {
    MEMORY_ACTION_WORDS
        .iter()
        .any(|word| permission_shapes::contains_tokens(tokens, &[*word]))
}

fn parse_prior_effect_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let (_, object_tokens) = primitives::parse_prefix(tokens, prior_effect_count_head)?;
    let references_this_way = permission_shapes::contains_tokens(object_tokens, &["this", "way"]);
    if !references_this_way && !contains_memory_action(object_tokens) {
        return None;
    }
    Some(Value::PendingEffectMetric {
        source: if permission_shapes::contains_tokens(object_tokens, &["chosen"]) {
            EffectMetricSource::ChosenObjects
        } else {
            EffectMetricSource::AffectedObjects
        },
        metric: EffectMetric::Count,
    })
}

pub(crate) fn parse_where_x_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if let Some(value) = parse_prior_effect_count_value(tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::WhereXIs));
    }
    if let Some((value, used)) = parse_value_prefix_lexed(tokens)
        && trim_lexed_commas(tokens.get(used..)?).is_empty()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::WhereXIs));
    }

    let commander_start =
        if permission_shapes::prefix_tokens(tokens, &["the", "greatest", "mana", "value", "of"]) {
            5
        } else if permission_shapes::prefix_tokens(tokens, &["greatest", "mana", "value", "of"]) {
            4
        } else {
            return None;
        };
    parse_where_x_greatest_commander_mana_value(tokens, commander_start)
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
}

pub(crate) fn parse_top_cards_view_shape(tokens: &[OwnedLexToken]) -> Option<TopCardsViewShape> {
    let tokens = trim_lexed_commas(tokens);
    if let Some((revealed, remainder)) =
        primitives::parse_prefix(tokens, that_many_cards_from_top_head)
        && trim_lexed_commas(remainder).is_empty()
    {
        return Some(TopCardsViewShape {
            revealed,
            count: Value::EventValue(EventValueSpec::Amount),
        });
    }
    let (revealed, count_tokens) = primitives::parse_prefix(tokens, top_cards_view_head)?;
    let (count, used) = parse_number_or_x_value_lexed(count_tokens)?;
    let library_tokens = trim_lexed_commas(count_tokens.get(used..)?);
    let (_, remainder) = primitives::parse_prefix(library_tokens, card_of_your_library)?;
    let remainder = trim_lexed_commas(remainder);
    if remainder.is_empty() {
        return Some(TopCardsViewShape { revealed, count });
    }
    if count != Value::X {
        return None;
    }
    let (_, value_tokens) =
        primitives::parse_prefix(remainder, primitives::phrase(&["where", "x", "is"]).void())?;
    Some(TopCardsViewShape {
        revealed,
        count: parse_where_x_value(trim_lexed_commas(value_tokens))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_typed_top_card_view_counts() {
        let tokens = lex_line("Look at the top three cards of your library", 0).unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();
        assert!(!shape.revealed);
        assert_eq!(shape.count, Value::Fixed(3));

        let tokens = lex_line(
            "Reveal the top X cards of your library, where X is the number of cards milled this way",
            0,
        )
        .unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();
        assert!(shape.revealed);
        assert!(shape.count.has_surface_hint(ValueSurfaceHint::WhereXIs));

        let tokens = lex_line("Look at that many cards from the top of your library", 0).unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();
        assert!(!shape.revealed);
        assert_eq!(shape.count, Value::EventValue(EventValueSpec::Amount));
    }
}
