//! cf8 p01 round 5: command-zone mechanics whose engine support existed but
//! whose lines were never read or rendered. Unrun.
#[path = "p01_support/mod.rs"]
mod support;

/// Liesa: the commander tax (CR 903.8) is paid in life. The static functions
/// in the command zone, where the engine reads it while casting.
#[test]
fn liesa_pays_commander_tax_in_life() {
    for definition in support::definitions("Liesa, Shroud of Dusk") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CommanderTaxLifeSubstitution"), "{debug}");
        assert!(debug.contains("life_per_previous_cast: 2"), "{debug}");
        assert!(debug.contains("Command"), "{debug}");
        let text = support::rendered(&definition);
        assert!(
            text.contains("rather than pay {2} for each previous time you've cast this spell from the command zone this game, pay 2 life that many times"),
            "{text}"
        );
        assert!(!text.contains("pay {2} and you pay 2 life"), "{text}");
    }
}

/// The Ur-Dragon: the eminence cost reduction renders its command-zone or
/// battlefield condition instead of "the stated condition".
#[test]
fn ur_dragon_eminence_condition_renders() {
    for definition in support::definitions("The Ur-Dragon") {
        let text = support::rendered(&definition);
        assert!(!text.contains("stated condition"), "{text}");
        assert!(text.contains("is in the command zone or on the battlefield"), "{text}");
        assert!(text.contains("dragon spells you cast cost {1} less"), "{text}");
    }
}
