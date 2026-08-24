use super::*;
use crate::lexer::lex_line;

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
fn parses_typed_source_counter_limit() {
    let tokens = lex_line(
        "This creature can't have more than seven dream counters on it.",
        0,
    )
    .expect("lex counter limit");
    assert_eq!(
        parse_counter_limit_fact_tokens(&tokens),
        Some(CounterLimitFact {
            counter_type: CounterType::Dream,
            maximum: 7,
        })
    );
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
