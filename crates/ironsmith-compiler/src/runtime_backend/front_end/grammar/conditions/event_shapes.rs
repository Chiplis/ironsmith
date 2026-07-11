use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::{
    CardsInHandRelationShape, LifeRelationShape, PlayerLifeChangeDirectionAst,
    PlayerWouldActionAst, SpellContextReferenceAst,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct QuantityObjectShape<'a> {
    pub(super) amount_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LifeRelationPlayerShape<'a> {
    pub(super) player_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TurnEventShape<'a> {
    pub(super) subject_tokens: &'a [OwnedLexToken],
    pub(super) amount_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SpellControllerShape<'a> {
    pub(super) controller_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SpellReferenceShape<'a> {
    pub(super) spell_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SpellCastThisTurnShape<'a> {
    pub(super) subject_tokens: &'a [OwnedLexToken],
    pub(super) object_tokens: &'a [OwnedLexToken],
    pub(super) negated: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LifeChangeThisTurnShape<'a> {
    pub(super) subject_tokens: &'a [OwnedLexToken],
    pub(super) amount_tokens: &'a [OwnedLexToken],
    pub(super) direction: PlayerLifeChangeDirectionAst,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlayerWouldShape<'a> {
    pub(super) subject_tokens: &'a [OwnedLexToken],
    pub(super) action: PlayerWouldActionAst,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SpellCastFilterPairShape<'a> {
    pub(super) left_tokens: &'a [OwnedLexToken],
    pub(super) right_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpellCastActionKind {
    DidNot,
    Didnt,
    Havent,
    Hasnt,
    Have,
    Has,
    Cast,
}

impl SpellCastActionKind {
    fn negated(self) -> bool {
        matches!(
            self,
            Self::DidNot | Self::Didnt | Self::Havent | Self::Hasnt
        )
    }
}

pub(super) fn parse_quantity_object_tail<'a>(
    tokens: &'a [OwnedLexToken],
    object_phrases: &[&[&str]],
) -> Option<QuantityObjectShape<'a>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    let amount_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(|input: &mut LexStream<'a>| expected_any_phrase(input, object_phrases)),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    expected_any_phrase(&mut input, object_phrases).ok()?;
    parse_end(&mut input).ok()?;
    Some(QuantityObjectShape { amount_tokens })
}

pub(super) fn parse_life_relation(
    tokens: &[OwnedLexToken],
) -> Option<(LifeRelationShape, Option<&[OwnedLexToken]>)> {
    let tokens = trim_clause(tokens);
    for (phrase, kind) in [
        (
            &["more", "life", "than", "you", "do"] as &[&str],
            LifeRelationShape::MoreThanYou,
        ),
        (
            &["more", "life", "than", "you"],
            LifeRelationShape::MoreThanYou,
        ),
        (
            &["more", "life", "than", "each", "other", "player"],
            LifeRelationShape::MoreThanEachOtherPlayer,
        ),
        (
            &["more", "life", "than", "each", "other", "players"],
            LifeRelationShape::MoreThanEachOtherPlayer,
        ),
        (
            &["more", "life", "than", "each", "opponent"],
            LifeRelationShape::MoreThanEachOpponent,
        ),
        (
            &["more", "life", "than", "each", "opponents"],
            LifeRelationShape::MoreThanEachOpponent,
        ),
    ] {
        if parse_complete_dynamic(tokens, phrase) {
            return Some((kind, None));
        }
    }
    let (_, player_tokens) =
        primitives::parse_prefix(tokens, primitives::phrase(&["more", "life", "than"]))?;
    (!player_tokens.is_empty()).then_some((LifeRelationShape::MoreThanYou, Some(player_tokens)))
}

pub(super) fn parse_no_opponent_more_life_than(
    tokens: &[OwnedLexToken],
) -> Option<LifeRelationPlayerShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    primitives::kw("no").parse_next(&mut input).ok()?;
    alt((primitives::kw("opponent"), primitives::kw("opponents")))
        .parse_next(&mut input)
        .ok()?;
    primitives::phrase(&["has", "more", "life", "than"])
        .parse_next(&mut input)
        .ok()?;
    let player_tokens = take_remaining(&mut input).ok()?;
    (!player_tokens.is_empty()).then_some(LifeRelationPlayerShape { player_tokens })
}

pub(super) fn parse_cards_in_hand_relation(
    tokens: &[OwnedLexToken],
) -> Option<CardsInHandRelationShape> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    parse_more_cards_in_hand_head(&mut input).ok()?;
    primitives::kw("than").parse_next(&mut input).ok()?;
    alt((
        primitives::any_phrase(&[&["you", "do"], &["you"]])
            .value(CardsInHandRelationShape::MoreThanYou),
        primitives::any_phrase(&[&["each", "other", "player"], &["each", "other", "players"]])
            .value(CardsInHandRelationShape::MoreThanEachOtherPlayer),
    ))
    .parse_next(&mut input)
    .ok()
    .filter(|_| input.is_empty())
}

