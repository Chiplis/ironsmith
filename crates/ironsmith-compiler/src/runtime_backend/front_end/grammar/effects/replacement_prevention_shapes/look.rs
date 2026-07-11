use super::super::*;

use crate::runtime_backend::front_end::grammar::leaf;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LookHandPlayerShape {
    TargetPlayer,
    TargetOpponent,
    Opponent,
    IteratedPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LookHandShape {
    pub(crate) player: LookHandPlayerShape,
    pub(crate) choose_card_name: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LookTopExileOneShape {
    pub(crate) count: u32,
    pub(crate) player: PlayerAst,
}

fn look_hand_player<'a>(input: &mut LexStream<'a>) -> WResult<LookHandPlayerShape> {
    alt((
        primitives::any_phrase(&[
            &["target", "player's"],
            &["target", "players'"],
            &["target", "players"],
            &["target", "player"],
        ])
        .value(LookHandPlayerShape::TargetPlayer),
        primitives::any_phrase(&[
            &["target", "opponent's"],
            &["target", "opponents'"],
            &["target", "opponent"],
            &["target", "opponents"],
        ])
        .value(LookHandPlayerShape::TargetOpponent),
        primitives::any_phrase(&[
            &["an", "opponent's"],
            &["an", "opponents'"],
            &["an", "opponents"],
            &["opponent's"],
            &["opponents'"],
            &["opponents"],
        ])
        .value(LookHandPlayerShape::Opponent),
        primitives::any_phrase(&[
            &["that", "player's"],
            &["that", "players'"],
            &["that", "players"],
        ])
        .value(LookHandPlayerShape::IteratedPlayer),
    ))
    .parse_next(input)
}

fn look_hand<'a>(input: &mut LexStream<'a>) -> WResult<LookHandShape> {
    primitives::phrase(&["look", "at"]).parse_next(input)?;
    let player = look_hand_player.parse_next(input)?;
    primitives::kw("hand").parse_next(input)?;
    let choose_card_name = opt((
        opt(primitives::comma()),
        primitives::kw("then"),
        primitives::phrase(&["choose", "any", "card", "name"]),
    ))
    .parse_next(input)?
    .is_some();
    primitives::sentence_end().parse_next(input)?;
    Ok(LookHandShape {
        player,
        choose_card_name,
    })
}

pub(crate) fn parse_look_hand_shape(tokens: &[OwnedLexToken]) -> Option<LookHandShape> {
    primitives::parse_all(tokens, look_hand, "look at hand shape").ok()
}

fn exile_one_followup(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_lexed_commas(tokens);
    let tokens = primitives::parse_prefix(
        tokens,
        opt(alt((primitives::kw("then"), primitives::kw("and")))),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens);
    primitives::parse_prefix(
        tokens,
        primitives::any_phrase(&[
            &["exile", "one", "of", "them"],
            &["exile", "one", "of", "those"],
            &["exile", "one", "of", "those", "cards"],
        ]),
    )
    .is_some()
}

pub(crate) fn parse_look_top_exile_one_shape(
    tokens: &[OwnedLexToken],
) -> Option<LookTopExileOneShape> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::phrase(&["look", "at"]))?;
    let (_, body) = primitives::parse_prefix(body, opt(primitives::kw("the")))?;
    let (_, body) = primitives::parse_prefix(body, primitives::kw("top"))?;
    let (count, body) = primitives::parse_prefix(body, leaf::parse_leaf_number_prefix_lexed)?;
    let (_, body) = primitives::parse_prefix(
        body,
        alt((
            primitives::phrase(&["cards", "of"]),
            primitives::phrase(&["card", "of"]),
            primitives::kw("of").void(),
        )),
    )?;
    let (owner_tokens, followup_tokens) =
        primitives::split_lexed_once_on_separator(body, || primitives::kw("library").void())?;
    let player = match parse_subject(trim_lexed_commas(owner_tokens)) {
        SubjectAst::Player(player) => player,
        _ => return None,
    };
    if !exile_one_followup(followup_tokens) {
        return None;
    }
    Some(LookTopExileOneShape { count, player })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_hand_targets_and_name_followup() {
        let tokens = lex_line("Look at an opponent's hand, then choose any card name.", 0).unwrap();
        assert_eq!(
            parse_look_hand_shape(&tokens),
            Some(LookHandShape {
                player: LookHandPlayerShape::Opponent,
                choose_card_name: true,
            })
        );
    }

    #[test]
    fn parses_look_top_exile_one_shape() {
        let tokens = lex_line(
            "Look at the top three cards of your library, then exile one of those cards.",
            0,
        )
        .unwrap();
        let shape = parse_look_top_exile_one_shape(&tokens).unwrap();
        assert_eq!(shape.count, 3);
        assert_eq!(shape.player, PlayerAst::You);
    }
}
