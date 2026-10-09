//! Regressions for mechanisms interrupted in the Claude handoff.
#[path = "cf8_p08/support.rs"]
mod support;

#[test]
fn copied_abilities_retain_source_qualities() {
    use ironsmith::ability::AbilityKind;
    for (name, body, required) in [
        ("Abstruse Archaic", "Mana cost: {4}\nType: Creature — Avatar\nPower/Toughness: 3/4\nVigilance\n{1}, {T}: Copy target activated or triggered ability you control from a colorless source. You may choose new targets for the copy. (Mana abilities can't be targeted.)", vec!["colorless: true"]),
        ("The Peregrine Dynamo", "Mana cost: {3}\nType: Legendary Artifact Creature — Construct\nPower/Toughness: 1/5\nHaste\n{1}, {T}: Copy target activated or triggered ability you control from another legendary source that's not a commander. You may choose new targets for the copy. (Mana abilities can't be targeted.)", vec!["Legendary", "noncommander: true", "other: true"]),
    ] {
        for definition in support::definitions(name, body) {
            let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated), _ => None,
            }).unwrap();
            let debug = format!("{activated:?}");
            for qualifier in &required { assert!(debug.contains(qualifier), "{name}: missing {qualifier}: {debug}"); }
        }
    }
}

#[test]
fn delayed_death_checks_the_referenced_creatures_last_controller() {
    for definition in support::definitions("Desperate Measures", "Mana cost: {B}\nType: Instant\nTarget creature gets +1/-1 until end of turn. When it dies under your control this turn, draw two cards.") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("DelayedTrigger"), "{debug}");
        assert!(debug.contains("controller: Some(You)"), "{debug}");
        assert!(debug.contains("DrawCards"), "{debug}");
    }
}
