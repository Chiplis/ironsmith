use super::super::*;

use crate::grammar::leaf;
use crate::util::parse_zone_word;
use winnow::combinator::{alt, opt, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeSharedTypeShape {
    PermanentType,
    CardType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeValueKindShape {
    Power,
    Toughness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeValueOperandShape<'a> {
    LifeTotal(PlayerAst),
    SourceStat {
        source_tokens: &'a [OwnedLexToken],
        kind: ExchangeValueKindShape,
    },
    TargetStat {
        target_tokens: &'a [OwnedLexToken],
        kind: ExchangeValueKindShape,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangeControlShape<'a> {
    pub heterogeneous: Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])>,
    pub filter_tokens: &'a [OwnedLexToken],
    pub count: u32,
    pub shared_type: Option<ExchangeSharedTypeShape>,
    pub invalid_shared_type: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeClauseShape<'a> {
    LifeTotalsOnly,
    LifeTotalsWith(PlayerAst),
    TextBoxes {
        target_tokens: &'a [OwnedLexToken],
    },
    Zones {
        player: PlayerAst,
        zone1: Zone,
        zone2: Zone,
    },
    Values {
        tokens: &'a [OwnedLexToken],
    },
    Control(ExchangeControlShape<'a>),
}

fn marker_anywhere<'a, O, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, O, ErrMode<ContextError>>,
{
    let mut input = LexStream::new(tokens);
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), parser)
        .parse_next(&mut input)
        .is_ok()
}

fn exchange_verb<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("exchange"), primitives::kw("exchanges")))
        .void()
        .parse_next(input)
}

fn dynamic_phrase<'a, 'p>(
    words: &'p [&'p str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut LexStream<'a>| {
        for word in words {
            let expected = *word;
            any.verify(move |token: &&OwnedLexToken| token.is_word(expected))
                .void()
                .parse_next(input)?;
        }
        Ok(())
    }
}

fn exact_shape(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    primitives::parse_all(
        tokens,
        (dynamic_phrase(phrase), primitives::sentence_end()).void(),
        "exchange exact surface",
    )
    .is_ok()
}

fn partner_shape(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let candidates = [
        (&["you"][..], PlayerAst::You),
        (&["target", "player"][..], PlayerAst::Target),
        (&["target", "players"][..], PlayerAst::Target),
        (&["target", "opponent"][..], PlayerAst::TargetOpponent),
        (&["target", "opponents"][..], PlayerAst::TargetOpponent),
        (&["that", "player"][..], PlayerAst::That),
        (&["that", "players"][..], PlayerAst::That),
        (&["opponent"][..], PlayerAst::Opponent),
        (&["opponents"][..], PlayerAst::Opponent),
        (&["an", "opponent"][..], PlayerAst::Opponent),
    ];
    candidates
        .iter()
        .find_map(|(words, player)| exact_shape(tokens, words).then_some(*player))
}

fn zone_lexed<'a>(input: &mut LexStream<'a>) -> WResult<Zone> {
    primitives::word_parser_text
        .verify_map(parse_zone_word)
        .parse_next(input)
}

fn player_zone_prefix<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    alt((
        primitives::kw("your").value(PlayerAst::You),
        primitives::phrase(&["target", "player"]).value(PlayerAst::Target),
        primitives::phrase(&["target", "players"]).value(PlayerAst::Target),
        primitives::phrase(&["target", "opponent"]).value(PlayerAst::TargetOpponent),
        primitives::phrase(&["target", "opponents"]).value(PlayerAst::TargetOpponent),
        primitives::phrase(&["an", "opponent"]).value(PlayerAst::Opponent),
        primitives::kw("opponent").value(PlayerAst::Opponent),
        primitives::kw("opponents").value(PlayerAst::Opponent),
    ))
    .parse_next(input)
}

fn parse_zone_exchange_lexed<'a>(input: &mut LexStream<'a>) -> WResult<(PlayerAst, Zone, Zone)> {
    let player = player_zone_prefix.parse_next(input)?;
    let zone1 = zone_lexed.parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let zone2 = zone_lexed.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok((player, zone1, zone2))
}

