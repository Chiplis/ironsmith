//! cf8 p01: values that were silently simplified — 2ˣ read as 2, "card type
//! among spells" read as a spell count, and a set-wide "card types among
//! them" read per card. Source-authored, deliberately unrun.
use ironsmith::effect::Value;

#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn mathemagics_draws_two_to_the_x() {
    for definition in support::definitions("Mathemagics") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("PowerOfTwo(X)"), "{debug}");
        let text = support::rendered(&definition);
        assert!(text.contains("2ˣ"), "{text}");
    }
    let _ = Value::PowerOfTwo(Box::new(Value::X));
}

#[test]
fn april_oneil_counts_card_types_among_spells_cast() {
    for definition in support::definitions("April O'Neil, Hacktivist") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CardTypesAmongSpellsCastThisTurn"), "{debug}");
        let text = support::rendered(&definition);
        assert!(text.contains("card types among spells you've cast this turn"), "{text}");
    }
}

#[test]
fn winter_constrains_the_exiled_set_not_each_card() {
    for definition in support::definitions("Winter, Cynical Opportunist") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("DistinctCardTypes"), "{debug}");
        assert!(!debug.contains("card_type_count: Some"), "{debug}");
    }
}
