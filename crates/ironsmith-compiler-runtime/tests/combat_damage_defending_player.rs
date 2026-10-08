//! UNVALIDATED implementation-first coverage: "deals combat damage to
//! defending player" trigger recipients (CR 506.2, 510.2).
use ironsmith::ability::AbilityKind;
use ironsmith::effects::{DealDamageEffect, DestroyEffect};

#[path = "p09_common/mod.rs"]
mod common;

fn fixture() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/combat_damage_defending_player.json.fixture"))
}

#[test]
fn defending_player_recipient_compiles_on_both_routes() {
    let rows = fixture();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        assert!(matches!(name, "Electryte" | "Latulla's Orders"), "unexpected cohort member {name}");
        for definition in common::definitions(row) {
            let triggers: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            }).collect();
            assert_eq!(triggers.len(), 1, "{name}");
            let display = triggers[0].trigger.display();
            assert!(display.contains("combat damage to"), "{name}: {display}");
            assert!(display.contains("defending player"), "{name}: {display}");
            assert!(!display.contains("a player"), "{name}: the recipient stays the defending player: {display}");
            let effects = common::all_effects(&definition);
            match name {
                "Electryte" => assert!(effects.iter().any(|effect| effect.downcast_ref::<DealDamageEffect>().is_some())),
                _ => assert!(effects.iter().any(|effect| effect.downcast_ref::<DestroyEffect>().is_some())),
            }
        }
    }
}
