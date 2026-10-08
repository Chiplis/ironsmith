//! Trigger subjects with relative clauses (p12-other). Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

#[test]
fn entered_this_turn_relative_clause_is_a_typed_subject_predicate() {
    for name in ["Goro-Goro and Satoru", "Whirlwind, Killer Cyclone"] {
        for definition in support::definitions(name) {
            let debug = support::debug(&definition);
            assert!(
                debug.contains("entered_battlefield_this_turn: true"),
                "{name}: the subject must keep 'that entered this turn'"
            );
            assert!(debug.contains("controller: Some(You)"), "{name}");
        }
    }
    // The relative clause is never dropped into an unqualified subject.
    for definition in support::definitions("Goro-Goro and Satoru") {
        assert_eq!(support::triggered_count(&definition), 1);
    }
}

#[test]
fn enchanted_by_aura_relative_clause_and_chosen_color_source() {
    for definition in support::definitions("Killian, Decisive Mentor") {
        assert_eq!(support::triggered_count(&definition), 2);
        let debug = support::debug(&definition);
        assert!(debug.contains("Aura"), "the attached-Aura predicate is retained");
    }
    for definition in support::definitions("Circle of Affliction") {
        let debug = support::debug(&definition);
        assert!(
            debug.contains("chosen_color: true"),
            "the damage source is restricted to the chosen color (CR 607.2a)"
        );
    }
}
