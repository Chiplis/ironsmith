//! "Unlock a locked door of up to one target Room you control" (CR 709.5f):
//! the unlock is restricted to the announced target. Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::effects::{TargetOnlyEffect, UnlockRoomDoorEffect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const GHOSTLY_KEYBEARER: &str = "Mana cost: {3}{U}\nType: Creature — Spirit\nPower/Toughness: 3/3\nFlying\nWhenever this creature deals combat damage to a player, unlock a locked door of up to one target Room you control.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

#[test]
fn keybearer_unlocks_only_the_targeted_room() {
    for definition in definitions("Ghostly Keybearer", GHOSTLY_KEYBEARER) {
        let mut target_only = None;
        let mut unlock = None;
        for ability in &definition.abilities {
            let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
                continue;
            };
            for effect in triggered.effects.all_effects() {
                if let Some(effect) = effect.downcast_ref::<TargetOnlyEffect>() {
                    target_only = Some(format!("{effect:?}"));
                }
                if let Some(effect) = effect.downcast_ref::<UnlockRoomDoorEffect>() {
                    unlock = Some(effect.clone());
                }
            }
        }
        let target_only = target_only.expect("announced Room target");
        assert!(target_only.contains("Room"), "{target_only}");
        let unlock = unlock.expect("unlock effect");
        assert!(unlock.room_filter.is_target_object, "unlock bound to the target");
    }
}
