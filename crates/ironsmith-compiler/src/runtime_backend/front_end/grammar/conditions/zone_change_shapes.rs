use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BattlefieldChangeShape {
    NoPermanentLeft,
    PermanentLeft,
    PermanentLeftUnderYourControl,
    LandPutIntoGraveyardFromBattlefield,
    NonlandPermanentLeftOrSpellWarped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DamagerShape {
    ThisCreature,
    EquippedCreature,
    EnchantedCreature,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DiedShape<'a> {
    pub(super) amount_tokens: &'a [OwnedLexToken],
    pub(super) under_your_control: bool,
    pub(super) damaged_by: Option<DamagerShape>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DeathShape<'a> {
    Died(DiedShape<'a>),
    CreatureCardPutIntoYourGraveyard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryWindowShape {
    ThisTurn,
    LastTurn,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum EntryShape<'a> {
    LandThisTurn,
    Object {
        object_tokens: &'a [OwnedLexToken],
        window: EntryWindowShape,
        other: bool,
    },
}

pub(super) fn parse_battlefield_change(tokens: &[OwnedLexToken]) -> Option<BattlefieldChangeShape> {
    let tokens = trim_clause(tokens);
    primitives::parse_all(
        tokens,
        alt((
            parse_no_permanent_left,
            parse_permanent_left_under_control,
            parse_land_put_into_graveyard,
            parse_nonland_left_or_warped,
            parse_permanent_left,
        )),
        "battlefield-change condition",
    )
    .ok()
}

pub(super) fn parse_death(tokens: &[OwnedLexToken]) -> Option<DeathShape<'_>> {
    let tokens = trim_clause(tokens);
    if parse_complete(tokens, parse_creature_card_put_into_graveyard) {
        return Some(DeathShape::CreatureCardPutIntoYourGraveyard);
    }
    parse_damaged_death(tokens)
        .or_else(|| parse_controlled_death(tokens))
        .or_else(|| parse_plain_death(tokens))
        .map(DeathShape::Died)
}

pub(super) fn parse_entry(tokens: &[OwnedLexToken]) -> Option<EntryShape<'_>> {
    let tokens = trim_clause(tokens);
    if parse_complete(tokens, parse_land_entry) {
        return Some(EntryShape::LandThisTurn);
    }
    parse_object_entry_last_turn(tokens).or_else(|| parse_object_entry_this_turn(tokens))
}

fn parse_no_permanent_left(input: &mut LexStream<'_>) -> WResult<BattlefieldChangeShape> {
    primitives::kw("no").parse_next(input)?;
    parse_permanent_noun(input)?;
    primitives::kw("left").parse_next(input)?;
    parse_battlefield_this_turn(input)?;
    Ok(BattlefieldChangeShape::NoPermanentLeft)
}

fn parse_permanent_left(input: &mut LexStream<'_>) -> WResult<BattlefieldChangeShape> {
    opt(parse_article).parse_next(input)?;
    parse_permanent_noun(input)?;
    primitives::kw("left").parse_next(input)?;
    parse_battlefield_this_turn(input)?;
    Ok(BattlefieldChangeShape::PermanentLeft)
}

fn parse_permanent_left_under_control(
    input: &mut LexStream<'_>,
) -> WResult<BattlefieldChangeShape> {
    opt(parse_article).parse_next(input)?;
    alt((
        (
            parse_permanent_or_creature_noun,
            primitives::kw("left"),
            parse_battlefield_under_your_control_this_turn,
        )
            .void(),
        (
            parse_permanent_noun,
            primitives::phrase(&["you", "controlled"]),
            primitives::kw("left"),
            parse_battlefield_this_turn,
        )
            .void(),
    ))
    .parse_next(input)?;
    Ok(BattlefieldChangeShape::PermanentLeftUnderYourControl)
}

fn parse_land_put_into_graveyard(input: &mut LexStream<'_>) -> WResult<BattlefieldChangeShape> {
    opt(parse_article).parse_next(input)?;
    alt((primitives::kw("land"), primitives::kw("lands"))).parse_next(input)?;
    primitives::phrase(&["you", "controlled"]).parse_next(input)?;
    alt((primitives::kw("was"), primitives::kw("were"))).parse_next(input)?;
    primitives::phrase(&["put", "into"]).parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::kw("graveyard").parse_next(input)?;
    primitives::kw("from").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["battlefield", "this", "turn"]).parse_next(input)?;
    Ok(BattlefieldChangeShape::LandPutIntoGraveyardFromBattlefield)
}

