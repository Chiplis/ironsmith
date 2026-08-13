use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Value;
use crate::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::grammar::shared_util::value_semantics::{
    parse_value_prefix_lexed, parse_where_x_greatest_commander_mana_value,
};
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
    if let Some((value, used)) = parse_value_prefix_lexed(tokens)
        && trim_lexed_commas(tokens.get(used..)?).is_empty()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::WhereXIs));
    }
    if let Some(value) = parse_prior_effect_count_value(tokens) {
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
    let (count, used) = parse_value_prefix_lexed(count_tokens)?;
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
    let where_x_value = parse_where_x_value(trim_lexed_commas(value_tokens));
    let typed_binding =
        crate::keyword_static::parse_value_binding_clause(remainder)
            .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs));
    Some(TopCardsViewShape {
        revealed,
        count: where_x_value.or(typed_binding)?,
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

        let tokens = lex_line(
            "Reveal the top X cards of your library, where X is the number of lands sacrificed this way",
            0,
        )
        .unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();
        let Value::PendingPriorEffectMetric(query) = shape.count.unhinted() else {
            panic!("expected typed prior-effect count, got {:?}", shape.count);
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Sacrificed)
        );
        assert_eq!(
            query
                .filter
                .as_ref()
                .map(|filter| filter.card_types.as_slice()),
            Some(&[crate::types::CardType::Land][..])
        );

        let tokens = lex_line("Look at that many cards from the top of your library", 0).unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();
        assert!(!shape.revealed);
        assert_eq!(shape.count, Value::EventValue(EventValueSpec::Amount));

        let tokens = lex_line("Reveal the top X plus one cards of your library", 0).unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();
        assert!(shape.revealed);
        assert_eq!(
            shape.count,
            Value::Add(Box::new(Value::X), Box::new(Value::Fixed(1)))
        );
    }

    #[test]
    fn looked_card_where_x_uses_typed_fixed_plus_party_value() {
        let tokens = lex_line(
            "Look at the top X cards of your library, where X is three plus the number of creatures in your party",
            0,
        )
        .unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();

        assert!(shape.count.has_surface_hint(ValueSurfaceHint::WhereXIs));
        assert_eq!(
            shape.count.unhinted(),
            &Value::Add(
                Box::new(Value::Fixed(3)),
                Box::new(Value::PartySize(crate::target::PlayerFilter::You)),
            )
        );
    }

    #[test]
    fn looked_card_where_x_preserves_battlefield_and_graveyard_sum_terms() {
        let tokens = lex_line(
            "Look at the top X cards of your library, where X is the number of Caves you control plus the number of Cave cards in your graveyard",
            0,
        )
        .unwrap();
        let shape = parse_top_cards_view_shape(&tokens).unwrap();
        let Value::Add(controlled, graveyard) = shape.count.unhinted() else {
            panic!(
                "expected two typed Cave-count terms, got {:#?}",
                shape.count
            );
        };
        let Value::Count(controlled) = controlled.as_ref() else {
            panic!("expected controlled-Cave count, got {controlled:#?}");
        };
        let Value::Count(graveyard) = graveyard.as_ref() else {
            panic!("expected graveyard-Cave count, got {graveyard:#?}");
        };

        assert_eq!(controlled.zone, Some(crate::Zone::Battlefield));
        assert_eq!(
            controlled.controller,
            Some(crate::target::PlayerFilter::You)
        );
        assert!(controlled.subtypes.contains(&crate::Subtype::Cave));
        assert_eq!(graveyard.zone, Some(crate::Zone::Graveyard));
        assert_eq!(graveyard.owner, Some(crate::target::PlayerFilter::You));
        assert!(graveyard.subtypes.contains(&crate::Subtype::Cave));
    }
}
