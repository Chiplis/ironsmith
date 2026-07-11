use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::object::CounterType;

use super::super::super::lexer::{LexStream, OwnedLexToken, render_token_slice, trim_lexed_commas};
use super::super::{filters, leaf, primitives, static_keyword_line_shapes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PregameBeginOnBattlefieldSpec {
    pub(crate) require_not_starting_player: bool,
    pub(crate) counters: Vec<(CounterType, u32)>,
    pub(crate) exile_cards_from_hand: usize,
}

pub(crate) fn parse_pregame_begin_on_battlefield_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<PregameBeginOnBattlefieldSpec>, CardTextError> {
    if primitives::parse_prefix(tokens, parse_pregame_intro_lexed).is_none() {
        return Ok(None);
    }
    let shape = static_keyword_line_shapes::parse_pregame_battlefield_shape(tokens)
        .ok_or_else(|| pregame_error(tokens, "missing battlefield destination"))?;
    let modifier_end = shape
        .if_you_do
        .map(|span| span.start)
        .unwrap_or(tokens.len());
    let modifier_tokens = trim_lexed_commas(
        tokens
            .get(shape.battlefield.end..modifier_end)
            .unwrap_or_default(),
    );
    let has_no_modifier = modifier_tokens.is_empty()
        || primitives::parse_all(
            modifier_tokens,
            primitives::sentence_end(),
            "pregame battlefield terminator",
        )
        .is_ok();
    let counters = if has_no_modifier {
        Vec::new()
    } else {
        let parsed = primitives::parse_all(
            modifier_tokens,
            parse_pregame_counter_modifier_lexed,
            "pregame counter modifier",
        )
        .map_err(|_| pregame_error(tokens, "unsupported battlefield modifier"))?;
        vec![parsed]
    };

    let exile_cards_from_hand = if let Some(if_you_do) = shape.if_you_do {
        let exile_tokens = tokens.get(if_you_do.end..).unwrap_or_default();
        primitives::parse_all(
            exile_tokens,
            parse_pregame_exile_followup_lexed,
            "pregame exile follow-up",
        )
        .map_err(|_| pregame_error(tokens, "unsupported exile follow-up"))? as usize
    } else {
        0
    };

    let require_not_starting_player = primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["youre", "not", "the", "starting", "player"]),
            primitives::phrase(&["you're", "not", "the", "starting", "player"]),
            primitives::phrase(&["you", "are", "not", "the", "starting", "player"]),
        ))
        .void()
    })
    .is_some();

    Ok(Some(PregameBeginOnBattlefieldSpec {
        require_not_starting_player,
        counters,
        exile_cards_from_hand,
    }))
}

pub(crate) fn parse_can_block_additional_creature_tokens(
    tokens: &[OwnedLexToken],
) -> Option<usize> {
    primitives::parse_all(
        tokens,
        parse_can_block_additional_creature_lexed,
        "can block additional creature",
    )
    .ok()
}

fn parse_pregame_intro_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("if").parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["is", "in", "your", "opening", "hand"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["is", "in", "your", "opening", "hand"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::phrase(&[
            "you", "may", "begin", "the", "game", "with",
        ])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["you", "may", "begin", "the", "game", "with"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["on", "the", "battlefield"])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["on", "the", "battlefield"])
        .void()
        .parse_next(input)
}

fn parse_pregame_counter_modifier_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(CounterType, u32)> {
    primitives::kw("with").parse_next(input)?;
    let count = parse_one_or_number(input)?;
    let (_, counter_type_tokens) = (
        repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
        )
        .void(),
        alt((primitives::kw("counter"), primitives::kw("counters"))),
    )
        .with_taken()
        .parse_next(input)?;
    primitives::phrase(&["on", "it"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let counter_type =
        filters::parse_counter_type_from_tokens(counter_type_tokens).ok_or_else(|| {
            primitives::backtrack_err("pregame counter modifier", "known counter type")
        })?;
    Ok((counter_type, count))
}

fn parse_pregame_exile_followup_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("exile").parse_next(input)?;
    let count = parse_one_or_number(input)?;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    primitives::phrase(&["from", "your", "hand"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(count)
}

fn parse_one_or_number<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    alt((
        alt((primitives::kw("a"), primitives::kw("an"))).value(1),
        leaf::parse_leaf_number_prefix_lexed,
    ))
    .parse_next(input)
}

fn parse_can_block_additional_creature_lexed<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    primitives::phrase(&["this", "creature", "can", "block"]).parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    let count = opt(leaf::parse_leaf_number_prefix_lexed)
        .map(|count| count.unwrap_or(1) as usize)
        .parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    opt(alt((
        primitives::phrase(&["each", "combat"]),
        primitives::phrase(&["this", "turn"]),
    )))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(count)
}

fn pregame_error(tokens: &[OwnedLexToken], message: &str) -> CardTextError {
    CardTextError::ParseError(format!(
        "{message} in pregame line (clause: '{}')",
        render_token_slice(tokens)
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_pregame_variants() {
        let tokens = lex_line(
            "If this card is in your opening hand, you may begin the game with it on the battlefield.",
            0,
        )
        .unwrap();
        let spec = parse_pregame_begin_on_battlefield_tokens(&tokens)
            .unwrap()
            .unwrap();
        assert!(spec.counters.is_empty());

        let tokens = lex_line(
            "If this card is in your opening hand and you're not the starting player, you may begin the game with it on the battlefield with a luck counter on it. If you do, exile a card from your hand.",
            0,
        )
        .unwrap();
        let spec = parse_pregame_begin_on_battlefield_tokens(&tokens)
            .unwrap()
            .unwrap();
        assert!(spec.require_not_starting_player);
        assert_eq!(spec.exile_cards_from_hand, 1);
        assert_eq!(spec.counters.len(), 1);
    }

    #[test]
    fn parses_additional_block_count() {
        let tokens = lex_line(
            "This creature can block an additional two creatures each combat.",
            0,
        )
        .unwrap();
        assert_eq!(parse_can_block_additional_creature_tokens(&tokens), Some(2));
    }
}
