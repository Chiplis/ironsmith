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

const GANDALF_OF_THE_SECRET_FIRE: &str = "Mana cost: {1}{U}{R}{W}\nType: Legendary Creature — Avatar Wizard\nPower/Toughness: 3/4\nWhenever you cast an instant or sorcery spell from your hand during an opponent's turn, exile that card with three time counters on it instead of putting it into your graveyard as it resolves. Then if the exiled card doesn't have suspend, it gains suspend. (At the beginning of your upkeep, remove a time counter. When the last is removed, you may play it without paying its mana cost.)";

/// Collateral: the suspend grant is a follow-up of the replacement, so it
/// reaches the exiled card (a new object, CR 400.7) rather than the spell on
/// the stack at trigger resolution.
#[test]
fn gandalf_grants_suspend_only_to_the_card_the_replacement_exiled() {
    for definition in
        support::definitions_for_text("Gandalf of the Secret Fire", GANDALF_OF_THE_SECRET_FIRE)
    {
        let debug = format!("{definition:?}");
        assert!(debug.contains("RegisterZoneReplacementEffect"), "{debug}");
        assert!(debug.contains("GainSuspendIfMissing"), "{debug}");
        assert!(debug.contains("Time"), "{debug}");
        // No resolution-time conditional over the spell on the stack.
        assert!(!debug.contains("ConditionalEffect"), "{debug}");
        let text = support::rendered(&definition);
        assert!(
            text.contains("exile that card with three time counters on it instead of putting it into your graveyard as it resolves. then if the exiled card doesn't have suspend, it gains suspend"),
            "{text}"
        );
    }
}
