use winnow::combinator::{opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::Value;
use crate::runtime_backend::front_end::grammar::{filters, leaf, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenKind, trim_lexed_commas,
};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use ironsmith_core::{EffectMetric, EffectMetricSource};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TapControlledObjectsThenEmptyManaShape {
    pub(crate) filter: ObjectFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EnergyPayAnyDestroyShape {
    pub(crate) energy: Value,
    pub(crate) filter: ObjectFilter,
    pub(crate) minimum_payment: u32,
}

pub(crate) fn parse_energy_pay_any_destroy_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EnergyPayAnyDestroyShape> {
    primitives::parse_all(
        tokens,
        parse_energy_pay_any_destroy,
        "energy pay-any destroy threshold",
    )
    .ok()
}

fn energy_symbol(input: &mut LexStream<'_>) -> WResult<()> {
    winnow::token::any
        .verify(|token: &&OwnedLexToken| {
            token
                .mana_group_inner()
                .is_some_and(|inner| inner.eq_ignore_ascii_case("e"))
        })
        .void()
        .parse_next(input)
}

fn energy_reminder(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::token_kind(TokenKind::LParen).parse_next(input)?;
    primitives::phrase(&["energy", "counters"]).parse_next(input)?;
    primitives::token_kind(TokenKind::RParen)
        .void()
        .parse_next(input)
}

fn parse_energy_pay_any_destroy(input: &mut LexStream<'_>) -> WResult<EnergyPayAnyDestroyShape> {
    primitives::phrase(&["you", "get"]).parse_next(input)?;
    let energy = leaf::parse_leaf_number_or_x_prefix_lexed
        .parse_next(input)?
        .into_value()
        .ok_or_else(|| primitives::backtrack_err("energy amount", "runtime value"))?;
    energy_symbol(input)?;
    opt(energy_reminder).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["then", "you", "may", "pay", "any", "amount", "of"]).parse_next(input)?;
    energy_symbol(input)?;
    primitives::end_of_sentence().parse_next(input)?;

    primitives::phrase(&["destroy", "each"]).parse_next(input)?;
    let filter_tokens = repeat_till(
        1..,
        winnow::token::any.void(),
        peek(primitives::phrase(&["with", "mana", "value"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&[
        "with", "mana", "value", "less", "than", "or", "equal", "to", "the", "amount", "of",
    ])
    .parse_next(input)?;
    energy_symbol(input)?;
    primitives::phrase(&["paid", "this", "way"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let mut filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(filter_tokens),
        false,
    )
    .map_err(|_| primitives::backtrack_err("energy destroy threshold", "object filter"))?;
    filter.zone = Some(Zone::Battlefield);
    filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
        Value::PendingEffectMetric {
            source: EffectMetricSource::Outcome,
            metric: EffectMetric::Count,
        },
    )));
    Ok(EnergyPayAnyDestroyShape {
        energy,
        filter,
        minimum_payment: 0,
    })
}

pub(crate) fn parse_tap_controlled_objects_then_empty_mana_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TapControlledObjectsThenEmptyManaShape> {
    primitives::parse_all(
        tokens,
        parse_tap_controlled_objects_then_empty_mana,
        "tap controlled objects then empty mana",
    )
    .ok()
}

fn parse_tap_controlled_objects_then_empty_mana(
    input: &mut LexStream<'_>,
) -> WResult<TapControlledObjectsThenEmptyManaShape> {
    primitives::phrase(&["tap", "all"]).parse_next(input)?;
    let filter_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["target", "player", "controls"])),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&[
        "target", "player", "controls", "and", "that", "player", "loses",
    ])
    .parse_next(input)?;
    primitives::phrase(&["all", "unspent", "mana"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let mut filter = filters::parse_object_filter_with_grammar_entrypoint_lexed(
        trim_lexed_commas(filter_tokens),
        false,
    )
    .map_err(|_| primitives::backtrack_err("tap controlled objects", "object filter"))?;
    filter.zone = Some(Zone::Battlefield);
    filter.controller = Some(PlayerFilter::target_player());
    Ok(TapControlledObjectsThenEmptyManaShape { filter })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;
    use crate::types::CardType;

    #[test]
    fn parses_arbitrary_controlled_object_filter_before_emptying_mana() {
        let tokens = lex_line(
            "Tap all artifacts target player controls and that player loses all unspent mana.",
            0,
        )
        .unwrap();
        let shape = parse_tap_controlled_objects_then_empty_mana_tokens(&tokens).unwrap();
        assert!(shape.filter.card_types.contains(&CardType::Artifact));
        assert_eq!(shape.filter.controller, Some(PlayerFilter::target_player()));
        assert_eq!(shape.filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parses_energy_payment_destroy_threshold_to_typed_values() {
        let tokens = lex_line(
            "You get X {E} (energy counters), then you may pay any amount of {E}. Destroy each artifact, creature, and enchantment with mana value less than or equal to the amount of {E} paid this way.",
            0,
        )
        .unwrap();
        let shape = parse_energy_pay_any_destroy_tokens(&tokens).unwrap();
        assert_eq!(shape.energy, Value::X);
        assert_eq!(shape.minimum_payment, 0);
        assert_eq!(shape.filter.card_types.len(), 3);
        assert!(matches!(
            shape.filter.mana_value,
            Some(crate::filter::Comparison::LessThanOrEqualExpr(_))
        ));
    }
}