pub(super) fn parse_cards_drawn_this_turn(tokens: &[OwnedLexToken]) -> Option<TurnEventShape<'_>> {
    let tokens = trim_clause(tokens);
    for action in [0u8, 1, 2] {
        let mut input = LexStream::new(tokens);
        let Ok(subject_tokens) = repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(|input: &mut LexStream<'_>| parse_draw_action(input, action)),
        )
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input) else {
            continue;
        };
        if parse_draw_action(&mut input, action).is_err() {
            continue;
        }
        let Ok(amount_tokens) =
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(parse_card_noun))
                .map(|((), ())| ())
                .take()
                .parse_next(&mut input)
        else {
            continue;
        };
        if parse_card_noun(&mut input).is_err()
            || primitives::phrase(&["this", "turn"])
                .parse_next(&mut input)
                .is_err()
        {
            continue;
        }
        if input.is_empty() {
            return Some(TurnEventShape {
                subject_tokens,
                amount_tokens,
            });
        }
    }
    None
}

pub(super) fn parse_lands_entered_this_turn(
    tokens: &[OwnedLexToken],
) -> Option<TurnEventShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    let subject_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("had").void()))
            .map(|((), _)| ())
            .take()
            .parse_next(&mut input)
            .ok()?;
    primitives::kw("had").parse_next(&mut input).ok()?;
    let amount_tokens = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(parse_land_noun))
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    parse_land_noun(&mut input).ok()?;
    alt((primitives::kw("enter"), primitives::kw("entered")))
        .parse_next(&mut input)
        .ok()?;
    opt(primitives::kw("the")).parse_next(&mut input).ok()?;
    primitives::phrase(&["battlefield", "under"])
        .parse_next(&mut input)
        .ok()?;
    alt((
        primitives::kw("your"),
        primitives::kw("their"),
        primitives::kw("that"),
        primitives::kw("its"),
    ))
    .parse_next(&mut input)
    .ok()?;
    primitives::phrase(&["control", "this", "turn"])
        .parse_next(&mut input)
        .ok()?;
    input.is_empty().then_some(TurnEventShape {
        subject_tokens,
        amount_tokens,
    })
}

pub(super) fn parse_target_spell_controller_poisoned(
    tokens: &[OwnedLexToken],
) -> Option<SpellControllerShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    let controller_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::kw("poisoned").void()),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    primitives::kw("poisoned").parse_next(&mut input).ok()?;
    input
        .is_empty()
        .then_some(SpellControllerShape { controller_tokens })
}

pub(super) fn parse_no_mana_spent_to_cast(
    tokens: &[OwnedLexToken],
) -> Option<SpellReferenceShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    primitives::phrase(&["no", "mana"])
        .parse_next(&mut input)
        .ok()?;
    alt((primitives::kw("was"), primitives::kw("were")))
        .parse_next(&mut input)
        .ok()?;
    primitives::phrase(&["spent", "to", "cast"])
        .parse_next(&mut input)
        .ok()?;
    let spell_tokens = take_remaining(&mut input).ok()?;
    (!spell_tokens.is_empty()).then_some(SpellReferenceShape { spell_tokens })
}

pub(super) fn parse_more_creatures_than_controller(
    tokens: &[OwnedLexToken],
) -> Option<SpellControllerShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    primitives::kw("more").parse_next(&mut input).ok()?;
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .parse_next(&mut input)
        .ok()?;
    primitives::kw("than").parse_next(&mut input).ok()?;
    let controller_tokens = take_remaining(&mut input).ok()?;
    (!controller_tokens.is_empty()).then_some(SpellControllerShape { controller_tokens })
}

pub(super) fn parse_spell_cast_this_turn(
    tokens: &[OwnedLexToken],
) -> Option<SpellCastThisTurnShape<'_>> {
    let tokens = trim_clause(tokens);
    for action in [
        SpellCastActionKind::DidNot,
        SpellCastActionKind::Didnt,
        SpellCastActionKind::Havent,
        SpellCastActionKind::Hasnt,
        SpellCastActionKind::Have,
        SpellCastActionKind::Has,
        SpellCastActionKind::Cast,
    ] {
        if let Some(shape) = parse_spell_cast_with_action(tokens, action) {
            return Some(shape);
        }
    }
    None
}

pub(super) fn is_another_spell(tokens: &[OwnedLexToken]) -> bool {
    parse_complete_dynamic(trim_clause(tokens), &["another", "spell"])
}

