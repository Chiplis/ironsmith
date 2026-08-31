use super::*;

use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForEachPlayerKind {
    Opponent,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForEachNoControlLoseGameShape<'a> {
    pub player_kind: ForEachPlayerKind,
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterSpellConditionalKind {
    IfKicked,
    SecondCastThisTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterSpellConditionalShape<'a> {
    pub kind: CounterSpellConditionalKind,
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExileGreatestPowerCreatureShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

fn for_each_player_kind<'a>(input: &mut LexStream<'a>) -> WResult<ForEachPlayerKind> {
    alt((
        alt((
            primitives::phrase(&["for", "each", "opponent"]),
            primitives::phrase(&["for", "each", "opponents"]),
            primitives::phrase(&["each", "opponent"]),
            primitives::phrase(&["each", "opponents"]),
        ))
        .value(ForEachPlayerKind::Opponent),
        alt((
            primitives::phrase(&["for", "each", "player"]),
            primitives::phrase(&["for", "each", "players"]),
            primitives::phrase(&["each", "player"]),
            primitives::phrase(&["each", "players"]),
        ))
        .value(ForEachPlayerKind::Player),
    ))
    .parse_next(input)
}

fn negated_auxiliary<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("doesn't").void(),
            primitives::kw("doesnt").void(),
            primitives::kw("don't").void(),
            primitives::kw("dont").void(),
            primitives::kw("cannot").void(),
            primitives::kw("can't").void(),
            primitives::kw("cant").void(),
        )),
        alt((
            primitives::phrase(&["does", "not"]),
            primitives::phrase(&["do", "not"]),
            primitives::phrase(&["can", "not"]),
        )),
        primitives::phrase(&["doesn", "t"]),
        primitives::phrase(&["don", "t"]),
        primitives::phrase(&["can", "t"]),
    ))
    .parse_next(input)
}

fn lose_game_phrase<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["loses", "the", "game"]),
        primitives::phrase(&["lose", "the", "game"]),
    ))
    .parse_next(input)
}

fn parse_for_each_no_control_lose_game_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ForEachNoControlLoseGameShape<'a>> {
    opt(primitives::kw("then")).parse_next(input)?;
    let player_kind = for_each_player_kind.parse_next(input)?;
    primitives::kw("who").parse_next(input)?;
    negated_auxiliary.parse_next(input)?;
    primitives::kw("control").parse_next(input)?;
    let filter_tokens = repeat_till(1.., any.void(), peek(lose_game_phrase))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    lose_game_phrase.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ForEachNoControlLoseGameShape {
        player_kind,
        filter_tokens,
    })
}

pub fn parse_for_each_no_control_lose_game_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ForEachNoControlLoseGameShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_for_each_no_control_lose_game_lexed,
        "for-each-no-control-lose-game",
    )
}

fn second_spell_cast_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["that's", "second", "spell", "cast", "this", "turn"]),
        primitives::phrase(&["that's", "the", "second", "spell", "cast", "this", "turn"]),
        primitives::phrase(&["thats", "second", "spell", "cast", "this", "turn"]),
        primitives::phrase(&["thats", "the", "second", "spell", "cast", "this", "turn"]),
        primitives::phrase(&["that", "s", "second", "spell", "cast", "this", "turn"]),
        primitives::phrase(&[
            "that", "s", "the", "second", "spell", "cast", "this", "turn",
        ]),
    ))
    .parse_next(input)
}

fn parse_counter_spell_conditional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterSpellConditionalShape<'a>> {
    primitives::kw("counter").parse_next(input)?;
    let target_tokens = (primitives::kw("target"), primitives::kw("spell"))
        .take()
        .parse_next(input)?;
    let kind = alt((
        primitives::phrase(&["if", "it", "was", "kicked"])
            .value(CounterSpellConditionalKind::IfKicked),
        second_spell_cast_tail.value(CounterSpellConditionalKind::SecondCastThisTurn),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterSpellConditionalShape {
        kind,
        target_tokens,
    })
}

pub fn parse_counter_spell_conditional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CounterSpellConditionalShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_counter_spell_conditional_lexed,
        "counter-spell-conditional",
    )
}

fn battlefield_phrase<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["on", "the", "battlefield"]),
        primitives::phrase(&["on", "battlefield"]),
    ))
    .parse_next(input)
}

fn parse_exile_greatest_power_creature_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileGreatestPowerCreatureShape<'a>> {
    primitives::kw("exile").parse_next(input)?;
    let target_tokens = (primitives::kw("target"), primitives::kw("creature"))
        .take()
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::phrase(&[
            "greatest",
            "power",
            "among",
            "creatures",
        ])),
    )
    .void()
    .parse_next(input)?;
    primitives::phrase(&["greatest", "power", "among", "creatures"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(battlefield_phrase))
        .void()
        .parse_next(input)?;
    battlefield_phrase.parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(ExileGreatestPowerCreatureShape { target_tokens })
}

pub fn parse_exile_greatest_power_creature_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ExileGreatestPowerCreatureShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_exile_greatest_power_creature_lexed,
        "exile-greatest-power-creature",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, render_token_slice};

    #[test]
    fn parses_counter_spell_conditionals() {
        let kicked = lex_line("Counter target spell if it was kicked.", 0).unwrap();
        assert_eq!(
            parse_counter_spell_conditional_tokens(&kicked)
                .unwrap()
                .kind,
            CounterSpellConditionalKind::IfKicked
        );

        let second = lex_line(
            "Counter target spell that's the second spell cast this turn.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_counter_spell_conditional_tokens(&second)
                .unwrap()
                .kind,
            CounterSpellConditionalKind::SecondCastThisTurn
        );
    }

    #[test]
    fn parses_greatest_power_exile_shape() {
        let tokens = lex_line(
            "Exile target creature with the greatest power among creatures on the battlefield.",
            0,
        )
        .unwrap();
        let shape = parse_exile_greatest_power_creature_tokens(&tokens).unwrap();
        assert_eq!(render_token_slice(shape.target_tokens), "target creature");
    }

    #[test]
    fn parses_no_control_lose_game_shape() {
        let tokens = lex_line(
            "For each opponent who doesn't control a creature loses the game.",
            0,
        )
        .unwrap();
        let shape = parse_for_each_no_control_lose_game_tokens(&tokens).unwrap();
        assert_eq!(shape.player_kind, ForEachPlayerKind::Opponent);
        assert_eq!(render_token_slice(shape.filter_tokens), "a creature");
    }
}
