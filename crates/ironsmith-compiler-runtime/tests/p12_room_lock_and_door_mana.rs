//! "Lock or unlock a door of target Room you control" (CR 709.5c) and
//! "Spend this mana only to cast Room spells and unlock doors" (CR 709.5e).
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::effects::UnlockRoomDoorEffect;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const KEYS: &str = "Mana cost: {1}\nType: Artifact\n{1}, {T}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\n{3}, {T}, Sacrifice this artifact: Lock or unlock a door of target Room you control. Activate only as a sorcery.";
const MARINA: &str = "Mana cost: {G}{U}\nType: Legendary Creature — Elf Druid\nPower/Toughness: 1/2\nWhen Marina Vendrell enters, reveal the top seven cards of your library. Put all enchantment cards from among them into your hand and the rest on the bottom of your library in a random order.\n{T}: Lock or unlock a door of target Room you control. Activate only as a sorcery.";
const SMOKY_LOUNGE: &str = "Mana cost: {2}{R}\nType: Enchantment — Room\nAt the beginning of your first main phase, add {R}{R}. Spend this mana only to cast Room spells and unlock doors.";

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

fn lock_or_unlock(definition: &CardDefinition) -> UnlockRoomDoorEffect {
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            for effect in activated.effects.all_effects() {
                if let Some(effect) = effect.downcast_ref::<UnlockRoomDoorEffect>() {
                    return effect.clone();
                }
            }
        }
    }
    panic!("lock-or-unlock effect")
}

#[test]
fn keys_and_marina_toggle_a_door_of_the_targeted_room() {
    for (name, text) in [("Keys to the House", KEYS), ("Marina Vendrell", MARINA)] {
        for definition in definitions(name, text) {
            let effect = lock_or_unlock(&definition);
            assert!(effect.allow_lock, "{name}: lock offered");
            assert!(effect.room_filter.is_target_object, "{name}: bound to the target");
        }
    }
}

#[test]
fn smoky_lounge_mana_pays_room_spells_and_unlock_costs_only() {
    for definition in definitions("Smoky Lounge", SMOKY_LOUNGE) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CastSpellOrUnlockDoor {"), "{debug}");
        assert!(!debug.contains("CastSpellOrUnlockDoorOrTurnFaceUp"), "{debug}");
        assert!(debug.contains("Room"), "{debug}");
    }
}
