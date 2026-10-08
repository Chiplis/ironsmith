//! "As long as this creature is enchanted by exactly two Auras, it has base
//! power and toughness 5/5 and vigilance." (Timber Paladin): the shared "has"
//! governs both the layer-7b base P/T and the keyword list, under one
//! condition. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const TIMBER_PALADIN: &str = "Mana cost: {1}{G}\nType: Artifact Creature — Knight\nPower/Toughness: 1/1\nAs long as this creature is enchanted by exactly one Aura, it has base power and toughness 3/3.\nAs long as this creature is enchanted by exactly two Auras, it has base power and toughness 5/5 and vigilance.\nAs long as this creature is enchanted by three or more Auras, it has base power and toughness 10/10, vigilance, and trample.";

#[test]
fn timber_paladin_tiers_compile_with_their_keywords() {
    for definition in compile::compile_both("Timber Paladin", TIMBER_PALADIN) {
        let all = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        for pt in ["power: 3, toughness: 3", "power: 5, toughness: 5", "power: 10, toughness: 10"] {
            assert!(all.contains(pt), "{pt}: {all}");
        }
        assert!(all.matches("Vigilance").count() >= 2, "{all}");
        assert!(all.contains("Trample"), "{all}");
    }
}
