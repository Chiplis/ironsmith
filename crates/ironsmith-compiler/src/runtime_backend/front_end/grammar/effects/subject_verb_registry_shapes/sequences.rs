use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};
use crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChoosePlayerToEffectShape<'a> {
    pub(crate) choose_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReturnHalfControlledShape<'a> {
    pub(crate) filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HistoricalHalfDamageShape<'a> {
    pub(crate) card_type_word: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExiledHandOwner {
    Your,
    Their,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DrawForExiledHandShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) hand_owner: ExiledHandOwner,
    pub(crate) shuffles_first: bool,
    pub(crate) starts_with_draws: bool,
}

fn connector<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("and"), primitives::kw("then")))
        .void()
        .parse_next(input)
}

fn optional_connector<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(connector).void().parse_next(input)
}

fn return_half<'a>(input: &mut LexStream<'a>) -> WResult<ReturnHalfControlledShape<'a>> {
    optional_connector.parse_next(input)?;
    primitives::phrase(&["return", "half", "the"]).parse_next(input)?;
    let filter_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["they", "control"])),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["they", "control", "to", "their"]).parse_next(input)?;
    alt((
        primitives::kw("owner's"),
        primitives::kw("owners'"),
        primitives::kw("owners"),
        primitives::kw("owner"),
    ))
    .parse_next(input)?;
    alt((primitives::kw("hand"), primitives::kw("hands"))).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["rounded", "up"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ReturnHalfControlledShape { filter_tokens })
}

fn deal_action<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("deal"), primitives::kw("deals")))
        .void()
        .parse_next(input)
}

fn historical_half_damage<'a>(input: &mut LexStream<'a>) -> WResult<HistoricalHalfDamageShape<'a>> {
    optional_connector.parse_next(input)?;
    repeat_till(0.., any.void(), peek(deal_action))
        .map(|((), ())| ())
        .parse_next(input)?;
    deal_action.parse_next(input)?;
    primitives::phrase(&[
        "damage", "to", "that", "player", "equal", "to", "half", "the", "damage", "dealt", "by",
        "one", "of", "those",
    ])
    .parse_next(input)?;
    let card_type_word = primitives::word_parser_text.parse_next(input)?;
    primitives::phrase(&["spells", "this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["rounded", "down"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(HistoricalHalfDamageShape { card_type_word })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawAction {
    ShuffleThenDraw,
    Draw,
    Draws,
}

fn draw_action<'a>(input: &mut LexStream<'a>) -> WResult<DrawAction> {
    alt((
        primitives::phrase(&["shuffles", "then", "draws"]).value(DrawAction::ShuffleThenDraw),
        primitives::kw("draw").value(DrawAction::Draw),
        primitives::kw("draws").value(DrawAction::Draws),
    ))
    .parse_next(input)
}

fn draw_for_exiled_hand<'a>(input: &mut LexStream<'a>) -> WResult<DrawForExiledHandShape<'a>> {
    optional_connector.parse_next(input)?;
    let subject_tokens = repeat_till(0.., any.void(), peek(draw_action))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let action = draw_action.parse_next(input)?;
    primitives::phrase(&["a", "card", "for", "each", "card", "exiled", "from"])
        .parse_next(input)?;
    let hand_owner = alt((
        primitives::kw("your").value(ExiledHandOwner::Your),
        primitives::kw("their").value(ExiledHandOwner::Their),
    ))
    .parse_next(input)?;
    primitives::phrase(&["hand", "this", "way"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(DrawForExiledHandShape {
        subject_tokens: trim_edge_punctuation_tokens(subject_tokens),
        hand_owner,
        shuffles_first: action == DrawAction::ShuffleThenDraw,
        starts_with_draws: subject_tokens.is_empty() && action == DrawAction::Draws,
    })
}

fn strip_leading_connectors(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    tokens = trim_edge_punctuation_tokens(tokens);
    loop {
        let Some(((), rest)) = primitives::parse_prefix(tokens, connector) else {
            return tokens;
        };
        tokens = trim_edge_punctuation_tokens(rest);
    }
}

pub(crate) fn parse_choose_player_to_effect_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChoosePlayerToEffectShape<'_>> {
    let tokens = strip_leading_connectors(tokens);
    let (marker, _, effect_tokens) =
        primitives::find_prefix(tokens, || primitives::kw("to").void())?;
    let choose_tokens = trim_edge_punctuation_tokens(tokens.get(..marker)?);
    let effect_tokens = trim_edge_punctuation_tokens(effect_tokens);
    (!choose_tokens.is_empty() && !effect_tokens.is_empty()).then_some(ChoosePlayerToEffectShape {
        choose_tokens,
        effect_tokens,
    })
}

pub(crate) fn parse_return_half_controlled_shape(
    tokens: &[OwnedLexToken],
) -> Option<ReturnHalfControlledShape<'_>> {
    primitives::parse_all(tokens, return_half, "registry-return-half").ok()
}

pub(crate) fn parse_historical_half_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<HistoricalHalfDamageShape<'_>> {
    primitives::parse_all(tokens, historical_half_damage, "registry-half-damage").ok()
}

pub(crate) fn parse_draw_for_exiled_hand_shape(
    tokens: &[OwnedLexToken],
) -> Option<DrawForExiledHandShape<'_>> {
    primitives::parse_all(tokens, draw_for_exiled_hand, "registry-draw-exiled-hand").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_registry_sequence_shapes() {
        let choose = lex_line("Then choose target player to draw a card", 0).unwrap();
        let shape = parse_choose_player_to_effect_shape(&choose).unwrap();
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            vec!["draw", "a", "card"]
        );

        let returned = lex_line(
            "Return half the creatures they control to their owners' hands, rounded up.",
            0,
        )
        .unwrap();
        assert!(parse_return_half_controlled_shape(&returned).is_some());

        let draw = lex_line(
            "That player shuffles then draws a card for each card exiled from their hand this way.",
            0,
        )
        .unwrap();
        assert!(
            parse_draw_for_exiled_hand_shape(&draw)
                .unwrap()
                .shuffles_first
        );

        let historical = lex_line(
            "Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_historical_half_damage_shape(&historical)
                .unwrap()
                .card_type_word,
            "sorcery"
        );
    }
}
