use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhereXKnownValue<'a> {
    ThisAbilityResolvedThisTurnCount,
    YourLifeTotal,
    HalfYourLifeTotalRoundedUp,
    HalfYourLifeTotalRoundedDown,
    YourSpeed,
    EventDamageAmount,
    OpponentCount,
    PlayersBeingAttacked,
    TargetPlayerLifeTotal,
    TargetPlayersLifeTotalDifference,
    ThatPlayerLifeTotal,
    ThatPlayerSpeed,
    DiscardedCardManaValue,
    RevealedCardsTotalManaValue,
    DraftNotedHighestNumber {
        card_name_tokens: &'a [OwnedLexToken],
    },
}

pub fn parse_where_x_known_value_tokens(tokens: &[OwnedLexToken]) -> Option<WhereXKnownValue<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_where_x_known_value_lexed,
        "known where-X value",
    )
}

fn parse_where_x_known_value_lexed<'a>(input: &mut LexStream<'a>) -> WResult<WhereXKnownValue<'a>> {
    semantic_phrase(&["where", "x", "is"]).parse_next(input)?;
    let value = alt((
        parse_turn_and_player_value,
        parse_life_and_speed_value,
        parse_event_and_attack_value,
        parse_tagged_card_value,
        parse_draft_noted_value,
    ))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(value)
}

fn parse_turn_and_player_value<'a>(input: &mut LexStream<'a>) -> WResult<WhereXKnownValue<'a>> {
    alt((
        (
            opt(semantic_kw("the")),
            semantic_phrase(&[
                "number", "of", "times", "this", "ability", "has", "resolved", "this", "turn",
            ]),
        )
            .value(WhereXKnownValue::ThisAbilityResolvedThisTurnCount),
        (
            opt(semantic_kw("the")),
            semantic_phrase(&["number", "of", "opponents"]),
            opt(semantic_phrase(&["you", "have"])),
        )
            .value(WhereXKnownValue::OpponentCount),
        (
            opt(semantic_kw("the")),
            semantic_phrase(&["number", "of", "players", "being", "attacked"]),
        )
            .value(WhereXKnownValue::PlayersBeingAttacked),
    ))
    .parse_next(input)
}

fn parse_life_and_speed_value<'a>(input: &mut LexStream<'a>) -> WResult<WhereXKnownValue<'a>> {
    alt((
        semantic_phrase(&["your", "life", "total"]).value(WhereXKnownValue::YourLifeTotal),
        semantic_phrase(&["half", "your", "life", "total", "rounded", "down"])
            .value(WhereXKnownValue::HalfYourLifeTotalRoundedDown),
        (
            semantic_phrase(&["half", "your", "life", "total"]),
            opt(semantic_phrase(&["rounded", "up"])),
        )
            .value(WhereXKnownValue::HalfYourLifeTotalRoundedUp),
        semantic_phrase(&["your", "speed"]).value(WhereXKnownValue::YourSpeed),
        (
            alt((semantic_kw("target"), semantic_kw("the"))),
            opt(semantic_kw("target")),
            alt((semantic_kw("player"), semantic_kw("players"))),
            semantic_phrase(&["life", "total"]),
        )
            .value(WhereXKnownValue::TargetPlayerLifeTotal),
        (
            opt(semantic_kw("the")),
            semantic_phrase(&["difference", "between"]),
            opt(semantic_kw("the")),
            alt((semantic_kw("those"), semantic_kw("target"))),
            semantic_phrase(&["players", "life", "totals"]),
        )
            .value(WhereXKnownValue::TargetPlayersLifeTotalDifference),
        (
            semantic_kw("that"),
            alt((semantic_kw("player"), semantic_kw("players"))),
            semantic_phrase(&["life", "total"]),
        )
            .value(WhereXKnownValue::ThatPlayerLifeTotal),
        (
            semantic_kw("that"),
            alt((semantic_kw("player"), semantic_kw("players"))),
            semantic_kw("speed"),
        )
            .value(WhereXKnownValue::ThatPlayerSpeed),
    ))
    .parse_next(input)
}

fn parse_event_and_attack_value<'a>(input: &mut LexStream<'a>) -> WResult<WhereXKnownValue<'a>> {
    alt((
        (
            opt(semantic_kw("the")),
            semantic_phrase(&[
                "amount", "of", "damage", "it", "dealt", "to", "that", "player",
            ]),
        )
            .value(WhereXKnownValue::EventDamageAmount),
        (
            opt(semantic_kw("the")),
            semantic_phrase(&["number", "of", "players", "being", "attacked"]),
        )
            .value(WhereXKnownValue::PlayersBeingAttacked),
    ))
    .parse_next(input)
}

fn parse_tagged_card_value<'a>(input: &mut LexStream<'a>) -> WResult<WhereXKnownValue<'a>> {
    alt((
        (
            opt(semantic_kw("the")),
            semantic_kw("discarded"),
            alt((semantic_kw("card"), semantic_kw("cards"))),
            semantic_phrase(&["mana", "value"]),
        )
            .value(WhereXKnownValue::DiscardedCardManaValue),
        (
            opt(semantic_kw("the")),
            semantic_phrase(&["total", "mana", "value", "of"]),
            opt(semantic_kw("all")),
            semantic_phrase(&["cards", "revealed", "this", "way"]),
        )
            .value(WhereXKnownValue::RevealedCardsTotalManaValue),
    ))
    .parse_next(input)
}

fn parse_draft_noted_value<'a>(input: &mut LexStream<'a>) -> WResult<WhereXKnownValue<'a>> {
    opt(semantic_kw("the")).parse_next(input)?;
    semantic_phrase(&["highest", "number", "you", "noted", "for", "cards", "named"])
        .parse_next(input)?;
    let card_name_tokens = take_nonempty_semantic_body(input)?;
    Ok(WhereXKnownValue::DraftNotedHighestNumber { card_name_tokens })
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., semantic_noise),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| token.parser_word_pieces().is_empty())
        .void()
        .parse_next(input)
}

fn semantic_finish<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

fn take_nonempty_semantic_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let body = repeat_till(1.., any.void(), peek(semantic_finish))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    semantic_finish(input)?;
    let body = trim_lexed_commas(body);
    if body.is_empty() {
        return Err(primitives::backtrack_err(
            "where-X semantic body",
            "non-empty value body",
        ));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_known_player_and_tagged_values() {
        let tokens = lex_line("where X is half your life total, rounded down.", 0).unwrap();
        assert_eq!(
            parse_where_x_known_value_tokens(&tokens),
            Some(WhereXKnownValue::HalfYourLifeTotalRoundedDown)
        );

        let tokens = lex_line(
            "where X is the difference between those players' life totals.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_where_x_known_value_tokens(&tokens),
            Some(WhereXKnownValue::TargetPlayersLifeTotalDifference)
        );

        let tokens = lex_line(
            "where X is the total mana value of all cards revealed this way.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_where_x_known_value_tokens(&tokens),
            Some(WhereXKnownValue::RevealedCardsTotalManaValue)
        );
    }

    #[test]
    fn captures_draft_card_name() {
        let tokens = lex_line(
            "where X is the highest number you noted for cards named Arc Lightning.",
            0,
        )
        .unwrap();
        let Some(WhereXKnownValue::DraftNotedHighestNumber { card_name_tokens }) =
            parse_where_x_known_value_tokens(&tokens)
        else {
            panic!("expected draft value")
        };
        assert_eq!(render_token_slice(card_name_tokens), "Arc Lightning");
    }
}
