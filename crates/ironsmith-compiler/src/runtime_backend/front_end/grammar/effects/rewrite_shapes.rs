use super::*;

use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::{any, take_till};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EachPlayerCreaturesDamageShape {
    pub(crate) amount: Value,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CompoundBuffUnblockableShape<'a> {
    pub(crate) buff_tokens: &'a [OwnedLexToken],
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) unblockable_tail_tokens: &'a [OwnedLexToken],
}

fn parse_each_player_creatures_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EachPlayerCreaturesDamageShape> {
    let _: &[OwnedLexToken] = take_till(0.., |token: &OwnedLexToken| {
        token.is_any_word(&["deal", "deals"])
    })
    .parse_next(input)?;
    alt((primitives::kw("deal"), primitives::kw("deals")))
        .void()
        .parse_next(input)?;
    let amount = super::super::leaf::parse_leaf_modal_value_token.parse_next(input)?;
    primitives::phrase(&["damage", "to", "each", "player", "and", "each"]).parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .void()
        .parse_next(input)?;
    alt((
        primitives::phrase(&["they", "control"]),
        primitives::phrase(&["that", "player", "controls"]),
    ))
    .void()
    .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(EachPlayerCreaturesDamageShape { amount })
}

pub(crate) fn parse_each_player_creatures_damage_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerCreaturesDamageShape> {
    primitives::parse_all(
        tokens,
        parse_each_player_creatures_damage_lexed,
        "each-player-creatures-damage",
    )
    .ok()
}

fn parse_unblockable_conjunction_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("can't"), primitives::kw("cant")))
        .void()
        .parse_next(input)?;
    primitives::phrase(&["be", "blocked"]).parse_next(input)
}

fn parse_compound_buff_unblockable_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CompoundBuffUnblockableShape<'a>> {
    let (subject_tokens, buff_tokens) = (
        repeat_till(1.., any.void(), peek(primitives::kw("gets")))
            .map(|((), _)| ())
            .take(),
        primitives::kw("gets"),
        repeat_till(1.., any.void(), peek(parse_unblockable_conjunction_lexed)).map(|((), _)| ()),
    )
        .map(|(subject_tokens, _, ())| subject_tokens)
        .with_taken()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let unblockable_tail_tokens = (
        alt((primitives::kw("can't"), primitives::kw("cant"))).void(),
        primitives::phrase(&["be", "blocked"]),
    )
        .take()
        .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;

    Ok(CompoundBuffUnblockableShape {
        buff_tokens,
        subject_tokens,
        unblockable_tail_tokens,
    })
}

pub(crate) fn parse_compound_buff_unblockable_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CompoundBuffUnblockableShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_compound_buff_unblockable_lexed,
        "compound-buff-unblockable",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, render_token_slice};

    #[test]
    fn parses_each_player_and_controlled_creatures_damage() {
        let tokens = lex_line(
            "This creature deals X damage to each player and each creature they control.",
            0,
        )
        .unwrap();
        let parsed = parse_each_player_creatures_damage_tokens(&tokens).unwrap();
        assert_eq!(parsed.amount, Value::X);
    }

    #[test]
    fn splits_compound_buff_and_unblockable_shape() {
        let tokens = lex_line("Target creature gets +2/+2 and can't be blocked.", 0).unwrap();
        let parsed = parse_compound_buff_unblockable_tokens(&tokens).unwrap();
        assert_eq!(
            render_token_slice(parsed.buff_tokens),
            "Target creature gets +2/+2"
        );
        assert_eq!(render_token_slice(parsed.subject_tokens), "Target creature");
        assert_eq!(
            render_token_slice(parsed.unblockable_tail_tokens),
            "can't be blocked"
        );
    }
}
