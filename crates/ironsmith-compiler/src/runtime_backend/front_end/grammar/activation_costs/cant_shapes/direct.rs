use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::mana::ManaSymbol;

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::{leaf, primitives};

/// A complete, self-contained restriction surface whose semantic meaning does
/// not depend on a later filter or condition parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectCantFact {
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

pub(crate) fn parse_direct_cant_fact_tokens(tokens: &[OwnedLexToken]) -> Option<DirectCantFact> {
    primitives::parse_all(tokens, parse_direct_cant_fact_lexed, "direct cant fact").ok()
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
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parse(raw: &str) -> Option<DirectCantFact> {
        let tokens = lex_line(raw, 0).expect("lex direct cant fixture");
        parse_direct_cant_fact_tokens(&tokens)
    }

    #[test]
    fn parses_complete_direct_cant_alternatives() {
        let cases = [
            (
                "If a player would gain life, that player gains no life instead.",
                DirectCantFact::PlayerWouldGainNoLifeInstead,
            ),
            (
                "If a player would gain life, they gain no life instead.",
                DirectCantFact::PlayerWouldGainNoLifeInstead,
            ),
            (
                "Players can't gain life.",
                DirectCantFact::PlayersCantGainLife,
            ),
            (
                "Players can't search libraries.",
                DirectCantFact::PlayersCantSearchLibraries,
            ),
            (
                "Damage can't be prevented.",
                DirectCantFact::DamageCantBePrevented,
            ),
            ("You can't lose the game.", DirectCantFact::YouCantLoseGame),
            (
                "Your opponents can't win the game.",
                DirectCantFact::OpponentsCantWinGame,
            ),
            (
                "Your life total can't change.",
                DirectCantFact::YourLifeTotalCantChange,
            ),
            (
                "Your opponents can't cast spells.",
                DirectCantFact::OpponentsCantCastSpells,
            ),
            (
                "Your opponents can't draw more than one card each turn.",
                DirectCantFact::OpponentsCantDrawExtraCards,
            ),
            (
                "Counters can't be put on this permanent.",
                DirectCantFact::CantHaveCountersPlaced,
            ),
            (
                "This spell can't be countered.",
                DirectCantFact::ThisSpellCantBeCountered,
            ),
            (
                "This creature can't attack.",
                DirectCantFact::SourceCantAttack,
            ),
            ("This token can't block.", DirectCantFact::SourceCantBlock),
            (
                "This creature can't attack its owner.",
                DirectCantFact::SourceCantAttackItsOwner,
            ),
            (
                "Permanents you control can't be sacrificed.",
                DirectCantFact::PermanentsYouControlCantBeSacrificed,
            ),
            ("Can't be blocked.", DirectCantFact::SourceCantBeBlocked),
            (
                "This can't be blocked this turn.",
                DirectCantFact::TemporaryUnblockable,
            ),
            (
                "This creature can't attack alone.",
                DirectCantFact::SourceCantAttackAlone,
            ),
            (
                "This token can't attack or block.",
                DirectCantFact::SourceCantAttackOrBlock,
            ),
            (
                "This can't attack or block alone.",
                DirectCantFact::SourceCantAttackOrBlockAlone,
            ),
            (
                "This creature can't attack or block unless you have max speed.",
                DirectCantFact::SourceCantAttackOrBlockUnlessMaxSpeed,
            ),
            (
                "Creatures can't attack you unless their controller pays {X} for each creature they control that's attacking you, where X is the number of basic land types among lands you control.",
                DirectCantFact::DomainAttackTax,
            ),
        ];

        for (raw, expected) in cases {
            assert_eq!(parse(raw), Some(expected), "fixture: {raw}");
        }
    }

    #[test]
    fn accepts_legacy_complete_surface_alternatives() {
        let cases = [
            ("This can't attack.", DirectCantFact::SourceCantAttack),
            (
                "This token can't attack alone.",
                DirectCantFact::SourceCantAttackAlone,
            ),
            (
                "This creature can't block.",
                DirectCantFact::SourceCantBlock,
            ),
            (
                "This token can't be blocked.",
                DirectCantFact::SourceCantBeBlocked,
            ),
            (
                "This creature can't be blocked this turn.",
                DirectCantFact::TemporaryUnblockable,
            ),
            (
                "Can't be blocked this turn.",
                DirectCantFact::TemporaryUnblockable,
            ),
            (
                "This can't attack or block unless you have max speed.",
                DirectCantFact::SourceCantAttackOrBlockUnlessMaxSpeed,
            ),
            (
                "Creatures cannot attack you unless their controller pays X for each creature they control thats attacking you where X is the number of basic land type among lands you control",
                DirectCantFact::DomainAttackTax,
            ),
        ];

        for (raw, expected) in cases {
            assert_eq!(parse(raw), Some(expected), "fixture: {raw}");
        }
    }

    #[test]
    fn rejects_direct_cant_prefix_near_misses() {
        for raw in [
            "Players can't gain life this turn.",
            "Player can't search libraries.",
            "Damage can't be prevented by spells.",
            "This spell can't be countered this turn.",
            "This creature can't attack its controller.",
            "This token can't be blocked this turn.",
            "This creature can't attack or block unless you have speed.",
            "Creatures can't attack you unless their controller pays {X}.",
        ] {
            assert_eq!(parse(raw), None, "near miss: {raw}");
        }
    }
}
