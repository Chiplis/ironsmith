//! cf8 p05: "Its controller manifests dread." (CR 701.62): the controller of
//! the referenced object, not the source's controller, manifests dread.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;
use ironsmith::effects::ManifestDreadEffect;

fn collect(effect: &ironsmith::effect::Effect, out: &mut Vec<ironsmith::effect::Effect>) {
    out.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, out));
}

#[test]
fn referenced_objects_controller_manifests_dread() {
    let rows = support::rows("its_controller_manifest_dread");
    assert_eq!(rows.len(), 2);
    for row in &rows {
        for definition in support::definitions(row) {
            let mut all = Vec::new();
            if let Some(program) = definition.spell_effect.as_ref() {
                for effect in program.all_effects() {
                    collect(effect, &mut all);
                }
            }
            for ability in &definition.abilities {
                if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
                    for effect in triggered.effects.all_effects() {
                        collect(effect, &mut all);
                    }
                }
            }
            let manifests: Vec<_> = all
                .iter()
                .filter_map(|effect| effect.downcast_ref::<ManifestDreadEffect>())
                .collect();
            assert_eq!(manifests.len(), 1, "{}", row["name"]);
            let player = format!("{:?}", manifests[0]);
            assert!(player.contains("ControllerOf"), "{}: {player}", row["name"]);
        }
    }
}
