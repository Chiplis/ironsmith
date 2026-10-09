//! "For each basic land type among lands you control, this creature has
//! landwalk of that type" as five land-type-conditioned landwalk abilities
//! (CR 702.14). Source-authored, deliberately unrun.
#[path = "p02_line_families/compile.rs"]
mod compile;

const TREEFOLK: &str = "Mana cost: {4}{G}\nType: Creature — Treefolk\nPower/Toughness: 2/6\nDomain — For each basic land type among lands you control, this creature has landwalk of that type. (It can't be blocked as long as defending player controls a land of that type.)";

#[test]
fn magnigoth_treefolk_has_each_landwalk_while_you_control_that_land_type() {
    for definition in compile::compile_both("Magnigoth Treefolk", TREEFOLK) {
        let debug = format!("{definition:?}");
        for land_type in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            assert!(debug.contains(land_type), "{land_type}: {debug}");
        }
        assert_eq!(debug.matches("YouControl").count(), 5, "{debug}");
    }
}