fn parse_nonland_left_or_warped(input: &mut LexStream<'_>) -> WResult<BattlefieldChangeShape> {
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::phrase(&["nonland", "permanent", "left"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["battlefield", "this", "turn", "or"]).parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::phrase(&["spell", "was", "warped", "this", "turn"]).parse_next(input)?;
    Ok(BattlefieldChangeShape::NonlandPermanentLeftOrSpellWarped)
}

fn parse_damaged_death(tokens: &[OwnedLexToken]) -> Option<DiedShape<'_>> {
    let mut input = LexStream::new(tokens);
    let amount_tokens = take_until_creature_noun(&mut input).ok()?;
    parse_creature_noun(&mut input).ok()?;
    primitives::phrase(&["dealt", "damage", "by"])
        .parse_next(&mut input)
        .ok()?;
    let damaged_by = parse_damager(&mut input).ok()?;
    primitives::phrase(&["this", "turn", "died"])
        .parse_next(&mut input)
        .ok()?;
    parse_end(&mut input).ok()?;
    Some(DiedShape {
        amount_tokens,
        under_your_control: false,
        damaged_by: Some(damaged_by),
    })
}

fn parse_controlled_death(tokens: &[OwnedLexToken]) -> Option<DiedShape<'_>> {
    let mut input = LexStream::new(tokens);
    let amount_tokens = take_until_creature_noun(&mut input).ok()?;
    parse_creature_noun(&mut input).ok()?;
    primitives::phrase(&["died", "under", "your", "control", "this", "turn"])
        .parse_next(&mut input)
        .ok()?;
    parse_end(&mut input).ok()?;
    Some(DiedShape {
        amount_tokens,
        under_your_control: true,
        damaged_by: None,
    })
}

fn parse_plain_death(tokens: &[OwnedLexToken]) -> Option<DiedShape<'_>> {
    let mut input = LexStream::new(tokens);
    let amount_tokens = take_until_creature_noun(&mut input).ok()?;
    parse_creature_noun(&mut input).ok()?;
    primitives::phrase(&["died", "this", "turn"])
        .parse_next(&mut input)
        .ok()?;
    parse_end(&mut input).ok()?;
    Some(DiedShape {
        amount_tokens,
        under_your_control: false,
        damaged_by: None,
    })
}

fn parse_creature_card_put_into_graveyard(input: &mut LexStream<'_>) -> WResult<()> {
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::phrase(&[
        "creature",
        "card",
        "was",
        "put",
        "into",
        "your",
        "graveyard",
        "from",
        "anywhere",
        "this",
        "turn",
    ])
    .parse_next(input)
}

fn parse_land_entry(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["you", "had"]).parse_next(input)?;
    alt((primitives::kw("land"), primitives::kw("lands"))).parse_next(input)?;
    alt((primitives::kw("enter"), primitives::kw("entered"))).parse_next(input)?;
    primitives::phrase(&["battlefield", "under", "your", "control", "this", "turn"])
        .parse_next(input)
}

