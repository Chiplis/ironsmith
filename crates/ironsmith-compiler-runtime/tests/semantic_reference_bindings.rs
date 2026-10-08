//! cf8 p01: silent miscompiles where a remembered set or pronoun bound the
//! wrong object. Source-authored, deliberately unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn aurelias_fury_taps_only_the_creatures_dealt_damage() {
    for definition in support::definitions("Aurelia's Fury") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Aurelia's Fury", &text);
        assert!(text.contains("tap each creature dealt damage this way"), "{text}");
        assert!(text.contains("for each player dealt damage this way"), "{text}");
        let debug = format!("{:?}", definition.spell_effect);
        // TapAll over the remembered recipients is restricted to creatures.
        let tap = debug.split("TapEffect").nth(1).expect("tap effect");
        assert!(tap.contains("Creature"), "{tap}");
    }
}

#[test]
fn hog_monkey_rampage_checks_the_creature_you_control() {
    for definition in support::definitions("Hog-Monkey Rampage") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Hog-Monkey Rampage", &text);
        let debug = format!("{:?}", definition.spell_effect);
        let condition = debug
            .split("ConditionalEffect")
            .nth(1)
            .expect("conditional counter");
        // The "it" of the postcondition is narrowed to the controller's
        // member of the chosen pair.
        assert!(condition.contains("controller: Some(You)"), "{condition}");
    }
}

#[test]
fn stolen_uniform_watches_the_equipment_for_lost_control() {
    for definition in support::definitions("Stolen Uniform") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Stolen Uniform", &text);
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
        assert!(debug.contains("ControlChanged"), "{debug}");
        assert!(debug.contains("attached_to_object: Some"), "{debug}");
    }
}
