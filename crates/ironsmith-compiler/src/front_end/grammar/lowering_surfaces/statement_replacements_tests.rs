use super::*;
use crate::lexer::lex_line;

fn parse(raw: &str) -> Option<StatementReplacementSurfaceKind> {
    parse_statement_replacement_surface_tokens(&lex_line(raw, 0).unwrap())
}

#[test]
fn parses_complete_replacement_facts() {
    let cases = [
        (
            "If this spell was bargained, put one of those cards with mana value 4 or less onto the battlefield instead of putting it into your hand.",
            StatementReplacementSurfaceKind::BargainedReturnToBattlefield,
        ),
        (
            "If this spell was kicked, put two of those cards into your hand instead. Otherwise, put one of those cards into your hand.",
            StatementReplacementSurfaceKind::KickedCountOverride,
        ),
        (
            "If this spell was kicked, put those cards onto the battlefield instead of putting them into your hand.",
            StatementReplacementSurfaceKind::KickedMultiZoneToBattlefield,
        ),
        (
            "Clash with an opponent, then return target creature to its owner's hand. If you win, you may put that creature on top of its owner's library instead.",
            StatementReplacementSurfaceKind::ClashWinTopOfLibrary,
        ),
        (
            "If a creature died this turn, put that card onto the battlefield instead of putting it into your hand.",
            StatementReplacementSurfaceKind::MorbidSearchToBattlefield,
        ),
        (
            "You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.",
            StatementReplacementSurfaceKind::MorbidSearchToBattlefield,
        ),
    ];
    for (raw, expected) in cases {
        assert_eq!(parse(raw), Some(expected), "fixture: {raw}");
    }
}

#[test]
fn rejects_semantically_incomplete_near_misses() {
    for raw in [
        "If this spell was bargained, put one of those cards into your hand.",
        "If this spell was kicked, put those cards into your hand.",
        "Clash with an opponent. Put it on top of its owner's library instead.",
        "Put that card onto the battlefield instead of putting it into your hand.",
    ] {
        assert_eq!(parse(raw), None, "near miss: {raw}");
    }
}
