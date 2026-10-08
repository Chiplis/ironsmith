//! cf8 p01 round 3: targets "chosen at random" keep ChoiceCount.random and
//! are picked by the game at announcement. Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn random_targets_compile_and_render() {
    for (name, phrase) in [
        ("Goblin Test Pilot", "any target chosen at random"),
        ("Witch Hunt", "target opponent chosen at random"),
    ] {
        for definition in support::definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("random: true"), "{name}: {debug}");
            let text = support::rendered(&definition);
            assert!(text.contains(phrase), "{name}: {text}");
        }
    }
}
