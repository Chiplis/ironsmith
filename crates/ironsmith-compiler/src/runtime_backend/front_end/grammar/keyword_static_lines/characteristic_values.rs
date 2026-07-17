use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharacteristicSourceValueKind {
    Power,
    Toughness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharacteristicRelativeValue {
    Same,
    Plus(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharacteristicAggregateKind {
    BasicLandTypes,
    CreatureTypes,
    Colors,
    DistinctNames,
    DifferentPowers,
    CardTypes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacteristicAggregatePrefix<'a> {
    pub(crate) kind: CharacteristicAggregateKind,
    pub(crate) scope_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_characteristic_shared_value_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let (power_toughness_start, _, _) = primitives::find_prefix(tokens, || {
        primitives::phrase(&["power", "and", "toughness"])
    })?;
    if power_toughness_start > 0 && tokens[power_toughness_start - 1].is_word("with") {
        return None;
    }

    primitives::parse_all(
        tokens,
        parse_characteristic_shared_value_tail_lexed,
        "shared characteristic P/T value",
    )
    .ok()
}

pub(crate) fn parse_characteristic_source_value_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CharacteristicSourceValueKind> {
    primitives::parse_all(
        tokens,
        parse_characteristic_source_value_lexed,
        "characteristic source value",
    )
    .ok()
}

pub(crate) fn parse_characteristic_relative_value_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CharacteristicRelativeValue> {
    primitives::parse_all(
        tokens,
        parse_characteristic_relative_value_lexed,
        "relative characteristic value",
    )
    .ok()
}

pub(crate) fn strip_characteristic_number_of_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> &[OwnedLexToken] {
    primitives::parse_prefix(tokens, parse_characteristic_number_of_prefix_lexed)
        .map(|(_, rest)| rest)
        .unwrap_or(tokens)
}

pub(crate) fn parse_characteristic_aggregate_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CharacteristicAggregatePrefix<'_>> {
    primitives::parse_prefix(tokens, parse_characteristic_aggregate_prefix_lexed).map(
        |(kind, rest)| CharacteristicAggregatePrefix {
            kind,
            scope_tokens: trim_lexed_commas(rest),
        },
    )
}

pub(crate) fn characteristic_tokens_have_card_types_among_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        (
            alt((primitives::kw("type"), primitives::kw("types"))),
            primitives::kw("among"),
        )
            .void()
    })
    .is_some()
        && primitives::find_prefix(tokens, || primitives::kw("card").void()).is_some()
}

pub(crate) fn parse_iterated_mana_value_base_pt_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_iterated_mana_value_base_pt_tail_lexed,
        "iterated mana-value base P/T tail",
    )
    .is_ok()
}

fn parse_characteristic_shared_value_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::phrase(&["power", "and", "toughness"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["power", "and", "toughness"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::phrase(&["equal", "to"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["equal", "to"]).parse_next(input)?;
    let value_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            (primitives::kw("respectively"), primitives::sentence_end()).void(),
            primitives::sentence_end(),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    opt(primitives::kw("respectively")).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(trim_lexed_commas(value_tokens))
}

fn parse_characteristic_source_value_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CharacteristicSourceValueKind> {
    opt(alt((
        primitives::kw("source"),
        primitives::kw("sources"),
        primitives::kw("its"),
        primitives::kw("this"),
        primitives::kw("thiss"),
    )))
    .parse_next(input)?;
    opt(primitives::kw("creature")).parse_next(input)?;
    let kind = alt((
        primitives::kw("power").value(CharacteristicSourceValueKind::Power),
        primitives::kw("toughness").value(CharacteristicSourceValueKind::Toughness),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(kind)
}

fn parse_characteristic_relative_value_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CharacteristicRelativeValue> {
    primitives::phrase(&["that", "number"]).parse_next(input)?;
    let amount = opt((primitives::kw("plus"), leaf::parse_leaf_number_prefix_lexed))
        .map(|parsed| parsed.map(|(_, amount)| amount))
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(amount.map_or(
        CharacteristicRelativeValue::Same,
        CharacteristicRelativeValue::Plus,
    ))
}

fn parse_characteristic_number_of_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            primitives::kw("a"),
            primitives::kw("an"),
            primitives::kw("the"),
        )),
    )
    .parse_next(input)?;
    primitives::phrase(&["number", "of"])
        .void()
        .parse_next(input)
}

fn parse_characteristic_aggregate_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CharacteristicAggregateKind> {
    alt((
        (
            primitives::kw("basic"),
            primitives::kw("land"),
            alt((primitives::kw("type"), primitives::kw("types"))),
            primitives::kw("among"),
        )
            .value(CharacteristicAggregateKind::BasicLandTypes),
        (
            primitives::kw("creature"),
            alt((primitives::kw("type"), primitives::kw("types"))),
            primitives::kw("among"),
        )
            .value(CharacteristicAggregateKind::CreatureTypes),
        (
            alt((primitives::kw("color"), primitives::kw("colors"))),
            primitives::kw("among"),
        )
            .value(CharacteristicAggregateKind::Colors),
        (primitives::kw("differently"), primitives::kw("named"))
            .value(CharacteristicAggregateKind::DistinctNames),
        (
            primitives::kw("different"),
            alt((
                primitives::kw("powers").void(),
                (primitives::kw("power"), opt(primitives::kw("values"))).void(),
            )),
            primitives::kw("among"),
        )
            .value(CharacteristicAggregateKind::DifferentPowers),
        (
            primitives::kw("card"),
            alt((primitives::kw("type"), primitives::kw("types"))),
            primitives::kw("among"),
        )
            .value(CharacteristicAggregateKind::CardTypes),
    ))
    .parse_next(input)
}

fn parse_iterated_mana_value_base_pt_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("base").parse_next(input)?;
    primitives::kw("power").parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    opt(primitives::kw("base")).parse_next(input)?;
    primitives::kw("toughness").parse_next(input)?;
    primitives::phrase(&["each", "equal", "to"]).parse_next(input)?;
    alt((
        primitives::phrase(&["its", "mana", "value"]),
        primitives::phrase(&["their", "mana", "value"]),
        (
            primitives::kw("that"),
            alt((
                primitives::kw("permanent"),
                primitives::kw("permanents"),
                primitives::kw("object"),
                primitives::kw("objects"),
            )),
            opt(primitives::kw("s")),
            primitives::phrase(&["mana", "value"]),
        )
            .void(),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_characteristic_value_shapes() {
        let tokens = lex_line("that number plus two", 0).unwrap();
        assert_eq!(
            parse_characteristic_relative_value_tokens(&tokens),
            Some(CharacteristicRelativeValue::Plus(2))
        );
        let tokens = lex_line("the number of card types among cards in your graveyard", 0).unwrap();
        let stripped = strip_characteristic_number_of_prefix_tokens(&tokens);
        assert_eq!(
            parse_characteristic_aggregate_prefix_tokens(stripped).map(|spec| spec.kind),
            Some(CharacteristicAggregateKind::CardTypes)
        );

        let tokens = lex_line("the number of differently named lands you control", 0).unwrap();
        let stripped = strip_characteristic_number_of_prefix_tokens(&tokens);
        let parsed = parse_characteristic_aggregate_prefix_tokens(stripped)
            .expect("differently named characteristic aggregate should parse");
        assert_eq!(parsed.kind, CharacteristicAggregateKind::DistinctNames);
        assert_eq!(
            parsed
                .scope_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect::<Vec<_>>(),
            ["lands", "you", "control"]
        );
    }
}