pub(super) fn parse_target_spell_controller(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextReferenceAst> {
    primitives::parse_all(
        trim_clause(tokens),
        primitives::any_phrase(&[
            &["its", "controller"],
            &["that", "spell's", "controller"],
            &["that", "spells", "controller"],
            &["that", "spell", "controller"],
        ])
        .value(SpellContextReferenceAst::TargetSpell),
        "target-spell controller reference",
    )
    .ok()
}

pub(super) fn parse_target_spell_reference(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextReferenceAst> {
    primitives::parse_all(
        trim_clause(tokens),
        primitives::any_phrase(&[&["it"], &["that", "spell"]])
            .value(SpellContextReferenceAst::TargetSpell),
        "target-spell reference",
    )
    .ok()
}

pub(super) fn parse_spell_cast_filter_pair(
    tokens: &[OwnedLexToken],
) -> Option<SpellCastFilterPairShape<'_>> {
    let tokens = trim_clause(tokens);
    parse_spell_cast_filter_pair_with_both(tokens)
        .or_else(|| parse_named_spell_cast_filter_pair(tokens))
}

pub(super) fn parse_life_change_this_turn(
    tokens: &[OwnedLexToken],
) -> Option<LifeChangeThisTurnShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    let subject_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_life_change_direction))
            .map(|((), _)| ())
            .take()
            .parse_next(&mut input)
            .ok()?;
    let direction = parse_life_change_direction(&mut input).ok()?;
    let amount_tokens =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::kw("life").void()))
            .map(|((), ())| ())
            .take()
            .parse_next(&mut input)
            .ok()?;
    primitives::phrase(&["life", "this", "turn"])
        .parse_next(&mut input)
        .ok()?;
    input.is_empty().then_some(LifeChangeThisTurnShape {
        subject_tokens,
        amount_tokens,
        direction,
    })
}

pub(super) fn parse_player_would(tokens: &[OwnedLexToken]) -> Option<PlayerWouldShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut input = LexStream::new(tokens);
    let subject_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("would").void()))
            .map(|((), ())| ())
            .take()
            .parse_next(&mut input)
            .ok()?;
    primitives::kw("would").parse_next(&mut input).ok()?;
    let action = parse_player_would_action(&mut input).ok()?;
    parse_end(&mut input).ok()?;
    Some(PlayerWouldShape {
        subject_tokens,
        action,
    })
}

fn parse_spell_cast_filter_pair_with_both(
    tokens: &[OwnedLexToken],
) -> Option<SpellCastFilterPairShape<'_>> {
    let mut input = LexStream::new(tokens);
    primitives::kw("both").parse_next(&mut input).ok()?;
    parse_spell_cast_filter_pair_tail(&mut input, false)
}

fn parse_named_spell_cast_filter_pair(
    tokens: &[OwnedLexToken],
) -> Option<SpellCastFilterPairShape<'_>> {
    let mut input = LexStream::new(tokens);
    parse_spell_cast_filter_pair_tail(&mut input, true)
}

fn parse_spell_cast_filter_pair_tail<'a>(
    input: &mut LexStream<'a>,
    require_named: bool,
) -> Option<SpellCastFilterPairShape<'a>> {
    let left_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("and").void()))
            .map(|((), ())| ())
            .take()
            .parse_next(input)
            .ok()?;
    primitives::kw("and").parse_next(input).ok()?;
    let right_tokens = take_remaining(input).ok()?;
    if right_tokens.is_empty()
        || (require_named
            && (!has_spell_named_prefix(left_tokens) || !has_spell_named_prefix(right_tokens)))
    {
        return None;
    }
    Some(SpellCastFilterPairShape {
        left_tokens,
        right_tokens,
    })
}

fn has_spell_named_prefix(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        trim_clause(tokens),
        (
            opt(primitives::kw("a")),
            primitives::phrase(&["spell", "named"]),
        )
            .void(),
    )
    .is_some()
}

fn parse_player_would_action(input: &mut LexStream<'_>) -> WResult<PlayerWouldActionAst> {
    alt((
        primitives::any_phrase(&[&["draw", "a", "card"], &["draw", "card"]])
            .value(PlayerWouldActionAst::DrawCard),
        primitives::kw("proliferate").value(PlayerWouldActionAst::Proliferate),
        primitives::any_phrase(&[
            &["begin", "extra", "turn"],
            &["begin", "an", "extra", "turn"],
        ])
        .value(PlayerWouldActionAst::BeginExtraTurn),
    ))
    .parse_next(input)
}

fn parse_spell_cast_with_action(
    tokens: &[OwnedLexToken],
    action: SpellCastActionKind,
) -> Option<SpellCastThisTurnShape<'_>> {
    let mut input = LexStream::new(tokens);
    let subject_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(|input: &mut LexStream<'_>| parse_spell_cast_action(input, action)),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    parse_spell_cast_action(&mut input, action).ok()?;
    let object_tokens = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(primitives::phrase(&["this", "turn"])),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    primitives::phrase(&["this", "turn"])
        .parse_next(&mut input)
        .ok()?;
    input.is_empty().then_some(SpellCastThisTurnShape {
        subject_tokens,
        object_tokens,
        negated: action.negated(),
    })
}

