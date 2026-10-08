//! "As this creature enters, put a phylactery counter on an artifact you
//! control." (Phylactery Lich): a non-targeted selection headed by a singular
//! card-type noun (after the article) is a bare selection like "a creature
//! you control". Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const PHYLACTERY_LICH: &str = "Mana cost: {B}{B}{B}\nType: Creature — Zombie\nPower/Toughness: 5/5\nIndestructible\nAs this creature enters, put a phylactery counter on an artifact you control.\nWhen you control no permanents with phylactery counters on them, sacrifice this creature.";

#[test]
fn phylactery_lich_marks_a_chosen_artifact_as_it_enters() {
    for definition in compile::compile_both("Phylactery Lich", PHYLACTERY_LICH) {
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
            .expect("as-enters counter program");
        assert!(program.contains("PutCountersEffect"), "{program}");
        assert!(program.contains("Phylactery"), "{program}");
        assert!(program.contains("Artifact"), "{program}");
        assert!(!program.contains("Target("), "not a targeted choice: {program}");
    }
}
