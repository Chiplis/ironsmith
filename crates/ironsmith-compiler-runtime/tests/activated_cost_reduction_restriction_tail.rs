//! UNVALIDATED implementation-first coverage: an activated ability whose
//! self cost-reduction sentence is followed by an activation restriction
//! (CR 601.2f cost reduction; CR 602.5b restrictions stay on the ability).
use ironsmith::ability::{AbilityKind, ActivationTiming};
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn restriction_sentence_stays_with_the_activated_ability() {
    let rows = common::rows(include_str!("../../../fixtures/activated_cost_reduction_restriction_tail.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let ids = common::static_ids(&definition);
            assert_eq!(ids.iter().filter(|id| **id == StaticAbilityId::ActivatedAbilityCostReduction).count(), 1, "{name}: {ids:?}");
            let activated: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) if !activated.is_mana_ability() => Some(activated),
                _ => None,
            }).collect();
            assert_eq!(activated.len(), 1, "{name}");
            let lines = common::rendered(&definition);
            match name {
                "The Lonely Mountain" => {
                    assert_eq!(activated[0].timing, ActivationTiming::SorcerySpeed, "{name}");
                    assert!(lines.contains("for each Equipment you control") || lines.contains("for each equipment you control"), "{lines}");
                }
                "Radha's Firebrand" => {
                    assert!(lines.contains("Activate only once each turn"), "{lines}");
                    assert!(lines.contains("basic land type"), "{lines}");
                }
                other => panic!("unexpected cohort member {other}"),
            }
            assert!(!lines.contains("costs {1} less to activate for each Equipment you control. Activate"), "{lines}");
        }
    }
}
