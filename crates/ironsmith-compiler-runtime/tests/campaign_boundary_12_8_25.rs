//! Current-main source-regeneration gate. Authored, every test UNRUN.
use ironsmith_compiled_artifact::{
    ArtifactValidationError, CompiledCardArtifact, ENGINE_SCHEMA_HASH, FORMAT_VERSION,
};
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::{materialize_artifact, materialize_definition};

const PREVIOUS_SCHEMA: &str = "cf9f06e2cea9c4facdfe9b4aad19eaa4bca1062e4e9c9f28d22de18920cbd401";
const UNPUBLISHED_SCHEMA: &str = "9d0e162e131ecfbaf850e2331cd33a54ea932fb48eecd55978e271a26bc3938c";
const SUPERSEDED_UNPUBLISHED_SCHEMA: &str = "2b6fde5114ec4007309dcdb183ed453ec30033f59d1600319640ba45877a4c9f";

fn compile_source(name: &str, text: &str) -> CompiledCardArtifact {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    assert_eq!(FORMAT_VERSION, 18);
    assert_eq!(artifact.format_version, FORMAT_VERSION);
    assert_eq!(artifact.engine_schema_hash, ENGINE_SCHEMA_HASH);
    artifact.validate().unwrap();
    artifact
}

fn compile_frozen(fixture: &str, name: &str) -> CompiledCardArtifact {
    let rows: Vec<serde_json::Value> = serde_json::from_str(fixture).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    if let Some(text) = row["text"].as_str() { return compile_source(name, text); }
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    compile_source(name, &text)
}

#[test]
fn regenerated_full_bodies_admit_current_models_and_refuse_previous_envelopes() {
    for (fixture, name) in [
        (include_str!("../../../fixtures/temporary_prevention_bindings.json.fixture"), "Decorated Griffin"),
        (include_str!("../../../fixtures/temporary_prevention_bindings.json.fixture"), "Avacyn, Guardian Angel"),
        (include_str!("../../../fixtures/copular_characteristic_statics.json.fixture"), "Stonework Packbeast"),
        (include_str!("../../../fixtures/battlefield_destination_references.json.fixture"), "Trove Warden"),
        (include_str!("../../../fixtures/remove_any_source_counter_payloads.json.fixture"), "Arcbound Javelineer"),
        (include_str!("../../../fixtures/static_prevention_regressions.json.fixture"), "Guardian Seraph"),
        (include_str!("../../../fixtures/static_prevention_regressions.json.fixture"), "Sphere of Law"),
        (include_str!("../../../fixtures/static_prevention_regressions.json.fixture"), "Defang"),
        (include_str!("../../../fixtures/static_prevention_regressions.json.fixture"), "Candletrap"),
    ] {
        let artifact = compile_frozen(fixture, name);
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(restored, artifact);
        materialize_artifact(&restored).unwrap();

        let mut old = artifact.clone();
        old.format_version = 11;
        old.refresh_checksum();
        assert!(matches!(old.validate(),
            Err(ArtifactValidationError::UnsupportedFormat { found: 11, expected: 17 })));
        assert!(CompiledCardArtifact::from_json(&old.to_json().unwrap()).is_err());
        assert!(materialize_artifact(&old).is_err());

        for schema in [PREVIOUS_SCHEMA, UNPUBLISHED_SCHEMA, SUPERSEDED_UNPUBLISHED_SCHEMA] {
            let mut relabeled = artifact.clone();
            relabeled.engine_schema_hash = schema.into();
            relabeled.refresh_checksum();
            assert!(matches!(relabeled.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
            assert!(CompiledCardArtifact::from_json(&relabeled.to_json().unwrap()).is_err());
            assert!(materialize_artifact(&relabeled).is_err());
        }
    }
}

#[test]
fn regenerated_aura_rules_keep_the_live_source_relation_on_the_battlefield_owner() {
    use ironsmith_core::{AbilityKind, ObjectFilter, PlayerFilter, StaticAbilityPayload,
        StaticDamagePreventionAmount, TaggedObjectConstraint, TaggedOpbjectRelation, Zone};
    for name in ["Candletrap", "Demonic Torment", "Defang", "Muzzle", "Temporal Isolation"] {
        let artifact = compile_frozen(include_str!("../../../fixtures/static_prevention_regressions.json.fixture"), name);
        let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        let rules: Vec<_> = decoded.payload.definition.abilities.iter().filter_map(|ability| {
            let AbilityKind::Static(model) = &ability.kind else { return None };
            let StaticAbilityPayload::PreventMatchingDamage(spec) = &model.payload else { return None };
            Some((ability, spec))
        }).collect();
        assert_eq!(rules.len(), 1, "{name}: prevention belongs directly to the Aura");
        let (ability, spec) = rules[0];
        assert_eq!(ability.functional_zones, vec![Zone::Battlefield]);
        assert_eq!(spec.source_filter.tagged_constraints, vec![TaggedObjectConstraint {
            tag: "enchanted".into(), relation: TaggedOpbjectRelation::IsTaggedObject,
        }]);
        assert!(spec.source_filter.with_attached_object.is_none());
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::Any));
        assert_eq!(spec.target_object_filter, Some(ObjectFilter::permanent()));
        assert_eq!(spec.combat_only, matches!(name, "Candletrap" | "Demonic Torment"));
        assert!(!spec.noncombat_only);
        assert_eq!(spec.maximum_damage, None);
        assert_eq!(spec.amount, StaticDamagePreventionAmount::All);
        let native = materialize_artifact(&decoded).unwrap();
        let encoded = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(native).unwrap();
        let rule = encoded.abilities.iter().find(|ability| matches!(&ability.kind,
            AbilityKind::Static(model) if matches!(&model.payload, StaticAbilityPayload::PreventMatchingDamage(_))))
            .expect("native encoding must retain the Aura-owned prevention rule");
        assert_eq!(serde_json::to_value(rule).unwrap(), serde_json::to_value(ability).unwrap());
    }
}

