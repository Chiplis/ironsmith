//! cf8 p01: "of each basic land type" / "of each color" quantifiers keep
//! both their per-type semantics and their authored surface.
//! Source-authored, deliberately unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn coalition_victory_requires_every_basic_land_type_and_color() {
    for definition in support::definitions("Coalition Victory") {
        let text = support::rendered(&definition);
        assert!(
            text.contains("you control a land of each basic land type and a creature of each color"),
            "{text}"
        );
        let debug = format!("{definition:?}");
        // Five land-type leaves plus five color leaves.
        assert_eq!(debug.matches("PlayerControls {").count(), 10, "{debug}");
    }
}

#[test]
fn per_type_choices_and_domain_pump_render_their_quantifier() {
    for (name, phrase) in [
        ("Planar Overlay", "of each basic land type"),
        ("Sundering Titan", "choose a land of each basic land type"),
        ("Tromp the Domains", "+1/+1 for each basic land type among lands you control"),
    ] {
        for definition in support::definitions(name) {
            let text = support::rendered(&definition);
            assert!(text.contains(phrase), "{name}: {text}");
        }
    }
}
