//! "As this creature enters, mill three cards for each time it was kicked."
//! (Urborg Lhurgoyf): a fixed mill batch multiplied by the kick count.
//! Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const URBORG_LHURGOYF: &str = "Mana cost: {1}{G}\nType: Creature — Lhurgoyf\nPower/Toughness: */1+*\nKicker {U} and/or {B} (You may pay an additional {U} and/or {B} as you cast this spell.)\nAs this creature enters, mill three cards for each time it was kicked.\nUrborg Lhurgoyf's power is equal to the number of creature cards in your graveyard and its toughness is equal to that number plus 1.";

#[test]
fn urborg_lhurgoyf_mills_three_per_kick_as_it_enters() {
    for definition in compile::compile_both("Urborg Lhurgoyf", URBORG_LHURGOYF) {
        let program = definition
            .abilities
            .iter()
            .find_map(|ability| {
                let AbilityKind::Static(static_ability) = &ability.kind else {
                    return None;
                };
                let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram { program, .. } =
                    &static_ability.compiled_model()?.payload
                else {
                    return None;
                };
                Some(format!("{program:?}"))
            })
            .expect("as-enters mill program");
        assert!(program.contains("Mill"), "{program}");
        assert!(program.contains("Scaled(KickCount, 3)"), "{program}");
    }
}
