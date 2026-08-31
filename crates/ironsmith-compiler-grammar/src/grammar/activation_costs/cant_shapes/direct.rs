use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::lexer::parser_token_word_refs;
use crate::mana::ManaSymbol;
use crate::object::CounterType;

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterLimitFact {
    pub counter_type: CounterType,
    pub maximum: u32,
}

/// Parse "this can't have more than N [kind] counters on it" without
/// conflating the static cap with a prohibition on placing counters.
pub fn parse_counter_limit_fact_tokens(tokens: &[OwnedLexToken]) -> Option<CounterLimitFact> {
    let words = parser_token_word_refs(tokens);
    let cant_idx = crate::word_primitives::select_word_position(&words, |word| {
        matches!(word, "can't" | "cant" | "cannot")
    })?;
    if !crate::word_primitives::parse_any_sequence_complete(
        words.get(..cant_idx)?,
        &[
            &["this"],
            &["this", "permanent"],
            &["this", "creature"],
            &["this", "token"],
        ],
    ) || !words.get(cant_idx + 1..).is_some_and(|tail| {
        crate::word_primitives::parse_sequence_prefix(tail, &["have", "more", "than"])
    }) {
        return None;
    }

    let maximum_idx = cant_idx + 4;
    let maximum = crate::grammar::primitives::probe_shape(leaf::parse_number_complete(
        words.get(maximum_idx)?,
    ))?;
    let counter_noun_idx = maximum_idx
        + 1
        + crate::word_primitives::select_word_position(words.get(maximum_idx + 1..)?, |word| {
            matches!(word, "counter" | "counters")
        })?;
    let tail = words.get(counter_noun_idx + 1..)?;
    if !crate::word_primitives::parse_any_sequence_complete(
        tail,
        &[&["on", "it"], &["on", "this"], &["on", "this", "permanent"]],
    ) {
        return None;
    }
    let counter_type = super::super::super::filters::parse_counter_type_words(
        words.get(maximum_idx + 1..=counter_noun_idx)?,
    )?;
    Some(CounterLimitFact {
        counter_type,
        maximum,
    })
}

/// A complete, self-contained restriction surface whose semantic meaning does
/// not depend on a later filter or condition parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectCantFact {
    PlayerWouldGainNoLifeInstead,
    PlayersCantGainLife,
    PlayersCantSearchLibraries,
    DamageCantBePrevented,
    YouCantLoseGame,
    OpponentsCantWinGame,
    YourLifeTotalCantChange,
    OpponentsCantCastSpells,
    OpponentsCantDrawExtraCards,
    CantHaveCountersPlaced,
    ThisSpellCantBeCountered,
    SourceCantAttack,
    SourceCantBlock,
    SourceCantAttackItsOwner,
    PermanentsYouControlCantBeSacrificed,
    SourceCantBeBlocked,
    TemporaryUnblockable,
    SourceCantAttackAlone,
    SourceCantAttackOrBlock,
    SourceCantAttackOrBlockAlone,
    SourceCantAttackOrBlockUnlessMaxSpeed,
    DomainAttackTax,
}

pub fn parse_direct_cant_fact_tokens(tokens: &[OwnedLexToken]) -> Option<DirectCantFact> {
    crate::grammar::primitives::probe_all(tokens, parse_direct_cant_fact_lexed, "direct cant fact")
}

fn parse_direct_cant_fact_lexed<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    let fact = alt((
        parse_special_direct_cant_fact,
        parse_global_direct_cant_fact,
        parse_source_direct_cant_fact,
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(fact)
}

fn parse_special_direct_cant_fact<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    alt((
        parse_max_speed_restriction,
        parse_temporary_unblockable,
        parse_player_gain_life_replacement,
        parse_domain_attack_tax,
    ))
    .parse_next(input)
}

fn parse_global_direct_cant_fact<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    alt((
        parse_player_global_direct_cant_fact,
        parse_other_global_direct_cant_fact,
    ))
    .parse_next(input)
}

fn parse_player_global_direct_cant_fact<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    alt((
        (
            primitives::kw("players"),
            parse_cant,
            primitives::phrase(&["gain", "life"]),
        )
            .value(DirectCantFact::PlayersCantGainLife),
        (
            primitives::kw("players"),
            parse_cant,
            primitives::phrase(&["search", "libraries"]),
        )
            .value(DirectCantFact::PlayersCantSearchLibraries),
        (
            primitives::kw("damage"),
            parse_cant,
            primitives::phrase(&["be", "prevented"]),
        )
            .value(DirectCantFact::DamageCantBePrevented),
        (
            primitives::kw("you"),
            parse_cant,
            primitives::phrase(&["lose", "the", "game"]),
        )
            .value(DirectCantFact::YouCantLoseGame),
        (
            primitives::phrase(&["your", "opponents"]),
            parse_cant,
            primitives::phrase(&["win", "the", "game"]),
        )
            .value(DirectCantFact::OpponentsCantWinGame),
        (
            primitives::phrase(&["your", "life", "total"]),
            parse_cant,
            primitives::kw("change"),
        )
            .value(DirectCantFact::YourLifeTotalCantChange),
        (
            primitives::phrase(&["your", "opponents"]),
            parse_cant,
            primitives::phrase(&["cast", "spells"]),
        )
            .value(DirectCantFact::OpponentsCantCastSpells),
        (
            primitives::phrase(&["your", "opponents"]),
            parse_cant,
            primitives::phrase(&["draw", "more", "than", "one", "card", "each", "turn"]),
        )
            .value(DirectCantFact::OpponentsCantDrawExtraCards),
    ))
    .parse_next(input)
}

