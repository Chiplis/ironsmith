use super::*;

use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

use crate::cards::builders::{CardTextError, KeywordAction, PlayerAst};
use crate::filter::StackObjectKind;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NextSpellGrantAbilitySurface<'a> {
    CantBeCountered,
    Keyword(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NextSpellKeywordActionShape<'a> {
    Known(KeywordAction),
    SingleWord(&'a str),
}

fn next_spell_protection_action<'a>(input: &mut LexStream<'a>) -> WResult<KeywordAction> {
    primitives::phrase(&["protection", "from"]).parse_next(input)?;
    let value = primitives::word_parser_text.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    if value == "everything" {
        return Ok(KeywordAction::ProtectionFromEverything);
    }
    if let Ok(color) = super::super::leaf::parse_leaf_color_complete(value) {
        return Ok(KeywordAction::ProtectionFrom(color));
    }
    if let Ok(card_type) = super::super::leaf::parse_leaf_card_type_complete(value) {
        return Ok(KeywordAction::ProtectionFromCardType(card_type));
    }
    if let Ok(subtype) = super::super::leaf::parse_leaf_subtype_flexible_complete(value) {
        return Ok(KeywordAction::ProtectionFromSubtype(subtype));
    }
    Err(primitives::backtrack_err(
        "next-spell protection",
        "color, card type, subtype, or everything",
    ))
}

fn next_spell_keyword_action<'a>(
    input: &mut LexStream<'a>,
) -> WResult<NextSpellKeywordActionShape<'a>> {
    alt((
        alt((
            (
                primitives::phrase(&["first", "strike"]),
                primitives::sentence_end(),
            )
                .value(NextSpellKeywordActionShape::Known(
                    KeywordAction::FirstStrike,
                )),
            (
                primitives::phrase(&["double", "strike"]),
                primitives::sentence_end(),
            )
                .value(NextSpellKeywordActionShape::Known(
                    KeywordAction::DoubleStrike,
                )),
            (
                primitives::phrase(&["battle", "cry"]),
                primitives::sentence_end(),
            )
                .value(NextSpellKeywordActionShape::Known(KeywordAction::BattleCry)),
            (
                primitives::phrase(&["split", "second"]),
                primitives::sentence_end(),
            )
                .value(NextSpellKeywordActionShape::Known(
                    KeywordAction::SplitSecond,
                )),
        )),
        alt((
            (
                primitives::phrase(&["read", "ahead"]),
                primitives::sentence_end(),
            )
                .value(NextSpellKeywordActionShape::Known(KeywordAction::ReadAhead)),
            (
                primitives::phrase(&["umbra", "armor"]),
                primitives::sentence_end(),
            )
                .value(NextSpellKeywordActionShape::Known(
                    KeywordAction::UmbraArmor,
                )),
            (
                primitives::phrase(&["doctor", "companion"]),
                primitives::sentence_end(),
            )
                .value(NextSpellKeywordActionShape::Known(KeywordAction::Marker(
                    "doctor companion",
                ))),
            next_spell_protection_action.map(NextSpellKeywordActionShape::Known),
            (primitives::word_parser_text, primitives::sentence_end())
                .map(|(word, _)| NextSpellKeywordActionShape::SingleWord(word)),
        )),
    ))
    .parse_next(input)
}

pub(crate) fn parse_next_spell_keyword_action_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NextSpellKeywordActionShape<'_>> {
    primitives::parse_all(
        tokens,
        next_spell_keyword_action,
        "next-spell keyword action",
    )
    .ok()
}

#[derive(Debug, Clone)]
pub(crate) struct NextSpellGrantShape<'a> {
    pub(crate) player: PlayerAst,
    pub(crate) filters: Vec<ObjectFilter>,
    pub(crate) ability: NextSpellGrantAbilitySurface<'a>,
}

#[derive(Debug, Clone)]
struct CasterSpec {
    player: PlayerAst,
    cast_by: PlayerFilter,
}

