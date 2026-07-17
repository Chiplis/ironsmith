use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::effect::{EventValueSpec, Value};
use crate::runtime_backend::front_end::grammar::{primitives, values};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView,
};
use ironsmith_core::ValueSurfaceHint;

use super::{is_each_opponent_library_shape, parse_exile_library_owner_shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExileLibraryPlayerShape {
    Player(PlayerAst),
    EachOpponent,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExileLibraryCardsShape {
    pub(crate) player: ExileLibraryPlayerShape,
    pub(crate) count: Value,
}

fn trim_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0;
    let mut end = tokens.len();
    while start < end && tokens[start].kind == TokenKind::Comma {
        start += 1;
    }
    while end > start && tokens[end - 1].kind == TokenKind::Comma {
        end -= 1;
    }
    &tokens[start..end]
}

fn card_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)
}

fn library_player(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
    allow_each_opponent: bool,
) -> Option<ExileLibraryPlayerShape> {
    if allow_each_opponent && is_each_opponent_library_shape(tokens) {
        return Some(ExileLibraryPlayerShape::EachOpponent);
    }
    let owner = parse_exile_library_owner_shape(tokens, default_player)?;
    (owner.consumed_words == TokenWordView::new(tokens).len())
        .then_some(ExileLibraryPlayerShape::Player(owner.player))
}

fn strip_position_and_of<'a>(
    tokens: &'a [OwnedLexToken],
    position: &'static str,
) -> Option<&'a [OwnedLexToken]> {
    primitives::parse_prefix(
        tokens,
        (
            opt(primitives::kw("the")),
            primitives::kw(position),
            primitives::kw("of"),
        ),
    )
    .map(|(_, rest)| rest)
}

fn parse_position_count_and_owner<'a>(
    tokens: &'a [OwnedLexToken],
    position: &'static str,
) -> Option<(Value, bool, &'a [OwnedLexToken])> {
    let (_, after_position) = primitives::parse_prefix(
        tokens,
        (opt(primitives::kw("the")), primitives::kw(position)),
    )?;
    if let Some(((), after_cards)) = primitives::parse_prefix(after_position, card_word) {
        let (_, owner) = primitives::parse_prefix(after_cards, primitives::kw("of"))?;
        return Some((Value::Fixed(1), true, trim_commas(owner)));
    }
    let (count, used) = values::parse_value_prefix_lexed(after_position)?;
    let (_, after_cards) = primitives::parse_prefix(&after_position[used..], card_word)?;
    let (_, owner) = primitives::parse_prefix(after_cards, primitives::kw("of"))?;
    Some((count, false, trim_commas(owner)))
}

fn parse_position_count_without_owner(
    tokens: &[OwnedLexToken],
    position: &'static str,
) -> Option<(Value, bool)> {
    let (_, after_position) = primitives::parse_prefix(
        tokens,
        (opt(primitives::kw("the")), primitives::kw(position)),
    )?;
    if let Some(((), rest)) = primitives::parse_prefix(after_position, card_word)
        && trim_commas(rest).is_empty()
    {
        return Some((Value::Fixed(1), true));
    }
    let (count, used) = values::parse_value_prefix_lexed(after_position)?;
    let (_, rest) = primitives::parse_prefix(&after_position[used..], card_word)?;
    trim_commas(rest).is_empty().then_some((count, false))
}