fn parse_other_global_direct_cant_fact<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    alt((
        (
            primitives::kw("counters"),
            parse_cant,
            primitives::phrase(&["be", "put", "on", "this", "permanent"]),
        )
            .value(DirectCantFact::CantHaveCountersPlaced),
        (
            primitives::phrase(&["this", "spell"]),
            parse_cant,
            primitives::phrase(&["be", "countered"]),
        )
            .value(DirectCantFact::ThisSpellCantBeCountered),
        (
            primitives::phrase(&["permanents", "you", "control"]),
            parse_cant,
            primitives::phrase(&["be", "sacrificed"]),
        )
            .value(DirectCantFact::PermanentsYouControlCantBeSacrificed),
    ))
    .parse_next(input)
}

fn parse_source_direct_cant_fact<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    alt((
        (
            parse_source_subject,
            parse_cant,
            primitives::phrase(&["attack", "or", "block", "alone"]),
        )
            .value(DirectCantFact::SourceCantAttackOrBlockAlone),
        (
            parse_source_subject,
            parse_cant,
            primitives::phrase(&["attack", "or", "block"]),
        )
            .value(DirectCantFact::SourceCantAttackOrBlock),
        (
            primitives::phrase(&["this", "creature"]),
            parse_cant,
            primitives::phrase(&["attack", "its", "owner"]),
        )
            .value(DirectCantFact::SourceCantAttackItsOwner),
        (
            parse_source_subject,
            parse_cant,
            primitives::phrase(&["attack", "alone"]),
        )
            .value(DirectCantFact::SourceCantAttackAlone),
        (parse_source_subject, parse_cant, primitives::kw("attack"))
            .value(DirectCantFact::SourceCantAttack),
        (parse_source_subject, parse_cant, primitives::kw("block"))
            .value(DirectCantFact::SourceCantBlock),
        (
            parse_source_or_bare_subject,
            parse_cant,
            primitives::phrase(&["be", "blocked"]),
        )
            .value(DirectCantFact::SourceCantBeBlocked),
    ))
    .parse_next(input)
}

fn parse_max_speed_restriction<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    parse_source_without_token_subject.parse_next(input)?;
    parse_cant.parse_next(input)?;
    primitives::phrase(&[
        "attack", "or", "block", "unless", "you", "have", "max", "speed",
    ])
    .parse_next(input)?;
    Ok(DirectCantFact::SourceCantAttackOrBlockUnlessMaxSpeed)
}

fn parse_temporary_unblockable<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    parse_source_without_token_or_bare_subject.parse_next(input)?;
    parse_cant.parse_next(input)?;
    primitives::phrase(&["be", "blocked", "this", "turn"]).parse_next(input)?;
    Ok(DirectCantFact::TemporaryUnblockable)
}

fn parse_player_gain_life_replacement<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    primitives::phrase(&["if", "a", "player", "would", "gain", "life"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    alt((
        primitives::phrase(&["that", "player", "gains"]),
        primitives::phrase(&["they", "gain"]),
    ))
    .parse_next(input)?;
    primitives::phrase(&["no", "life", "instead"]).parse_next(input)?;
    Ok(DirectCantFact::PlayerWouldGainNoLifeInstead)
}

fn parse_domain_attack_tax<'a>(input: &mut LexStream<'a>) -> WResult<DirectCantFact> {
    primitives::kw("creatures").parse_next(input)?;
    parse_cant.parse_next(input)?;
    primitives::phrase(&["attack", "you", "unless", "their", "controller", "pays"])
        .parse_next(input)?;
    parse_surface_x.parse_next(input)?;
    primitives::phrase(&["for", "each", "creature", "they", "control"]).parse_next(input)?;
    alt((primitives::kw("that's"), primitives::kw("thats"))).parse_next(input)?;
    primitives::phrase(&["attacking", "you"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["where"]).parse_next(input)?;
    parse_surface_x.parse_next(input)?;
    primitives::phrase(&["is", "the", "number", "of", "basic", "land"]).parse_next(input)?;
    alt((primitives::kw("types"), primitives::kw("type"))).parse_next(input)?;
    primitives::phrase(&["among", "lands", "you", "control"]).parse_next(input)?;
    Ok(DirectCantFact::DomainAttackTax)
}

fn parse_cant<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("can't"),
        primitives::kw("cant"),
        primitives::kw("cannot"),
    ))
    .void()
    .parse_next(input)
}

fn parse_source_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "creature"]),
        primitives::phrase(&["this", "token"]),
        primitives::kw("this").void(),
    ))
    .void()
    .parse_next(input)
}

fn parse_source_without_token_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "creature"]),
        primitives::kw("this").void(),
    ))
    .void()
    .parse_next(input)
}

fn parse_source_or_bare_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(parse_source_subject).void().parse_next(input)
}

fn parse_source_without_token_or_bare_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    opt(parse_source_without_token_subject)
        .void()
        .parse_next(input)
}

fn parse_surface_x<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    leaf::parse_leaf_surface_mana_pip_lexed
        .verify(|pip| match pip {
            leaf::LeafManaPipToken::ManaGroup(symbols) => {
                symbols.first().copied() == Some(ManaSymbol::X) && symbols.get(1).is_none()
            }
            leaf::LeafManaPipToken::LegacyBare(symbol) => *symbol == ManaSymbol::X,
        })
        .void()
        .parse_next(input)
}

#[cfg(test)]
#[path = "direct_inline_tests.rs"]
mod tests;
