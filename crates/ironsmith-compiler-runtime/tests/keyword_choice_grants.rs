//! UNVALIDATED implementation-first coverage: "Choose <keyword list>. <Subject>
//! gain(s) that ability <duration>." is one resolution-time choice among the
//! listed keywords, granting only the chosen one.
use ironsmith::effects::ChooseModeEffect;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn choice_then_grant_lowers_to_one_modal_grant() {
    let rows = common::rows(include_str!("../../../fixtures/keyword_choice_grants.json.fixture"));
    assert_eq!(rows.len(), 3);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let (modes, keyword) = match name {
            "Angelic Skirmisher" => (3, "lifelink"),
            "Linvala, Shield of Sea Gate" => (2, "indestructible"),
            "Gabriel Angelfire" => (4, "trample"),
            other => panic!("unexpected cohort member {other}"),
        };
        for definition in common::definitions(row) {
            let effects = common::all_effects(&definition);
            let choices: Vec<_> = effects.iter().filter_map(|effect| effect.downcast_ref::<ChooseModeEffect>()).collect();
            assert_eq!(choices.len(), 1, "{name}");
            assert_eq!(choices[0].modes.len(), modes, "{name}");
            let lines = common::rendered(&definition);
            assert!(lines.contains(keyword), "{name}: {lines}");
        }
    }
}