#[test]
fn subtype_zone_construction_requires_regeneration_and_is_not_repaired_on_load() {
    let artifact = compile_frozen(include_str!("../../../fixtures/copular_characteristic_statics.json.fixture"),
        "Stonework Packbeast");
    let index = artifact.payload.definition.abilities.iter().position(|ability| {
        matches!(&ability.kind, ironsmith_core::AbilityKind::Static(model)
            if model.characteristic_defining_subtypes().is_some())
    }).expect("complete source contains the source-only subtype CDA");
    let zones = &artifact.payload.definition.abilities[index].functional_zones;
    for zone in [ironsmith_core::Zone::Hand, ironsmith_core::Zone::Stack,
        ironsmith_core::Zone::Graveyard, ironsmith_core::Zone::Exile] {
        assert!(zones.contains(&zone));
    }
    let mut historical_shape = artifact.payload.definition.clone();
    historical_shape.abilities[index].functional_zones = vec![ironsmith_core::Zone::Battlefield];
    let bytes = serde_json::to_vec(&historical_shape).unwrap();
    let restored = materialize_definition(serde_json::from_slice(&bytes).unwrap()).unwrap();
    assert_eq!(restored.abilities[index].functional_zones, vec![ironsmith_core::Zone::Battlefield],
        "loading preserves explicit data; only source recompilation reconstructs the missing zones");
}

#[test]
fn source_regeneration_selects_explicit_creature_family_without_recipient_inference() {
    use ironsmith_core::{AbilityKind, StaticAbilityPayload, Zone};
    for (name, text, creature_family) in [
        ("Independent creature choice", "Type: Artifact\nAs this artifact enters, choose a creature type.\nLands and creatures you control have the chosen creature type in addition to their other types.", true),
        ("Independent land choice", "Type: Artifact\nAs this artifact enters, choose a basic land type.\nLands you control are the chosen type in addition to their other types.", false),
    ] {
        let artifact = compile_source(name, text);
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(restored, artifact);
        let choice_ability = restored.payload.definition.abilities.iter().find(|ability| {
            matches!(&ability.kind, AbilityKind::Static(model)
                if matches!(&model.payload, StaticAbilityPayload::AddChosenCreatureType { .. }
                    | StaticAbilityPayload::AddChosenBasicLandType { .. }))
        }).expect("the complete chosen-type assertion survives source compilation");
        let AbilityKind::Static(model) = &choice_ability.kind else { unreachable!() };
        assert_eq!(matches!(&model.payload, StaticAbilityPayload::AddChosenCreatureType { .. }), creature_family);
        assert_eq!(choice_ability.functional_zones, vec![Zone::Battlefield]);
        materialize_artifact(&restored).unwrap();
    }
}
