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

/// Keeper of the Beasts: the target opponent must control more creatures than
/// you as you activate; on resolution only the opponent relation is rechecked
/// (CR 601.2c via 602.2b, 608.2b).
#[test]
fn keeper_of_the_beasts_targets_an_opponent_with_more_creatures_as_you_activate() {
    for definition in support::definitions("Keeper of the Beasts") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("OpponentWithMoreControlledObjectsThan"), "{debug}");
        assert!(debug.contains("as_you_activate: true"), "{debug}");
        assert!(debug.contains("CreateToken"), "{debug}");
        let text = support::rendered(&definition);
        assert!(
            text.contains("target opponent who controls more creatures than you do as you activate this ability"),
            "{text}"
        );
        assert!(!text.contains("target creature"), "{text}");
    }
}
