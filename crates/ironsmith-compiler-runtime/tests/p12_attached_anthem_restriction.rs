//! "Enchanted creature gets +0/+2 and can't be the target of spells."
//! Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

use ironsmith::ability::AbilityKind;

#[test]
fn spectral_shield_is_an_anthem_plus_a_targeting_restriction() {
    for definition in support::definitions("Spectral Shield") {
        let statics: Vec<_> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .collect();
        assert!(statics.iter().any(|ability| ability.contains("Anthem")), "{statics:?}");
        assert!(
            statics.iter().any(|ability| ability.contains("Target") && ability.contains("enchanted")),
            "the restriction applies to the enchanted creature: {statics:?}"
        );
    }
}
