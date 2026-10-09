//! UNVALIDATED implementation-first coverage: "target creature token, player,
//! or planeswalker" is one target over three domains (CR 115.1).
use ironsmith::ability::AbilityKind;
use ironsmith::effects::DealDamageEffect;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn coalborn_entity_targets_a_creature_token_player_or_planeswalker() {
    let rows = common::rows(include_str!("../../../fixtures/mixed_token_player_planeswalker_targets.json.fixture"));
    let row = common::row(&rows, "Coalborn Entity");
    for definition in common::definitions(row) {
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        }).expect("damage ability");
        let debug = format!("{activated:?}");
        assert!(debug.contains("token: true"), "creature arm is token-only: {debug}");
        assert!(debug.contains("Planeswalker"), "{debug}");
        let effects = common::all_effects(&definition);
        assert!(effects.iter().any(|effect| effect.downcast_ref::<DealDamageEffect>().is_some()));
        let lines = common::rendered(&definition);
        assert!(lines.contains("token"), "{lines}");
        assert!(lines.contains("player"), "{lines}");
    }
}