fn parse_object_entry_last_turn(tokens: &[OwnedLexToken]) -> Option<EntryShape<'_>> {
    let mut input = LexStream::new(tokens);
    primitives::phrase(&["you", "had"])
        .parse_next(&mut input)
        .ok()?;
    let object_tokens = take_until_entry_verb(&mut input).ok()?;
    parse_entry_verb(&mut input).ok()?;
    opt(primitives::kw("the")).parse_next(&mut input).ok()?;
    primitives::phrase(&["battlefield", "under", "your", "control", "last", "turn"])
        .parse_next(&mut input)
        .ok()?;
    parse_end(&mut input).ok()?;
    Some(EntryShape::Object {
        object_tokens,
        window: EntryWindowShape::LastTurn,
        other: has_other_prefix(object_tokens),
    })
}

fn parse_object_entry_this_turn(tokens: &[OwnedLexToken]) -> Option<EntryShape<'_>> {
    let mut input = LexStream::new(tokens);
    let object_tokens = take_until_entry_verb(&mut input).ok()?;
    parse_entry_verb(&mut input).ok()?;
    opt(primitives::kw("the")).parse_next(&mut input).ok()?;
    primitives::phrase(&["battlefield", "under", "your", "control", "this", "turn"])
        .parse_next(&mut input)
        .ok()?;
    parse_end(&mut input).ok()?;
    Some(EntryShape::Object {
        object_tokens,
        window: EntryWindowShape::ThisTurn,
        other: has_other_prefix(object_tokens),
    })
}

fn take_until_creature_noun<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(parse_creature_noun))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn take_until_entry_verb<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_entry_verb))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn parse_damager(input: &mut LexStream<'_>) -> WResult<DamagerShape> {
    alt((
        primitives::phrase(&["this", "creature"]).value(DamagerShape::ThisCreature),
        primitives::phrase(&["equipped", "creature"]).value(DamagerShape::EquippedCreature),
        primitives::phrase(&["enchanted", "creature"]).value(DamagerShape::EnchantedCreature),
    ))
    .parse_next(input)
}

fn parse_entry_verb(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("enter"), primitives::kw("entered")))
        .void()
        .parse_next(input)
}

fn parse_permanent_noun(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("permanent"), primitives::kw("permanents")))
        .void()
        .parse_next(input)
}

fn parse_permanent_or_creature_noun(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("permanent"),
        primitives::kw("permanents"),
        primitives::kw("creature"),
        primitives::kw("creatures"),
    ))
    .void()
    .parse_next(input)
}

fn parse_creature_noun(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .void()
        .parse_next(input)
}

fn parse_article(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    ))
    .void()
    .parse_next(input)
}

fn parse_battlefield_this_turn(input: &mut LexStream<'_>) -> WResult<()> {
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["battlefield", "this", "turn"]).parse_next(input)
}

fn parse_battlefield_under_your_control_this_turn(input: &mut LexStream<'_>) -> WResult<()> {
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["battlefield", "under", "your", "control", "this", "turn"])
        .parse_next(input)
}

fn has_other_prefix(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        alt((primitives::kw("another"), primitives::kw("other"))).void(),
    )
    .is_some()
}

fn parse_complete<'a, O>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexStream<'a>, O, winnow::error::ErrMode<winnow::error::ContextError>>,
) -> bool {
    primitives::parse_all(tokens, parser, "condition zone-change shape").is_ok()
}

fn parse_end(input: &mut LexStream<'_>) -> WResult<()> {
    eof.void().parse_next(input)
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
    fn parses_typed_zone_change_and_entry_shapes() {
        let death = lex("Two creatures died under your control this turn.");
        let DeathShape::Died(shape) = parse_death(&death).expect("death shape") else {
            panic!("expected died shape");
        };
        assert!(shape.under_your_control);
        assert_eq!(shape.amount_tokens.len(), 1);

        let entry = lex("Another creature entered the battlefield under your control this turn.");
        let EntryShape::Object { other, window, .. } = parse_entry(&entry).expect("entry shape")
        else {
            panic!("expected object entry");
        };
        assert!(other);
        assert_eq!(window, EntryWindowShape::ThisTurn);
    }
}