#[derive(Debug, Clone)]
struct RawNextSpellGrant<'a> {
    player: PlayerAst,
    cast_by: PlayerFilter,
    first_subject: &'a [OwnedLexToken],
    second_subject: Option<&'a [OwnedLexToken]>,
    ability: NextSpellGrantAbilitySurface<'a>,
}

fn caster_suffix<'a>(input: &mut LexStream<'a>) -> WResult<CasterSpec> {
    alt((
        primitives::phrase(&["you", "cast"]).value(CasterSpec {
            player: PlayerAst::You,
            cast_by: PlayerFilter::You,
        }),
        primitives::phrase(&["they", "cast"]).value(CasterSpec {
            player: PlayerAst::That,
            cast_by: PlayerFilter::IteratedPlayer,
        }),
        primitives::phrase(&["that", "player", "cast"]).value(CasterSpec {
            player: PlayerAst::That,
            cast_by: PlayerFilter::IteratedPlayer,
        }),
        primitives::phrase(&["target", "player", "cast"]).value(CasterSpec {
            player: PlayerAst::Target,
            cast_by: PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        }),
        primitives::phrase(&["target", "opponent", "cast"]).value(CasterSpec {
            player: PlayerAst::TargetOpponent,
            cast_by: PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
        }),
        primitives::phrase(&["opponent", "cast"]).value(CasterSpec {
            player: PlayerAst::Opponent,
            cast_by: PlayerFilter::Opponent,
        }),
        primitives::phrase(&["opponents", "cast"]).value(CasterSpec {
            player: PlayerAst::Opponent,
            cast_by: PlayerFilter::Opponent,
        }),
    ))
    .parse_next(input)
}

fn when_caster<'a>(input: &mut LexStream<'a>) -> WResult<CasterSpec> {
    alt((
        primitives::kw("you").value(CasterSpec {
            player: PlayerAst::You,
            cast_by: PlayerFilter::You,
        }),
        primitives::phrase(&["an", "opponent"]).value(CasterSpec {
            player: PlayerAst::Opponent,
            cast_by: PlayerFilter::Opponent,
        }),
        primitives::kw("opponent").value(CasterSpec {
            player: PlayerAst::Opponent,
            cast_by: PlayerFilter::Opponent,
        }),
        primitives::phrase(&["that", "player"]).value(CasterSpec {
            player: PlayerAst::That,
            cast_by: PlayerFilter::IteratedPlayer,
        }),
        primitives::phrase(&["target", "player"]).value(CasterSpec {
            player: PlayerAst::Target,
            cast_by: PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        }),
        primitives::phrase(&["target", "opponent"]).value(CasterSpec {
            player: PlayerAst::TargetOpponent,
            cast_by: PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
        }),
    ))
    .parse_next(input)
}

fn cant_be_countered<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["cant", "be", "countered"]),
        primitives::phrase(&["can't", "be", "countered"]),
    ))
    .parse_next(input)
}

fn ability_tail<'a>(input: &mut LexStream<'a>) -> WResult<NextSpellGrantAbilitySurface<'a>> {
    let ability_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    if primitives::parse_all(
        ability_tokens,
        (cant_be_countered, winnow::combinator::eof).void(),
        "next-spell-cant-be-countered",
    )
    .is_ok()
    {
        Ok(NextSpellGrantAbilitySurface::CantBeCountered)
    } else {
        Ok(NextSpellGrantAbilitySurface::Keyword(ability_tokens))
    }
}

fn direct_cant_ability<'a>(input: &mut LexStream<'a>) -> WResult<NextSpellGrantAbilitySurface<'a>> {
    cant_be_countered.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(NextSpellGrantAbilitySurface::CantBeCountered)
}

fn parse_standard_next_spell_grant<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RawNextSpellGrant<'a>> {
    primitives::phrase(&["the", "next"]).parse_next(input)?;
    let first_subject = repeat_till(1.., any.void(), peek(caster_suffix))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let caster = caster_suffix.parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let ability = alt((
        (
            alt((primitives::kw("has"), primitives::kw("have"))).void(),
            ability_tail,
        )
            .map(|(_, ability)| ability),
        direct_cant_ability,
    ))
    .parse_next(input)?;
    Ok(RawNextSpellGrant {
        player: caster.player,
        cast_by: caster.cast_by,
        first_subject,
        second_subject: None,
        ability,
    })
}

