use super::*;

use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::{any, take_till};

#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayerCreaturesDamageShape {
    pub amount: Value,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompoundBuffUnblockableShape<'a> {
    pub buff_tokens: &'a [OwnedLexToken],
    pub subject_tokens: &'a [OwnedLexToken],
    pub unblockable_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantBlockedBasePowerToughnessShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub power: Value,
    pub toughness: Value,
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

pub fn parse_each_player_creatures_damage_tokens(
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

pub fn parse_compound_buff_unblockable_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CompoundBuffUnblockableShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_compound_buff_unblockable_lexed,
        "compound-buff-unblockable",
    )
    .ok()
}

fn parse_cant_be_blocked(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("can't"), primitives::kw("cant")))
        .void()
        .parse_next(input)?;
    primitives::phrase(&["be", "blocked"])
        .void()
        .parse_next(input)
}

fn parse_cant_blocked_base_power_toughness_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantBlockedBasePowerToughnessShape<'a>> {
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_cant_be_blocked))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    parse_cant_be_blocked.parse_next(input)?;
    alt((
        primitives::phrase(&["this", "turn"]),
        primitives::phrase(&["until", "end", "of", "turn"]),
        primitives::phrase(&["until", "the", "end", "of", "turn"]),
    ))
    .void()
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("has"), primitives::kw("have"))).parse_next(input)?;
    primitives::phrase(&["base", "power", "and", "toughness"]).parse_next(input)?;
    let modifier = primitives::word_parser_text.parse_next(input)?;
    let (power, toughness) = super::super::leaf::parse_leaf_pt_modifier_values_complete(modifier)
        .map_err(|_| {
        primitives::backtrack_err(
            "cant-be-blocked base power/toughness",
            "power/toughness value",
        )
    })?;
    alt((
        primitives::phrase(&["until", "end", "of", "turn"]),
        primitives::phrase(&["until", "the", "end", "of", "turn"]),
    ))
    .void()
    .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;

    Ok(CantBlockedBasePowerToughnessShape {
        subject_tokens,
        power,
        toughness,
    })
}

pub fn parse_cant_blocked_base_power_toughness_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CantBlockedBasePowerToughnessShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_cant_blocked_base_power_toughness_lexed,
        "cant-be-blocked base-power/toughness",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, render_token_slice};

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

    #[test]
    fn parses_cant_be_blocked_then_base_power_toughness_shape() {
        let tokens = lex_line(
            "That creature can't be blocked this turn and has base power and toughness 1/1 until end of turn.",
            0,
        )
        .unwrap();
        let parsed = parse_cant_blocked_base_power_toughness_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(parsed.subject_tokens), "That creature");
        assert_eq!(parsed.power, Value::Fixed(1));
        assert_eq!(parsed.toughness, Value::Fixed(1));
    }
}
