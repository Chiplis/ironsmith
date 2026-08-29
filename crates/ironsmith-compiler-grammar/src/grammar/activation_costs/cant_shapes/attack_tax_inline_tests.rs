use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_typed_per_attacker_tax() {
    for (raw, amount, covers_planeswalkers) in [
        (
            "Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.",
            2,
            false,
        ),
        (
            "Creatures cannot attack you unless their controller pays 1 for each creature they control thats attacking you",
            1,
            false,
        ),
        (
            "Creatures can't attack you or planeswalkers you control unless their controller pays {2} for each of those creatures.",
            2,
            true,
        ),
    ] {
        let tokens = lex_line(raw, 0).unwrap();
        assert_eq!(
            parse_per_attacker_cant_tax_tokens(&tokens),
            Some(PerAttackerCantTaxFact {
                amount,
                covers_planeswalkers,
            })
        );
    }
}

#[test]
fn rejects_non_generic_or_incomplete_taxes() {
    for raw in [
        "Creatures can't attack you unless their controller pays {W} for each creature they control that's attacking you.",
        "Creatures can't attack you unless their controller pays {2}.",
    ] {
        let tokens = lex_line(raw, 0).unwrap();
        assert_eq!(parse_per_attacker_cant_tax_tokens(&tokens), None);
    }
}
