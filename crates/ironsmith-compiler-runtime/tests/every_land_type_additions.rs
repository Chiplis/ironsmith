//! "Each land with an everything counter on it is every land type in addition
//! to its other types." (Omo, Queen of Vesuva): CR 205.3i/305.7 additive
//! land-type family grant with the explicit "in addition" tail.
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const OMO: &str = "Mana cost: {2}{G/U}\nType: Legendary Creature — Shapeshifter Noble\nPower/Toughness: 1/5\nWhenever Omo enters or attacks, put an everything counter on each of up to one target land and up to one target creature.\nEach land with an everything counter on it is every land type in addition to its other types.\nEach nonland creature with an everything counter on it is every creature type.";

#[test]
fn omo_adds_every_land_type_and_every_creature_type() {
    for definition in compile::compile_both("Omo, Queen of Vesuva", OMO) {
        let statics: Vec<String> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .collect();
        assert!(statics.iter().any(|debug| debug.contains("Land") && debug.to_ascii_lowercase().contains("everything")), "{statics:?}");
        assert!(statics.iter().any(|debug| debug.contains("Creature") && debug.to_ascii_lowercase().contains("everything")), "{statics:?}");
        assert_eq!(statics.len(), 2, "{statics:?}");
    }
}
