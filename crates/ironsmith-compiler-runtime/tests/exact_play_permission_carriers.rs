//! Typed carrier scenarios only. No frozen card credit; all scenarios unrun.
use ironsmith_core::{AbilityKind, StaticAbilityPayload};
use ironsmith_core::value_model::ManaSpendMode;
use ironsmith_compiler_runtime::{compile_to_artifact, into_runtime_definition};

#[test]
fn default_shapes_and_explicit_modes_survive_direct_and_artifact_materializers() {
    let source = "Type: Enchantment\nYou may play lands from your graveyard.";
    let (baseline, _) = compile_to_artifact("Carrier probe", source, false).unwrap();
    let old = baseline.to_json().unwrap();
    assert!(!std::str::from_utf8(&old).unwrap().contains("cast_mana_spend_mode"));
    ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&old).unwrap().validate().unwrap();
    for mode in [ManaSpendMode::AnyColor, ManaSpendMode::AnyType] {
        let mut compiled = ironsmith_compiler::CompilerFacade::new().compile_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Carrier probe"),
            source.into(), ironsmith_compiler::CompilePolicy { allow_unsupported: false }).unwrap().definition;
        let mut found = false;
        for ability in &mut compiled.abilities {
            if let AbilityKind::Static(ability) = &mut ability.kind
                && let StaticAbilityPayload::Grants(spec) = &mut ability.payload {
                spec.cast_mana_spend_mode = mode; found = true;
            }
        }
        assert!(found, "the grammar must construct the actual typed grant carrier");
        let mut artifact = baseline.clone();
        artifact.payload.definition = ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled).unwrap();
        artifact.refresh_checksum();
        artifact.validate().unwrap();
        let decoded = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(artifact, decoded);
        let direct = into_runtime_definition(compiled).unwrap();
        let loaded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
        for definition in [direct, loaded] {
            let retained = definition.abilities.iter().find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Static(ability) => ability.grant_spec(), _ => None,
            }).unwrap();
            assert_eq!(retained.cast_mana_spend_mode, mode);
        }
    }
}

#[test]
fn tagged_effect_omits_legacy_false_and_retains_explicit_selected_permission_mode() {
    let legacy = ironsmith_core::GrantPlayTaggedEffect::<ironsmith_compiled_artifact::WireCost>::new(
        "exact".into(), ironsmith_core::PlayerFilter::You, ironsmith_core::GrantPlayTaggedDuration::ForAsLongAsExiled,
        true, ManaSpendMode::AnyColor);
    let old = serde_json::to_value(&legacy).unwrap();
    assert!(old.get("permission_bound_mana").is_none());
    let decoded: ironsmith_core::GrantPlayTaggedEffect<ironsmith_compiled_artifact::WireCost> = serde_json::from_value(old).unwrap();
    assert!(!decoded.permission_bound_mana);
    let mut marked = legacy;
    marked.permission_bound_mana = true;
    let encoded = serde_json::to_value(&marked).unwrap();
    assert_eq!(encoded["permission_bound_mana"], true);
    let decoded: ironsmith_core::GrantPlayTaggedEffect<ironsmith_compiled_artifact::WireCost> = serde_json::from_value(encoded).unwrap();
    assert!(decoded.permission_bound_mana);
    assert_eq!(decoded.mana_spend_mode, ManaSpendMode::AnyColor);
}
