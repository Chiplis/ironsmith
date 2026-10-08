//! "Return it to the battlefield transformed under your control attached to
//! <target>": the transformed entry (CR 712.14) composes with the attach
//! (CR 303.4f, CR 701.3). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::effects::{AttachObjectsEffect, MoveToZoneEffect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const CARDS: [(&str, &str); 3] = [
    ("Accursed Witch", "Mana cost: {3}{B}\nType: Creature — Human Shaman\nPower/Toughness: 4/2\nSpells your opponents cast that target this creature cost {1} less to cast.\nWhen this creature dies, return it to the battlefield transformed under your control attached to target opponent."),
    ("Radiant Grace", "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+0 and has vigilance.\nWhen enchanted creature dies, return this card to the battlefield transformed under your control attached to target opponent."),
    ("Vengeful Strangler", "Mana cost: {1}{B}\nType: Creature — Spirit\nPower/Toughness: 3/4\nThis creature can't block.\nWhen this creature dies, return it to the battlefield transformed under your control attached to target creature or planeswalker an opponent controls."),
];

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

fn all_effects(definition: &CardDefinition) -> Vec<ironsmith::effect::Effect> {
    fn collect(effect: &ironsmith::effect::Effect, out: &mut Vec<ironsmith::effect::Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| collect(child, out));
    }
    let mut all = Vec::new();
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() {
                collect(effect, &mut all);
            }
        }
    }
    all
}

#[test]
fn dies_trigger_returns_transformed_then_attaches_to_the_target() {
    for (name, text) in CARDS {
        for definition in definitions(name, text) {
            let effects = all_effects(&definition);
            let moved = effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<MoveToZoneEffect>())
                .unwrap_or_else(|| panic!("{name}: return"));
            assert_eq!(moved.zone, ironsmith::Zone::Battlefield, "{name}");
            assert!(moved.enters_transformed, "{name}: enters transformed");
            let attach = effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<AttachObjectsEffect>())
                .unwrap_or_else(|| panic!("{name}: attach"));
            let target = format!("{:?}", attach.target);
            if name == "Vengeful Strangler" {
                assert!(target.contains("Planeswalker") && target.contains("Opponent"), "{target}");
            } else {
                assert!(target.contains("Opponent"), "{name}: {target}");
            }
        }
    }
}