fn split_on<'a>(
    tokens: &'a [OwnedLexToken],
    words: &'static [&'static str],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let mut input = LexStream::new(tokens);
    let (_, taken) = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), dynamic_phrase(words))
        .with_taken()
        .parse_next(&mut input)
        .ok()?;
    let marker_start = taken.len().checked_sub(words.len())?;
    Some((
        trim_lexed_commas(&tokens[..marker_start]),
        trim_lexed_commas(&tokens[taken.len()..]),
    ))
}

fn split_shared_type(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<ExchangeSharedTypeShape>) {
    let Some((head, tail)) =
        split_on(tokens, &["that", "share"]).or_else(|| split_on(tokens, &["that", "shares"]))
    else {
        return (tokens, None);
    };
    let shared = primitives::parse_all(
        tail,
        parse_shared_type_tail_lexed,
        "exchange shared-type relation",
    )
    .ok();
    (head, shared)
}

fn parse_shared_type_tail_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ExchangeSharedTypeShape> {
    opt(primitives::kw("a")).parse_next(input)?;
    let shared_type = alt((
        alt((
            primitives::phrase(&["permanent", "type"]),
            primitives::phrase(&["one", "of", "those", "permanent", "types"]),
        ))
        .value(ExchangeSharedTypeShape::PermanentType),
        alt((
            primitives::phrase(&["card", "type"]),
            primitives::phrase(&["one", "of", "those", "types"]),
        ))
        .value(ExchangeSharedTypeShape::CardType),
    ))
    .parse_next(input)?;
    opt(primitives::phrase(&["with", "it"])).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(shared_type)
}

fn has_shared_type_relation(tokens: &[OwnedLexToken]) -> bool {
    split_on(tokens, &["that", "share"]).is_some()
        || split_on(tokens, &["that", "shares"]).is_some()
}

fn parse_control_shape(tokens: &[OwnedLexToken]) -> Option<ExchangeControlShape<'_>> {
    let (_, body) = primitives::parse_prefix(tokens, primitives::phrase(&["control", "of"]))?;
    let heterogeneous = split_on(body, &["and"]).map(|(left, right)| {
        let (right, _) = split_shared_type(right);
        (left, right)
    });
    let mut input = LexStream::new(body);
    let count = leaf::parse_leaf_number_prefix_lexed
        .parse_next(&mut input)
        .unwrap_or(2);
    opt(primitives::kw("target")).parse_next(&mut input).ok()?;
    let consumed = body.len().checked_sub(input.len())?;
    let (filter_tokens, shared_type) = split_shared_type(&body[consumed..]);
    Some(ExchangeControlShape {
        heterogeneous,
        filter_tokens,
        count,
        shared_type,
        invalid_shared_type: has_shared_type_relation(body) && shared_type.is_none(),
    })
}

fn life_total_operand(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let candidates = [
        (&["your", "life", "total"][..], PlayerAst::You),
        (
            &["target", "player", "life", "total"][..],
            PlayerAst::Target,
        ),
        (
            &["target", "players", "life", "total"][..],
            PlayerAst::Target,
        ),
        (
            &["target", "player's", "life", "total"][..],
            PlayerAst::Target,
        ),
        (
            &["target", "players'", "life", "total"][..],
            PlayerAst::Target,
        ),
        (
            &["target", "opponent", "life", "total"][..],
            PlayerAst::TargetOpponent,
        ),
        (
            &["target", "opponents", "life", "total"][..],
            PlayerAst::TargetOpponent,
        ),
        (
            &["target", "opponent's", "life", "total"][..],
            PlayerAst::TargetOpponent,
        ),
        (
            &["target", "opponents'", "life", "total"][..],
            PlayerAst::TargetOpponent,
        ),
        (
            &["an", "opponent", "life", "total"][..],
            PlayerAst::Opponent,
        ),
        (&["opponent", "life", "total"][..], PlayerAst::Opponent),
        (&["opponents", "life", "total"][..], PlayerAst::Opponent),
    ];
    candidates
        .iter()
        .find_map(|(words, player)| exact_shape(tokens, words).then_some(*player))
}

