//! cf8 p01: remembered objects/players render by the action that produced
//! them, never by internal tag names. Source-authored, deliberately unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn remembered_references_render_without_internal_tags() {
    for (name, phrase) in [
        ("Mishra's Research Desk", "you may play that card"),
        ("Riverwheel Sweep", "you may play that card"),
        ("Strongbox Raider", "you may play that card"),
        ("Chrome Courier", "you put an artifact card into your hand this way"),
        ("Town Greeter", "you put a town card into your hand this way"),
        ("Mutalith Vortex Beast", "for each flip you lose"),
    ] {
        for definition in support::definitions(name) {
            let text = support::rendered(&definition);
            support::assert_no_internal_markers(name, &text);
            assert!(text.contains(phrase), "{name}: {text}");
        }
    }
}