pub(crate) fn parse_exile_dynamic_top_library_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    let tokens = trim_commas(tokens);
    if let Some((_, after_cards)) = primitives::parse_prefix(tokens, card_word)
        && let Some((before_equal, after_equal)) =
            primitives::split_lexed_once_on_separator(after_cards, || {
                primitives::phrase(&["equal", "to"]).void()
            })
        && let Some((_, position_tokens)) =
            primitives::parse_prefix(trim_commas(before_equal), primitives::kw("from"))
        && let Some(owner_tokens) = strip_position_and_of(trim_commas(position_tokens), "top")
    {
        let player = library_player(trim_commas(owner_tokens), default_player, false)?;
        let (count, used) = values::parse_value_prefix_lexed(trim_commas(after_equal))?;
        if used == trim_commas(after_equal).len() {
            return Some(ExileLibraryCardsShape {
                player,
                count: count.with_surface_hint(ValueSurfaceHint::EqualTo),
            });
        }
    }
    let (count, after_from) = if let Some((_, after_from)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["that", "many"]),
            card_word,
            primitives::kw("from"),
        ),
    ) {
        (Value::EventValue(EventValueSpec::Amount), after_from)
    } else {
        let (_, after_cards) = primitives::parse_prefix(tokens, card_word)?;
        let (count_tokens, after_from) =
            primitives::split_lexed_once_on_separator(after_cards, || {
                primitives::kw("from").void()
            })?;
        if TokenWordView::new(count_tokens).is_empty() {
            return None;
        }
        let count = values::parse_add_mana_equal_amount_value_lexed(trim_commas(count_tokens))?;
        (count, after_from)
    };
    let owner_tokens = strip_position_and_of(trim_commas(after_from), "top")?;
    let player = library_player(trim_commas(owner_tokens), default_player, false)?;
    Some(ExileLibraryCardsShape { player, count })
}

pub(crate) fn parse_exile_top_library_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    let tokens = trim_commas(tokens);
    let (count, player) = if let Some((count, _implicit, owner_tokens)) =
        parse_position_count_and_owner(tokens, "top")
    {
        (count, library_player(owner_tokens, default_player, true)?)
    } else {
        let (count, _implicit) = parse_position_count_without_owner(tokens, "top")?;
        (count, ExileLibraryPlayerShape::Player(default_player))
    };
    Some(ExileLibraryCardsShape { player, count })
}

pub(crate) fn parse_exile_bottom_library_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ExileLibraryCardsShape> {
    let tokens = trim_commas(tokens);
    let (count, _, owner_tokens) = parse_position_count_and_owner(tokens, "bottom")?;
    if count != Value::Fixed(1) {
        return None;
    }
    let player = library_player(owner_tokens, default_player, true)?;
    Some(ExileLibraryCardsShape { player, count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_dynamic_top_and_single_bottom_library_shapes() {
        let dynamic = parse_exile_dynamic_top_library_shape(
            &lex("that many cards from the top of your library"),
            PlayerAst::Implicit,
        )
        .unwrap();
        assert_eq!(dynamic.count, Value::EventValue(EventValueSpec::Amount));
        assert_eq!(
            dynamic.player,
            ExileLibraryPlayerShape::Player(PlayerAst::You)
        );

        let top = parse_exile_top_library_shape(
            &lex("the top two cards of each opponent's library"),
            PlayerAst::Implicit,
        )
        .unwrap();
        assert_eq!(top.count, Value::Fixed(2));
        assert_eq!(top.player, ExileLibraryPlayerShape::EachOpponent);

        let damaged_player_top = parse_exile_top_library_shape(
            &lex("the top card of that player's library"),
            PlayerAst::Implicit,
        )
        .unwrap();
        assert_eq!(damaged_player_top.count, Value::Fixed(1));
        assert_eq!(
            damaged_player_top.player,
            ExileLibraryPlayerShape::Player(PlayerAst::That)
        );

        let implicit_library_top =
            parse_exile_top_library_shape(&lex("the top four cards"), PlayerAst::You).unwrap();
        assert_eq!(implicit_library_top.count, Value::Fixed(4));
        assert_eq!(
            implicit_library_top.player,
            ExileLibraryPlayerShape::Player(PlayerAst::You)
        );

        assert!(
            parse_exile_top_library_shape(
                &lex("the top four cards from a graveyard"),
                PlayerAst::You,
            )
            .is_none(),
            "the implicit-library route must consume the complete top-card phrase"
        );

        let bottom = parse_exile_bottom_library_shape(
            &lex("the bottom card of their library"),
            PlayerAst::Opponent,
        )
        .unwrap();
        assert_eq!(
            bottom.player,
            ExileLibraryPlayerShape::Player(PlayerAst::Opponent)
        );

        let excess = parse_exile_dynamic_top_library_shape(
            &lex(
                "cards from the top of your library equal to the excess damage dealt to that creature this way",
            ),
            PlayerAst::Implicit,
        )
        .unwrap();
        assert_eq!(
            excess.count.unhinted(),
            &Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::ExcessDamage,
            }
        );
        assert!(excess.count.has_surface_hint(ValueSurfaceHint::EqualTo));
    }
}
