//! Typed "Cast this spell only ..." windows (CR 506-511) and the Rapid Fire
//! body ("If it doesn't have rampage, that creature gains rampage 2").
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const RAPID_FIRE: &str = "Mana cost: {3}{W}\nType: Instant\nCast this spell only before blockers are declared.\nTarget creature gains first strike until end of turn. If it doesn't have rampage, that creature gains rampage 2 until end of turn. (Whenever the creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)";

#[test]
fn rapid_fire_compiles_with_its_window_and_conditional_rampage() {
    for definition in compile::compile_both("Rapid Fire", RAPID_FIRE) {
        let timing = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .find(|debug| debug.contains("BeforeBlockersAreDeclared"));
        assert!(timing.is_some(), "typed timing window");
        let effects = format!("{:?}", definition.spell_effect);
        assert!(effects.contains("ConditionalEffect"), "{effects}");
        assert!(effects.to_ascii_lowercase().contains("rampage"), "{effects}");
        assert!(effects.contains("FirstStrike"), "{effects}");
    }
}
