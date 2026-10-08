//! cf8 p01 round 3: a cast trigger replaces where the resolving spell goes
//! (CR 608.2n, 614.1a) instead of exiling it from the stack. Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn goliath_registers_an_exile_with_dream_counter_replacement() {
    for definition in support::definitions("Goliath Daydreamer") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("RegisterZoneReplacementEffect"), "{debug}");
        assert!(debug.contains("Dream"), "{debug}");
        let text = support::rendered(&definition);
        assert!(text.contains("exile that card with a dream counter on it instead of putting it into your graveyard as it resolves"), "{text}");
    }
}

#[test]
fn lilah_plots_only_the_card_the_replacement_exiled() {
    for definition in support::definitions("Lilah, Undefeated Slickshot") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("RegisterZoneReplacementEffect"), "{debug}");
        assert!(debug.contains("BecomePlotted"), "{debug}");
        let text = support::rendered(&definition);
        assert!(text.contains("if you do, it becomes plotted"), "{text}");
    }
}
