use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::effect::Value;
use crate::mana::ManaCost;
use crate::target::PlayerFilter;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, render_token_slice};
use super::super::leaf;
use super::super::primitives;
use super::ActivationCostSegmentCst;

pub(crate) fn parse_bare_symbol_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivationCostSegmentCst> {
    primitives::parse_all(
        tokens,
        parse_bare_symbol_segment_lexed,
        "activation-bare-symbol-segment",
    )
    .ok()
}

pub(crate) fn parse_pay_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(tokens, parse_pay_segment_lexed, "pay-cost")
}

pub(crate) fn parse_mill_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(tokens, parse_mill_segment_lexed, "mill")
}

pub(crate) fn parse_behold_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(tokens, parse_behold_segment_lexed, "behold")
}

pub(crate) fn parse_blight_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    parse_simple_segment(tokens, parse_blight_segment_lexed, "blight")
}

fn parse_simple_segment<'a>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<
        LexStream<'a>,
        ActivationCostSegmentCst,
        winnow::error::ErrMode<winnow::error::ContextError>,
    >,
    label: &str,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    primitives::parse_all(tokens, parser, label).map_err(|_| {
        CardTextError::ParseError(format!(
            "rewrite {label} parser does not yet support '{}'",
            render_token_slice(tokens).trim().to_ascii_lowercase()
        ))
    })
}

fn parse_bare_symbol_segment_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentCst> {
    let tokens: Vec<&OwnedLexToken> = repeat(1.., any).parse_next(input)?;
    if tokens.len() == 1 {
        if is_tap_activation_symbol_token(tokens[0]) {
            return Ok(ActivationCostSegmentCst::Tap);
        }
        if is_untap_symbol_token(tokens[0]) {
            return Ok(ActivationCostSegmentCst::Untap);
        }
    }

    if tokens.iter().all(|token| is_energy_symbol_token(token)) {
        return u32::try_from(tokens.len())
            .map(ActivationCostSegmentCst::Energy)
            .map_err(|_| primitives::backtrack_err("energy cost", "representable energy count"));
    }
    if tokens
        .iter()
        .any(|token| is_reserved_activation_symbol_token(token))
    {
        return Err(primitives::backtrack_err(
            "activation symbol",
            "unmixed tap, untap, or energy symbol",
        ));
    }

    let mut pips = Vec::new();
    for token in tokens {
        let parsed = leaf::parse_leaf_surface_mana_pip_token(token)
            .ok_or_else(|| primitives::backtrack_err("mana cost", "one or more mana symbols"))?;
        pips.push(parsed.into_pip());
    }
    Ok(ActivationCostSegmentCst::Mana(ManaCost::from_pips(pips)))
}

fn parse_pay_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("pay").parse_next(input)?;
    alt((
        parse_life_payment,
        parse_counted_energy_payment,
        parse_energy_payment,
        parse_bare_symbol_segment_lexed,
    ))
    .parse_next(input)
}

fn parse_life_payment<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    let amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    alt((primitives::kw("life"), primitives::kw("lives"))).parse_next(input)?;
    let per_card_in_hand = opt(primitives::phrase(&[
        "for", "each", "card", "in", "your", "hand",
    ]))
    .parse_next(input)?
    .is_some();
    eof.parse_next(input)?;
    let value = if per_card_in_hand {
        let cards = Value::CardsInHand(PlayerFilter::You);
        if amount == 1 {
            cards
        } else {
            Value::Scaled(Box::new(cards), amount as i32)
        }
    } else {
        Value::Fixed(amount as i32)
    };
    Ok(ActivationCostSegmentCst::Life(value))
}

fn parse_counted_energy_payment<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentCst> {
    let amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    parse_energy_symbol.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Energy(amount))
}

fn parse_energy_payment<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    let symbols: Vec<()> = repeat(1.., parse_energy_symbol).parse_next(input)?;
    eof.parse_next(input)?;
    let count = u32::try_from(symbols.len())
        .map_err(|_| primitives::backtrack_err("energy payment", "representable energy count"))?;
    Ok(ActivationCostSegmentCst::Energy(count))
}

fn parse_energy_symbol<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| is_energy_symbol_token(token))
        .void()
        .parse_next(input)
}

fn parse_mill_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("mill").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Mill(count))
}

fn parse_behold_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("behold").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    let subtype_word = primitives::word_parser_text.parse_next(input)?;
    let subtype = leaf::parse_leaf_subtype_flexible_complete(subtype_word)
        .map_err(|_| primitives::backtrack_err("behold subtype", "known subtype"))?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Behold { subtype, count })
}

fn parse_blight_segment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ActivationCostSegmentCst> {
    primitives::kw("blight").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ActivationCostSegmentCst::Blight { count })
}

fn is_energy_symbol_token(token: &OwnedLexToken) -> bool {
    match token.kind {
        TokenKind::ManaGroup => token.slice.eq_ignore_ascii_case("{e}"),
        TokenKind::Word | TokenKind::Number => token
            .as_word()
            .is_some_and(|word| word.eq_ignore_ascii_case("e")),
        _ => false,
    }
}

pub(crate) fn is_tap_activation_symbol_token(token: &OwnedLexToken) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case("t"))
        || token.slice.eq_ignore_ascii_case("{t}")
}

fn is_untap_symbol_token(token: &OwnedLexToken) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case("q"))
        || token.slice.eq_ignore_ascii_case("{q}")
}

fn is_reserved_activation_symbol_token(token: &OwnedLexToken) -> bool {
    is_energy_symbol_token(token)
        || is_tap_activation_symbol_token(token)
        || is_untap_symbol_token(token)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::super::{ActivationCostSegmentKind, parse_activation_cost_segment_kind_tokens};
    use super::*;

    fn parse(raw: &str) -> ActivationCostSegmentCst {
        let tokens = lex_line(raw, 0).unwrap();
        match parse_activation_cost_segment_kind_tokens(&tokens) {
            ActivationCostSegmentKind::Pay => parse_pay_segment_tokens(&tokens).unwrap(),
            ActivationCostSegmentKind::Mill => parse_mill_segment_tokens(&tokens).unwrap(),
            ActivationCostSegmentKind::Behold => parse_behold_segment_tokens(&tokens).unwrap(),
            ActivationCostSegmentKind::Blight => parse_blight_segment_tokens(&tokens).unwrap(),
            _ => parse_bare_symbol_segment_tokens(&tokens).unwrap(),
        }
    }

    #[test]
    fn simple_segments_return_typed_cst() {
        assert_eq!(
            parse("pay 2 life"),
            ActivationCostSegmentCst::Life(Value::Fixed(2))
        );
        assert_eq!(
            parse("pay 1 life for each card in your hand"),
            ActivationCostSegmentCst::Life(Value::CardsInHand(PlayerFilter::You))
        );
        assert_eq!(parse("pay {e}"), ActivationCostSegmentCst::Energy(1));
        assert_eq!(parse("mill three cards"), ActivationCostSegmentCst::Mill(3));
        assert_eq!(
            parse("behold a goblin"),
            ActivationCostSegmentCst::Behold {
                subtype: crate::types::Subtype::Goblin,
                count: 1,
            }
        );
        assert_eq!(
            parse("blight 2"),
            ActivationCostSegmentCst::Blight { count: 2 }
        );
    }
}
