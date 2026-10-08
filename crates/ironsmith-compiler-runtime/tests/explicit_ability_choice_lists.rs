//! cf8/p07: "gains your choice of A, B, or C" is one resolution-time choice
//! among the listed abilities; the list is never coordinated into separate
//! clauses (which misread "your choice of A" as a life gain).
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

const FIXTURE: &str = include_str!("../../../fixtures/explicit_ability_choice_lists.json.fixture");

fn debug_of(definition: &ironsmith::cards::CardDefinition) -> String {
    format!("{:?}", definition.abilities)
}

#[test]
fn explicit_choice_lists_compile_to_one_mode_choice() {
    let rows = support::rows(FIXTURE);
    for (name, oracle_id, options) in [
        ("Atraxa's Skitterfang", "8255b408-cc8c-453f-982c-f361b5559cec", &["Flying", "Vigilance", "Deathtouch", "Lifelink"][..]),
        ("Hunter's Axe", "d7d59fef-1401-464b-b1bb-ee5da92cde51", &["Trample", "Deathtouch"][..]),
    ] {
        let row = support::row(&rows, name);
        assert_eq!(row["oracle_id"], oracle_id);
        for definition in support::definitions(row) {
            let debug = debug_of(&definition);
            assert!(debug.contains("ChooseModeEffect"), "{name}: one choice among the abilities");
            assert!(!debug.contains("GainLifeEffect"), "{name}: no misread life gain");
            for option in options {
                assert!(debug.contains(option), "{name}: option {option} kept");
            }
        }
    }
}
