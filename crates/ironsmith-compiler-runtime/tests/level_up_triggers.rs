//! cf8 p01: triggered abilities printed inside a level-up range exist only
//! while the level-counter range holds (CR 711.2a); the gate is event-time
//! (ConditionQualified), not an intervening "if". Source-authored, unrun.
use ironsmith::ability::AbilityKind;

#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn level_range_triggers_are_event_time_gated() {
    for (name, phrase) in [
        ("Lighthouse Chronologist", "take an extra turn after this one"),
        ("Lord of Shatterskull Pass", "deals 6 damage to each creature defending player controls"),
    ] {
        for definition in support::definitions(name) {
            let gated = definition
                .abilities
                .iter()
                .filter(|ability| match &ability.kind {
                    AbilityKind::Triggered(triggered) => triggered
                        .trigger
                        .downcast_ref::<ironsmith::triggers::ConditionQualifiedTrigger>()
                        .is_some_and(|qualified| {
                            qualified.surface.starts_with("__ironsmith_level_range:")
                        }),
                    _ => false,
                })
                .count();
            assert_eq!(gated, 1, "{name}");
            let text = support::rendered(&definition);
            assert!(text.contains(phrase), "{name}: {text}");
            assert!(!text.contains("__ironsmith_level_range"), "{name}: {text}");
        }
    }
}