fn parse_paired_next_spell_grant<'a>(input: &mut LexStream<'a>) -> WResult<RawNextSpellGrant<'a>> {
    primitives::phrase(&["the", "next"]).parse_next(input)?;
    let first_subject = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["and", "the", "next"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["and", "the", "next"]).parse_next(input)?;
    let second_subject = repeat_till(1.., any.void(), peek(caster_suffix))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    let caster = caster_suffix.parse_next(input)?;
    primitives::phrase(&["this", "turn", "each", "have"]).parse_next(input)?;
    let ability = ability_tail.parse_next(input)?;
    Ok(RawNextSpellGrant {
        player: caster.player,
        cast_by: caster.cast_by,
        first_subject,
        second_subject: Some(second_subject),
        ability,
    })
}

fn parse_when_next_spell_grant<'a>(input: &mut LexStream<'a>) -> WResult<RawNextSpellGrant<'a>> {
    primitives::kw("when").parse_next(input)?;
    let caster = when_caster.parse_next(input)?;
    primitives::phrase(&["next", "cast"]).parse_next(input)?;
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    )))
    .parse_next(input)?;
    let first_subject = repeat_till(1.., any.void(), peek(primitives::phrase(&["this", "turn"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    alt((
        primitives::phrase(&["it", "gains"]),
        primitives::phrase(&["it", "has"]),
    ))
    .parse_next(input)?;
    let ability = ability_tail.parse_next(input)?;
    Ok(RawNextSpellGrant {
        player: caster.player,
        cast_by: caster.cast_by,
        first_subject,
        second_subject: None,
        ability,
    })
}

fn parse_raw_next_spell_grant<'a>(input: &mut LexStream<'a>) -> WResult<RawNextSpellGrant<'a>> {
    alt((
        parse_paired_next_spell_grant,
        parse_when_next_spell_grant,
        parse_standard_next_spell_grant,
    ))
    .parse_next(input)
}

fn spell_filter(
    subject_tokens: &[OwnedLexToken],
    cast_by: PlayerFilter,
) -> Result<ObjectFilter, CardTextError> {
    let subject_words = crate::lexer::token_word_refs(subject_tokens);
    let mut filter =
        super::super::filters::parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    // A subject such as "an instant or sorcery spell from your hand" carries
    // two pieces of information: it will be a spell when the one-shot grant
    // is consumed, and its cast origin is the hand.  Keep an explicit origin
    // zone from the parsed subject; the runtime's temporary-spell matcher
    // compares such filters against the authoritative cast-origin snapshot.
    // Subjects with no origin clause still match the spell on the stack.
    if subject_words
        .windows(3)
        .any(|window| window == ["from", "your", "hand"])
    {
        filter.zone = Some(Zone::Hand);
        filter.owner = Some(PlayerFilter::You);
    } else if subject_words
        .windows(3)
        .any(|window| window == ["from", "your", "graveyard"])
    {
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
    } else if filter.zone.is_none() {
        filter.zone = Some(Zone::Stack);
    }
    filter.stack_kind = Some(StackObjectKind::Spell);
    filter.has_mana_cost = true;
    filter.cast_by = Some(cast_by);
    Ok(filter)
}

pub(crate) fn parse_next_spell_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<NextSpellGrantShape<'_>>, CardTextError> {
    let Some(raw) =
        primitives::parse_all(tokens, parse_raw_next_spell_grant, "next-spell-grant").ok()
    else {
        return Ok(None);
    };
    let mut filters = vec![spell_filter(raw.first_subject, raw.cast_by.clone())?];
    if let Some(second_subject) = raw.second_subject {
        filters.push(spell_filter(second_subject, raw.cast_by)?);
    }
    Ok(Some(NextSpellGrantShape {
        player: raw.player,
        filters,
        ability: raw.ability,
    }))
}

#[cfg(test)]
#[path = "next_spell_grants/tests.rs"]
mod tests;
