use winnow::ascii::{dec_int, digit1};
use winnow::error::{ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::literal;

use crate::cards::builders::CardTextError;
use crate::effect::Value;
use crate::{PowerToughness, PtValue};

use super::common::{finish_text_parse, spaced};

pub(crate) fn parse_leaf_power_toughness(input: &mut &str) -> WResult<PowerToughness> {
    (
        spaced(parse_leaf_pt_value),
        literal('/'),
        spaced(parse_leaf_pt_value),
    )
        .map(|(power, _, toughness)| PowerToughness::new(power, toughness))
        .context(StrContext::Label("power/toughness"))
        .context(StrContext::Expected(StrContextValue::Description(
            "power and toughness separated by /",
        )))
        .parse_next(input)
}

pub(crate) fn parse_leaf_power_toughness_complete(
    raw: &str,
) -> Result<PowerToughness, CardTextError> {
    finish_text_parse(raw, parse_leaf_power_toughness, "leaf-power-toughness")
}

pub(crate) fn parse_leaf_unsigned_pt(input: &mut &str) -> WResult<(i32, i32)> {
    (
        parse_unsigned_pt_component,
        literal('/'),
        parse_unsigned_pt_component,
    )
        .map(|(power, _, toughness)| (power, toughness))
        .context(StrContext::Label("unsigned power/toughness"))
        .context(StrContext::Expected(StrContextValue::Description(
            "unsigned numeric power and toughness",
        )))
        .parse_next(input)
}

pub(crate) fn parse_leaf_unsigned_pt_complete(raw: &str) -> Result<(i32, i32), CardTextError> {
    finish_text_parse(raw, parse_leaf_unsigned_pt, "leaf-unsigned-pt")
}

pub(crate) fn parse_leaf_pt_modifier_values(input: &mut &str) -> WResult<(Value, Value)> {
    (
        parse_leaf_pt_modifier_value,
        literal('/'),
        parse_leaf_pt_modifier_value,
    )
        .map(|(power, _, toughness)| (power, toughness))
        .context(StrContext::Label("power/toughness modifier"))
        .context(StrContext::Expected(StrContextValue::Description(
            "signed number or X on each side of /",
        )))
        .parse_next(input)
}

pub(crate) fn parse_leaf_pt_modifier_values_complete(
    raw: &str,
) -> Result<(Value, Value), CardTextError> {
    finish_text_parse(
        raw,
        parse_leaf_pt_modifier_values,
        "leaf-power-toughness-modifier",
    )
}

fn parse_leaf_pt_modifier_value(input: &mut &str) -> WResult<Value> {
    winnow::combinator::alt((
        literal("+x").value(Value::X),
        literal("+X").value(Value::X),
        literal("-x").value(Value::XTimes(-1)),
        literal("-X").value(Value::XTimes(-1)),
        literal("−x").value(Value::XTimes(-1)),
        literal("−X").value(Value::XTimes(-1)),
        winnow::combinator::alt((literal("+0"), literal("-0"), literal("−0")))
            .value(Value::Fixed(0)),
        winnow::combinator::alt((
            literal("x").value(Value::X),
            literal("X").value(Value::X),
            (
                literal('−'),
                digit1.try_map(|digits: &str| digits.parse::<i32>()),
            )
                .map(|(_, value)| Value::Fixed(-value)),
            dec_int.map(Value::Fixed),
        )),
    ))
    .parse_next(input)
}

fn parse_leaf_pt_value(input: &mut &str) -> WResult<PtValue> {
    winnow::combinator::alt((
        (literal("*+"), parse_signed_pt_component).map(|(_, value)| PtValue::StarPlus(value)),
        (parse_signed_pt_component, literal("+*")).map(|(value, _)| PtValue::StarPlus(value)),
        literal("0.5").value(PtValue::Fixed(0)),
        literal(".5").value(PtValue::Fixed(0)),
        literal("*").value(PtValue::Star),
        parse_signed_pt_component.map(PtValue::Fixed),
    ))
    .parse_next(input)
}

fn parse_signed_pt_component(input: &mut &str) -> WResult<i32> {
    dec_int.parse_next(input)
}

fn parse_unsigned_pt_component(input: &mut &str) -> WResult<i32> {
    digit1
        .try_map(|digits: &str| digits.parse::<i32>())
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixed_star_and_legacy_half_values() {
        assert_eq!(
            parse_leaf_power_toughness_complete("*+1/2").unwrap(),
            PowerToughness::new(PtValue::StarPlus(1), PtValue::Fixed(2))
        );
        assert_eq!(
            parse_leaf_power_toughness_complete("1+*/0.5").unwrap(),
            PowerToughness::new(PtValue::StarPlus(1), PtValue::Fixed(0))
        );
        assert_eq!(
            parse_leaf_power_toughness_complete("*/*").unwrap(),
            PowerToughness::new(PtValue::Star, PtValue::Star)
        );
    }

    #[test]
    fn unsigned_parser_rejects_signed_components() {
        assert_eq!(parse_leaf_unsigned_pt_complete("2/3").unwrap(), (2, 3));
        assert!(parse_leaf_unsigned_pt_complete("+2/3").is_err());
        assert!(parse_leaf_unsigned_pt_complete("2/-3").is_err());
    }

    #[test]
    fn modifier_parser_returns_runtime_values() {
        assert_eq!(
            parse_leaf_pt_modifier_values_complete("+2/-1").unwrap(),
            (Value::Fixed(2), Value::Fixed(-1))
        );
        assert_eq!(
            parse_leaf_pt_modifier_values_complete("X/−X").unwrap(),
            (Value::X, Value::XTimes(-1))
        );
    }
}