fn parse_spell_cast_action(input: &mut LexStream<'_>, action: SpellCastActionKind) -> WResult<()> {
    match action {
        SpellCastActionKind::DidNot => {
            primitives::phrase(&["did", "not", "cast"]).parse_next(input)
        }
        SpellCastActionKind::Didnt => (
            alt((primitives::kw("didn't"), primitives::kw("didnt"))),
            primitives::kw("cast"),
        )
            .void()
            .parse_next(input),
        SpellCastActionKind::Havent => (
            alt((primitives::kw("haven't"), primitives::kw("havent"))),
            primitives::kw("cast"),
        )
            .void()
            .parse_next(input),
        SpellCastActionKind::Hasnt => (
            alt((primitives::kw("hasn't"), primitives::kw("hasnt"))),
            primitives::kw("cast"),
        )
            .void()
            .parse_next(input),
        SpellCastActionKind::Have => primitives::phrase(&["have", "cast"]).parse_next(input),
        SpellCastActionKind::Has => primitives::phrase(&["has", "cast"]).parse_next(input),
        SpellCastActionKind::Cast => primitives::kw("cast").void().parse_next(input),
    }
}

fn parse_draw_action(input: &mut LexStream<'_>, kind: u8) -> WResult<()> {
    match kind {
        0 => primitives::phrase(&["has", "drawn"]).parse_next(input),
        1 => primitives::phrase(&["have", "drawn"]).parse_next(input),
        _ => primitives::kw("drew").void().parse_next(input),
    }
}

fn parse_more_cards_in_hand_head(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("more").parse_next(input)?;
    parse_card_noun(input)?;
    opt(alt((
        primitives::phrase(&["in", "hand"]),
        primitives::phrase(&["in", "your", "hand"]),
        primitives::phrase(&["in", "their", "hand"]),
    )))
    .parse_next(input)?;
    Ok(())
}

fn parse_card_noun(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)
}

fn parse_land_noun(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("land"), primitives::kw("lands")))
        .void()
        .parse_next(input)
}

fn parse_life_change_direction(input: &mut LexStream<'_>) -> WResult<PlayerLifeChangeDirectionAst> {
    alt((
        primitives::kw("gained").value(PlayerLifeChangeDirectionAst::Gained),
        primitives::kw("lost").value(PlayerLifeChangeDirectionAst::Lost),
    ))
    .parse_next(input)
}

fn expected_any_phrase(input: &mut LexStream<'_>, phrases: &[&[&str]]) -> WResult<()> {
    for phrase in phrases {
        let mut probe = input.clone();
        if expected_phrase(&mut probe, phrase).is_ok() {
            *input = probe;
            return Ok(());
        }
    }
    Err(primitives::backtrack_err(
        "condition phrase",
        "one of the expected phrases",
    ))
}

fn expected_phrase(input: &mut LexStream<'_>, phrase: &[&str]) -> WResult<()> {
    for expected in phrase {
        let _: &OwnedLexToken = any
            .verify(|token: &&OwnedLexToken| token.is_word(expected))
            .parse_next(input)?;
    }
    Ok(())
}

fn parse_complete_dynamic(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let mut input = LexStream::new(tokens);
    expected_phrase(&mut input, phrase).is_ok() && input.is_empty()
}

fn parse_end(input: &mut LexStream<'_>) -> WResult<()> {
    eof.void().parse_next(input)
}

fn take_remaining<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    rest.parse_next(input)
}

fn trim_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    super::super::super::util::trim_edge_punctuation_tokens(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_typed_turn_event_and_spell_cast_shapes() {
        let drawn = lex("You have drawn three cards this turn.");
        let shape = parse_cards_drawn_this_turn(&drawn).expect("draw event");
        assert_eq!(shape.subject_tokens.len(), 1);
        assert_eq!(shape.amount_tokens.len(), 1);

        let cast = lex("You haven't cast another spell this turn.");
        let shape = parse_spell_cast_this_turn(&cast).expect("spell cast condition");
        assert!(shape.negated);
        assert!(is_another_spell(shape.object_tokens));

        let pair = lex("Both an artifact spell and an enchantment spell");
        let pair = parse_spell_cast_filter_pair(&pair).expect("spell-filter pair");
        assert!(!pair.left_tokens.is_empty());
        assert!(!pair.right_tokens.is_empty());

        let action = lex("You would draw a card");
        let action = parse_player_would(&action).expect("would-action shape");
        assert_eq!(action.action, PlayerWouldActionAst::DrawCard);
    }
}
