//! cf8 p01 round 5: a value that reads the card revealed at random from a
//! player's hand (Singe-Mind Ogre). Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn singe_mind_ogre_loses_life_equal_to_the_random_card() {
    for definition in support::definitions("Singe-Mind Ogre") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("random: true"), "{debug}");
        assert!(debug.contains("ManaValueOf"), "{debug}");
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Singe-Mind Ogre", &text);
        assert!(text.contains("at random from their hand"), "{text}");
        assert!(!text.contains("that spell's mana value"), "{text}");
    }
}
