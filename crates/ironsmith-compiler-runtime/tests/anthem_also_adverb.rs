//! UNVALIDATED implementation-first coverage: the cumulative adverb in
//! "Creatures you control also get +1/+0 and have ..." (CR 613.4c: each
//! static ability applies independently; "also" never changes the subject).
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn jetmir_keeps_three_independent_conditional_anthems_and_grants() {
    let rows = common::rows(include_str!("../../../fixtures/anthem_also_adverb.json.fixture"));
    let row = common::row(&rows, "Jetmir, Nexus of Revels");
    assert_eq!(row["oracle_id"], "da72a4bc-ce6f-4b72-bc66-2ee33cfa87df");
    for definition in common::definitions(row) {
        let ids = common::static_ids(&definition);
        assert_eq!(ids.iter().filter(|id| **id == StaticAbilityId::Anthem).count(), 3, "{ids:?}");
        let lines = common::rendered(&definition);
        for (keyword, threshold) in [("vigilance", "three"), ("trample", "six"), ("double strike", "nine")] {
            assert!(lines.contains(keyword), "{lines}");
            assert!(lines.contains(&format!("{threshold} or more creatures")), "{lines}");
        }
        assert!(!lines.to_lowercase().contains("also creatures"), "{lines}");
    }
}

#[test]
fn also_is_not_accepted_as_a_whole_subject() {
    let text = "Mana cost: {1}\nType: Enchantment\nAlso get +1/+0.";
    assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Also probe", text, false).is_err());
}
