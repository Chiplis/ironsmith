//! "Tap all creatures target player controls. Those creatures don't untap
//! during that player's next untap step." (Sleep): "that player" is the
//! declared target player; the tagged set carries no controller of its own.
//! Source-authored and UNRUN.
#[path = "cf8_p08/support.rs"]
mod support;

const SLEEP: &str = "Mana cost: {2}{U}{U}\nType: Sorcery\nTap all creatures target player controls. Those creatures don't untap during that player's next untap step.";

#[test]
fn the_named_step_belongs_to_the_target_player() {
    for definition in support::definitions("Sleep", SLEEP) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("PlayersNextUntapStep"), "{debug}");
        assert!(!debug.contains("IteratedPlayer"), "{debug}");
        let text = support::rendered(&definition);
        assert!(text.contains("next untap step"), "{text}");
    }
}
