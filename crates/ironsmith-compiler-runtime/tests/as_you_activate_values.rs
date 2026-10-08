//! cf8 p01 round 3: "as you activate this ability" is kept as an
//! activation-time sampling hint on where-X values. Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn activation_time_where_x_values_keep_their_clause() {
    for name in ["Agility Bobblehead", "Endurance Bobblehead", "Lukka, Bound to Ruin"] {
        for definition in support::definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("AsYouActivateThisAbility"), "{name}: {debug}");
            let text = support::rendered(&definition);
            assert!(text.contains("as you activate this ability"), "{name}: {text}");
        }
    }
}
