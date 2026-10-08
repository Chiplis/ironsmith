//! Discard qualifiers: chosen creature type and a trailing relation that keeps
//! the leading card qualifier (p12-other). Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

#[test]
fn tsabos_decree_discards_creature_cards_of_the_chosen_type() {
    for definition in support::definitions("Tsabo's Decree") {
        let debug = support::debug(&definition);
        assert!(debug.contains("chosen_creature_type: true"), "{debug}");
        assert!(debug.contains("Creature"));
    }
}

#[test]
fn void_discard_keeps_nonland_with_the_chosen_number() {
    for definition in support::definitions("Void") {
        let discard = support::effects(&definition)
            .into_iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::DiscardEffect>().cloned())
            .expect("discard");
        let debug = format!("{discard:?}");
        assert!(debug.contains("excluded_card_types: [Land]"), "nonland survives: {debug}");
        assert!(debug.contains("ChosenNumber"), "{debug}");
    }
}
