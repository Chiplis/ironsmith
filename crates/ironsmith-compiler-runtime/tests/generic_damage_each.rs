//! UNVALIDATED: affected exact-card producers use the completed damage owner.
use ironsmith::ability::AbilityKind;
use ironsmith::effect::Effect;
use ironsmith::effects::{DealDamageEachEffect, DealDamageEffect, ForEachObject};
use ironsmith::target::ChooseSpec;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
fn damage_owner(effect: &Effect) -> bool {
    effect.downcast_ref::<DealDamageEffect>().is_some()
        || effect.downcast_ref::<DealDamageEachEffect>().is_some()
}
fn has_damage(effect: &Effect) -> bool {
    let mut found = damage_owner(effect);
    effect.visit_child_effects(&mut |child| found |= has_damage(child));
    found
}
fn check(effect: &Effect, owners: &mut usize) {
    if let Some(each) = effect.downcast_ref::<ForEachObject>() {
        assert!(
            !each.effects.iter().any(has_damage),
            "quantified damage must not remain in a serial object loop: {effect:?}"
        );
    }
    if effect.downcast_ref::<DealDamageEachEffect>().is_some() {
        *owners += 1;
    }
    if let Some(damage) = effect.downcast_ref::<DealDamageEffect>()
        && matches!(damage.target.base(), ChooseSpec::All(_))
    {
        *owners += 1;
    }
    effect.visit_child_effects(&mut |child| check(child, owners));
}
#[test]
fn exact_goblin_shrine_and_choking_vines_have_one_completed_batch_owner() {
    let fixtures = [
        include_str!("../../../fixtures/live_static_condition_scopes.json.fixture"),
        include_str!("../../../fixtures/combat_blocked_status.json.fixture"),
    ];
    for (name, text) in ["Goblin Shrine", "Choking Vines"].into_iter().zip(fixtures) {
        let rows: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        let input = format!(
            "Mana cost: {}\nType: {}\n{}",
            row["mana_cost"].as_str().unwrap(),
            row["type_line"].as_str().unwrap(),
            row["oracle_text"].as_str().unwrap()
        );
        let (result, loss) =
            ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, input, false));
        let (artifact, direct) = result.unwrap();
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        for definition in [
            direct,
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded)
                .unwrap(),
        ] {
            let mut owners = 0;
            if let Some(program) = &definition.spell_effect {
                for segment in &program.segments {
                    for effect in &segment.default_effects {
                        check(effect, &mut owners);
                    }
                }
            }
            for ability in &definition.abilities {
                if let AbilityKind::Triggered(trigger) = &ability.kind {
                    for segment in &trigger.effects.segments {
                        for effect in &segment.default_effects {
                            check(effect, &mut owners);
                        }
                    }
                }
            }
            assert_eq!(
                owners, 1,
                "{name}: actual complete body has one damage-set owner"
            );
        }
    }
}
#[test]
fn generic_damage_each_ast_lowers_directly_to_its_typed_payload() {
    let (artifact, direct) = compile_to_artifact(
        "Generic damage",
        "Mana cost: {R}\nType: Sorcery\nDeal 1 damage to each creature.",
        false,
    )
    .unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    for definition in [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ] {
        let mut owners = 0;
        for segment in &definition.spell_effect.as_ref().unwrap().segments {
            for effect in &segment.default_effects {
                check(effect, &mut owners);
            }
        }
        assert_eq!(owners, 1);
        assert!(format!("{:?}", definition.spell_effect).contains("DealDamageEachEffect"));
    }
}
