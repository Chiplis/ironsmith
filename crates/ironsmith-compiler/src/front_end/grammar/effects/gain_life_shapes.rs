use super::*;

use winnow::combinator::{alt, eof, peek, repeat, repeat_till};
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GainLifeEqualPowerShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GainXPlusLifeShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) bonus: u32,
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
}

fn gain_word<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("gain"), primitives::kw("gains")))
        .void()
        .parse_next(input)
}

fn parse_gain_life_equal_power_lexed<'a>(
    input: &mut LexStream<'a>,
) -> winnow::error::ModalResult<GainLifeEqualPowerShape<'a>> {
    let subject_tokens = repeat_till(0.., any.void(), peek(gain_word))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    gain_word.parse_next(input)?;
    primitives::phrase(&["life", "equal", "to"]).parse_next(input)?;
    primitives::phrase(&["its", "power"]).parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any.void())
        .take()
        .verify(|tokens: &&[OwnedLexToken]| {
            tokens
                .iter()
                .all(|token| token.parser_word_pieces().is_empty())
        })
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(GainLifeEqualPowerShape { subject_tokens })
}

pub(crate) fn parse_gain_life_equal_power_tokens(
    tokens: &[OwnedLexToken],
) -> Option<GainLifeEqualPowerShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_gain_life_equal_power_lexed,
        "gain-life-equal-power",
    )
    .ok()
}

fn subject_is_not_negated(subject_tokens: &[OwnedLexToken]) -> bool {
    !subject_tokens.last().is_some_and(|token| {
        token.is_any_word(&[
            "cant", "can't", "cannot", "doesnt", "doesn't", "don't", "dont",
        ])
    })
}

fn parse_gain_x_plus_life_lexed<'a>(
    input: &mut LexStream<'a>,
) -> winnow::error::ModalResult<GainXPlusLifeShape<'a>> {
    let subject_tokens = repeat_till(0.., any.void(), peek(gain_word))
        .map(|((), _)| ())
        .take()
        .verify(|tokens: &&[OwnedLexToken]| subject_is_not_negated(tokens))
        .parse_next(input)?;
    gain_word.parse_next(input)?;
    primitives::phrase(&["x", "plus"]).parse_next(input)?;
    let bonus = super::super::leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::kw("life").parse_next(input)?;
    let trailing_tokens = repeat::<_, _, (), _, _>(0.., any.void())
        .take()
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(GainXPlusLifeShape {
        subject_tokens,
        bonus,
        trailing_tokens,
    })
}

pub(crate) fn parse_gain_x_plus_life_tokens(
    tokens: &[OwnedLexToken],
) -> Option<GainXPlusLifeShape<'_>> {
    primitives::parse_all(tokens, parse_gain_x_plus_life_lexed, "gain-x-plus-life").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, render_token_slice};

    #[test]
    fn parses_equal_power_life_shape() {
        let tokens = lex_line("You gain life equal to its power.", 0).unwrap();
        let shape = parse_gain_life_equal_power_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(shape.subject_tokens), "You");
    }

    #[test]
    fn rejects_equal_power_found_inside_a_later_action() {
        let tokens = lex_line(
            "You gain life equal to that card's toughness, lose life equal to its power, then put it into your hand.",
            0,
        )
        .unwrap();

        assert!(parse_gain_life_equal_power_tokens(&tokens).is_none());
    }

    #[test]
    fn parses_x_plus_life_shape() {
        let tokens = lex_line("You gain X plus 3 life, where X is two.", 0).unwrap();
        let shape = parse_gain_x_plus_life_tokens(&tokens).unwrap();
        assert_eq!(shape.bonus, 3);
        assert_eq!(render_token_slice(shape.subject_tokens), "You");
        assert_eq!(
            render_token_slice(shape.trailing_tokens),
            ", where X is two."
        );
    }
}