fn source_stat_operand(tokens: &[OwnedLexToken]) -> Option<ExchangeValueKindShape> {
    let source_heads = [
        &["its"][..],
        &["this"][..],
        &["thiss"][..],
        &["this's"][..],
        &["this", "creature"][..],
        &["this", "creature's"][..],
        &["thiss", "creature"][..],
        &["thiss", "creature's"][..],
        &["this", "creatures"][..],
        &["thiss", "creatures"][..],
    ];
    for head in source_heads {
        let mut power = head.to_vec();
        power.push("power");
        if exact_shape(tokens, &power) {
            return Some(ExchangeValueKindShape::Power);
        }
        let mut toughness = head.to_vec();
        toughness.push("toughness");
        if exact_shape(tokens, &toughness) {
            return Some(ExchangeValueKindShape::Toughness);
        }
    }

    let words = parser_token_word_refs(tokens);
    let (source_words, kind) = match words.split_last()? {
        (&"power", source_words) => (source_words, ExchangeValueKindShape::Power),
        (&"toughness", source_words) => (source_words, ExchangeValueKindShape::Toughness),
        _ => return None,
    };
    crate::util::source_reference_surface_for_possessive_words(source_words).map(|_| kind)
}

fn target_stat_operand(tokens: &[OwnedLexToken]) -> Option<ExchangeValueOperandShape<'_>> {
    let prefixes = [
        (&["the", "power", "of"][..], ExchangeValueKindShape::Power),
        (&["power", "of"][..], ExchangeValueKindShape::Power),
        (
            &["the", "toughness", "of"][..],
            ExchangeValueKindShape::Toughness,
        ),
        (&["toughness", "of"][..], ExchangeValueKindShape::Toughness),
    ];
    for (prefix, kind) in prefixes {
        let parser = dynamic_phrase(prefix);
        if let Some((_, rest)) = primitives::parse_prefix(tokens, parser) {
            return Some(ExchangeValueOperandShape::TargetStat {
                target_tokens: rest,
                kind,
            });
        }
    }
    None
}

fn classify_value_operand(tokens: &[OwnedLexToken]) -> Option<ExchangeValueOperandShape<'_>> {
    if let Some(player) = life_total_operand(tokens) {
        return Some(ExchangeValueOperandShape::LifeTotal(player));
    }
    if let Some(kind) = source_stat_operand(tokens) {
        return Some(ExchangeValueOperandShape::SourceStat {
            source_tokens: tokens,
            kind,
        });
    }
    target_stat_operand(tokens)
}

pub fn parse_exchange_value_operands(
    tokens: &[OwnedLexToken],
) -> Option<(ExchangeValueOperandShape<'_>, ExchangeValueOperandShape<'_>)> {
    let (left, right) = split_on(tokens, &["with"]).or_else(|| split_on(tokens, &["and"]))?;
    Some((
        classify_value_operand(left)?,
        classify_value_operand(right)?,
    ))
}

pub fn parse_exchange_clause_shape(tokens: &[OwnedLexToken]) -> Option<ExchangeClauseShape<'_>> {
    let tokens = primitives::parse_prefix(tokens, exchange_verb)
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    if exact_shape(tokens, &["life", "totals"]) {
        return Some(ExchangeClauseShape::LifeTotalsOnly);
    }
    if let Some((_, partner_tokens)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["life", "totals", "with"]))
    {
        return Some(ExchangeClauseShape::LifeTotalsWith(partner_shape(
            partner_tokens,
        )?));
    }
    let text_box_prefixes = [
        &["the", "text", "boxes", "of"][..],
        &["text", "boxes", "of"][..],
    ];
    for prefix in text_box_prefixes {
        if let Some((_, target_tokens)) = primitives::parse_prefix(tokens, dynamic_phrase(prefix)) {
            return Some(ExchangeClauseShape::TextBoxes { target_tokens });
        }
    }
    if let Ok((player, zone1, zone2)) =
        primitives::parse_all(tokens, parse_zone_exchange_lexed, "exchange zones")
    {
        return Some(ExchangeClauseShape::Zones {
            player,
            zone1,
            zone2,
        });
    }
    if let Some(control) = parse_control_shape(tokens) {
        return Some(ExchangeClauseShape::Control(control));
    }
    if marker_anywhere(
        tokens,
        alt((
            primitives::kw("life"),
            primitives::kw("power"),
            primitives::kw("toughness"),
        )),
    ) {
        return Some(ExchangeClauseShape::Values { tokens });
    }
    None
}

#[cfg(test)]
#[path = "exchange_shapes/tests.rs"]
mod tests;
