//! Explicit token-entry and attachment differences, including artifact transport.
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::object::AttachmentTarget;
use ironsmith::types::Subtype;
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact(name, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}

#[test]
fn adjudicate_never_enters_against_enchantment_protection() {
    let text = "Type: Sorcery\nCreate a white Aura enchantment token with \"Enchant creature\" and \"Sacrifice this token: Untap enchanted creature.\" attached to target creature.";
    for definition in definitions("Aura action", text) {
        for protection in [false, true] {
            let body = if protection { "Type: Creature\nPower/Toughness: 2/2\nProtection from enchantments" }
                else { "Type: Creature\nPower/Toughness: 2/2" };
            let host = ironsmith_compiler_runtime::compile_to_runtime_definition("Host", body, false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let host = game.create_object_from_definition(&host, A, Zone::Battlefield);
            let source = game.create_object_from_definition(&definition, A, Zone::Stack);
            let mut context = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(host)]);
            for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
                execute_effect(&mut game, effect, &mut context).unwrap();
            }
            let auras = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| object.subtypes.contains(&Subtype::Aura)).count();
            assert_eq!(auras, usize::from(!protection));
        }
    }
}

#[test]
fn multi_target_arm_keeps_equipment_when_protection_prevents_only_attachment() {
    let text = "Type: Sorcery\nFor each of up to two target creatures you control, create a colorless Equipment artifact token with \"Equipped creature gets +1/+0.\" and equip {2} attached to that creature.";
    for definition in definitions("Equipment action", text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let ordinary = ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
        let protected = ironsmith_compiler_runtime::compile_to_runtime_definition("Protected", "Type: Creature\nPower/Toughness: 2/2\nProtection from artifacts", false).unwrap();
        let ordinary = game.create_object_from_definition(&ordinary, A, Zone::Battlefield);
        let protected = game.create_object_from_definition(&protected, A, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut context = EffectContext::new_default(source, A).with_targets(vec![ResolvedTarget::Object(ordinary), ResolvedTarget::Object(protected)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut context).unwrap();
        }
        let equipment: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| object.subtypes.contains(&Subtype::Equipment)).collect();
        assert_eq!(equipment.len(), 2);
        assert_eq!(equipment.iter().filter(|object| object.attached_to == Some(AttachmentTarget::Object(ordinary))).count(), 1);
        assert_eq!(equipment.iter().filter(|object| object.attached_to.is_none()).count(), 1);
        assert!(!equipment.iter().any(|object| object.attached_to == Some(AttachmentTarget::Object(protected))));
    }
}
