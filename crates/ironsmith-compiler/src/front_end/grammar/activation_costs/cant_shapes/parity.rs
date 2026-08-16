use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::filter::ParityRequirement;

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManaValueParityCantFact {
    OpponentsCantCastSpells(ParityRequirement),
    OpponentsCantBlockWithCreatures(ParityRequirement),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CantFallbackFact {
    SourceCantAttackOrBlockUnlessEvenCounters,
    SourceDamageDoubledForManaValueParity(ParityRequirement),
}

pub(crate) fn parse_mana_value_parity_cant_fact_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ManaValueParityCantFact> {
    primitives::parse_all(
        tokens,
        parse_mana_value_parity_cant_fact_lexed,
        "mana-value parity cant fact",
    )
    .ok()
}

pub(crate) fn parse_cant_fallback_fact_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CantFallbackFact> {
    primitives::parse_all(tokens, parse_cant_fallback_fact_lexed, "cant fallback fact").ok()
}

fn parse_mana_value_parity_cant_fact_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ManaValueParityCantFact> {
    primitives::phrase(&["your", "opponents"]).parse_next(input)?;
    parse_cant.parse_next(input)?;
    let fact = alt((
        parse_cast_parity_restriction,
        parse_block_parity_restriction,
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(fact)
}

fn parse_cast_parity_restriction<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ManaValueParityCantFact> {
    primitives::phrase(&["cast", "spells", "with"]).parse_next(input)?;
    let parity = parse_mana_value_parity.parse_next(input)?;
    primitives::phrase(&["mana", "values"]).parse_next(input)?;
    Ok(ManaValueParityCantFact::OpponentsCantCastSpells(parity))
}

fn parse_block_parity_restriction<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ManaValueParityCantFact> {
    primitives::phrase(&["block", "with", "creatures", "with"]).parse_next(input)?;
    let parity = parse_mana_value_parity.parse_next(input)?;
    primitives::phrase(&["mana", "values"]).parse_next(input)?;
    Ok(ManaValueParityCantFact::OpponentsCantBlockWithCreatures(
        parity,
    ))
}

fn parse_cant_fallback_fact_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CantFallbackFact> {
    let fact = alt((
        parse_even_counter_combat_restriction,
        parse_double_damage_parity_replacement,
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(fact)
}

fn parse_even_counter_combat_restriction<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantFallbackFact> {
    alt((
        primitives::phrase(&["this", "creature"]),
        primitives::kw("this").void(),
    ))
    .parse_next(input)?;
    parse_cant.parse_next(input)?;
    primitives::phrase(&[
        "attack", "or", "block", "unless", "it", "has", "an", "even", "number", "of", "counters",
        "on", "it",
    ])
    .parse_next(input)?;
    Ok(CantFallbackFact::SourceCantAttackOrBlockUnlessEvenCounters)
}

fn parse_double_damage_parity_replacement<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantFallbackFact> {
    primitives::kw("if").parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::phrase(&["source", "you", "control", "with"]).parse_next(input)?;
    parse_indefinite_article.parse_next(input)?;
    let parity = parse_mana_value_parity.parse_next(input)?;
    primitives::phrase(&["mana", "value", "would", "deal", "damage", "to"]).parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::kw("permanent").parse_next(input)?;
    primitives::kw("or").parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::kw("player").parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "it",
        "deals",
        "double",
        "that",
        "damage",
        "to",
        "that",
        "permanent",
        "or",
        "player",
        "instead",
    ])
    .parse_next(input)?;
    Ok(CantFallbackFact::SourceDamageDoubledForManaValueParity(
        parity,
    ))
}

fn parse_indefinite_article<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("a"), primitives::kw("an")))
        .void()
        .parse_next(input)
}

fn parse_mana_value_parity<'a>(input: &mut LexStream<'a>) -> WResult<ParityRequirement> {
    alt((
        primitives::kw("odd").value(ParityRequirement::Odd),
        primitives::kw("even").value(ParityRequirement::Even),
    ))
    .parse_next(input)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).expect("lex parity cant fixture")
    }

    #[test]
    fn parses_complete_mana_value_parity_restrictions() {
        let cases = [
            (
                "Your opponents can't cast spells with odd mana values.",
                ManaValueParityCantFact::OpponentsCantCastSpells(ParityRequirement::Odd),
            ),
            (
                "Your opponents can't cast spells with even mana values.",
                ManaValueParityCantFact::OpponentsCantCastSpells(ParityRequirement::Even),
            ),
            (
                "Your opponents can't block with creatures with odd mana values.",
                ManaValueParityCantFact::OpponentsCantBlockWithCreatures(ParityRequirement::Odd),
            ),
            (
                "Your opponents cannot block with creatures with even mana values.",
                ManaValueParityCantFact::OpponentsCantBlockWithCreatures(ParityRequirement::Even),
            ),
        ];

        for (raw, expected) in cases {
            assert_eq!(
                parse_mana_value_parity_cant_fact_tokens(&lex(raw)),
                Some(expected),
                "fixture: {raw}"
            );
        }
    }

    #[test]
    fn parses_complete_typed_fallback_surfaces() {
        let cases = [
            (
                "This creature can't attack or block unless it has an even number of counters on it.",
                CantFallbackFact::SourceCantAttackOrBlockUnlessEvenCounters,
            ),
            (
                "This cant attack or block unless it has an even number of counters on it",
                CantFallbackFact::SourceCantAttackOrBlockUnlessEvenCounters,
            ),
            (
                "If a source you control with an odd mana value would deal damage to a permanent or player, it deals double that damage to that permanent or player instead.",
                CantFallbackFact::SourceDamageDoubledForManaValueParity(ParityRequirement::Odd),
            ),
            (
                "If source you control with a even mana value would deal damage to a permanent or a player it deals double that damage to that permanent or player instead",
                CantFallbackFact::SourceDamageDoubledForManaValueParity(ParityRequirement::Even),
            ),
        ];

        for (raw, expected) in cases {
            assert_eq!(
                parse_cant_fallback_fact_tokens(&lex(raw)),
                Some(expected),
                "fixture: {raw}"
            );
        }
    }

    #[test]
    fn rejects_parity_and_fallback_near_misses() {
        for raw in [
            "Your opponents can't cast spells with mana values.",
            "Your opponents can't cast spells with even mana value.",
            "Your opponents can't block creatures with even mana values.",
            "Your opponents can't cast spells with even mana values this turn.",
        ] {
            assert_eq!(
                parse_mana_value_parity_cant_fact_tokens(&lex(raw)),
                None,
                "near miss: {raw}"
            );
        }

        for raw in [
            "This creature can't attack or block unless it has an odd number of counters on it.",
            "This creature can't attack unless it has an even number of counters on it.",
            "If a source you control with an odd mana value would deal damage, it deals double that damage instead.",
            "If a source you control with an odd mana value would deal damage to a permanent or player, it deals that damage to that permanent or player instead.",
        ] {
            assert_eq!(
                parse_cant_fallback_fact_tokens(&lex(raw)),
                None,
                "near miss: {raw}"
            );
        }
    }
}
