//! Compiler/cache release admission, authored only: UNVALIDATED / UNRUN.
//! Synthetic envelope mutations are not recovered historical artifacts.
use ironsmith_compiled_artifact::{ArtifactValidationError, CompiledCardArtifact,
    ENGINE_SCHEMA_HASH, FORMAT_VERSION};
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::{ArtifactRegistrationError, CardRegistryArtifactExt};
use ironsmith_runtime_catalog::artifact_materializer::{ArtifactMaterializationError,
    encode_runtime_definition, materialize_artifact, materialize_definition};

const PUBLISHED_13_SCHEMA: &str =
    "cbaf3a819cee97d5351ddc85c7789a85508caace7573a7f8aa3fe7af0b87854c";

fn compile_source(name: &str, text: &str) -> CompiledCardArtifact {
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());
    let (compiled, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!artifact_loss.is_lossy(), "artifact {name}: {}", artifact_loss.reasons_text());
    assert_eq!(FORMAT_VERSION, 14);
    assert_eq!(artifact.format_version, 14);
    assert_eq!(artifact.engine_schema_hash, ENGINE_SCHEMA_HASH);
    artifact.validate().unwrap();
    let bytes = artifact.to_json().unwrap();
    let decoded = CompiledCardArtifact::from_json(&bytes).unwrap();
    assert_eq!(decoded, artifact);
    assert_eq!(decoded.to_json().unwrap(), bytes);
    materialize_artifact(&decoded).unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&decoded).unwrap();
    assert!(registry.get(name).is_some());
    // This model is freshly and independently compiled above. It is never
    // extracted from a failed envelope to bypass the release admission gate.
    let wire = encode_runtime_definition(direct).unwrap();
    materialize_definition(serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
    artifact
}

fn source(row: &serde_json::Value) -> String {
    if let Some(text) = row["text"].as_str() { return text.into(); }
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    text
}

fn assert_old_envelopes_refused(artifact: &CompiledCardArtifact) {
    for version in [11, 12, 13] {
        let mut stale = artifact.clone();
        stale.format_version = version;
        stale.refresh_checksum();
        assert!(matches!(stale.validate(), Err(ArtifactValidationError::UnsupportedFormat {
            found, expected: 14,
        }) if found == version));
        assert!(CompiledCardArtifact::from_json(&stale.to_json().unwrap()).is_err());
        assert!(matches!(materialize_artifact(&stale),
            Err(ArtifactMaterializationError::InvalidArtifact(
                ArtifactValidationError::UnsupportedFormat { found, expected: 14 }
            )) if found == version));
        let mut registry = ironsmith::cards::CardRegistry::new();
        assert!(matches!(registry.register_compiled_artifact(&stale),
            Err(ArtifactRegistrationError::Invalid(
                ArtifactValidationError::UnsupportedFormat { found, expected: 14 }
            )) if found == version));
        assert!(registry.get(&artifact.card.name).is_none());
    }
    for schema in [PUBLISHED_13_SCHEMA,
        "fbc604c03fa9eed8de6567576052324b9c8bee8d9ddc319f2ae25bc0a62b9c5d",
        "cf9f06e2cea9c4facdfe9b4aad19eaa4bca1062e4e9c9f28d22de18920cbd401",
        "9d0e162e131ecfbaf850e2331cd33a54ea932fb48eecd55978e271a26bc3938c",
        "2b6fde5114ec4007309dcdb183ed453ec30033f59d1600319640ba45877a4c9f"] {
        let mut stale = artifact.clone();
        stale.engine_schema_hash = schema.into();
        stale.refresh_checksum();
        assert!(matches!(stale.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(CompiledCardArtifact::from_json(&stale.to_json().unwrap()).is_err());
        assert!(matches!(materialize_artifact(&stale),
            Err(ArtifactMaterializationError::InvalidArtifact(
                ArtifactValidationError::EngineSchemaMismatch { .. }))));
        let mut registry = ironsmith::cards::CardRegistry::new();
        assert!(matches!(registry.register_compiled_artifact(&stale),
            Err(ArtifactRegistrationError::Invalid(ArtifactValidationError::EngineSchemaMismatch { .. }))));
        assert!(registry.get(&artifact.card.name).is_none());
    }
    let mut inconsistent = artifact.clone();
    inconsistent.payload.canonical_text.push('!');
    assert!(matches!(inconsistent.validate(), Err(ArtifactValidationError::ChecksumMismatch { .. })));
    assert!(CompiledCardArtifact::from_json(&inconsistent.to_json().unwrap()).is_err());
    assert!(matches!(materialize_artifact(&inconsistent),
        Err(ArtifactMaterializationError::InvalidArtifact(
            ArtifactValidationError::ChecksumMismatch { .. }))));
    let mut registry = ironsmith::cards::CardRegistry::new();
    assert!(matches!(registry.register_compiled_artifact(&inconsistent),
        Err(ArtifactRegistrationError::Invalid(ArtifactValidationError::ChecksumMismatch { .. }))));
    assert!(registry.get(&artifact.card.name).is_none());
}

#[test]
fn complete_source_bodies_require_the_current_cache_boundary_on_all_routes() {
    // Whole-source coverage supplements each cohort's semantic/runtime suite.
    // This envelope gate alone grants no measured card-recovery credit.
    for (fixture, names) in [
        (include_str!("../../../fixtures/intervening_predicate_cohort.json.fixture"),
            &["Aurora Champion", "Bull-Rush Bruiser", "Sickle Dancer", "Dragonfly Swarm", "Walltop Sentries"][..]),
        (include_str!("../../../fixtures/enter_copy_exceptions.json.fixture"),
            &["Protean Raider", "Sakashima of a Thousand Faces",
                "Sakashima of a Thousand Faces // Sakashima of a Thousand Faces"][..]),
        (include_str!("../../../fixtures/suspended_counter_bodies.json.fixture"),
            &["Fury Charm", "Shivan Sand-Mage", "Timebender", "Timecrafting"][..]),
        (include_str!("../../../fixtures/plural_controller_untap.json.fixture"),
            &["Breaching Leviathan", "Cone of Cold", "Dragon Turtle", "Lorthos, the Tidemaker", "Sudden Storm"][..]),
        (include_str!("../../../fixtures/die_result_programs.json.fixture"),
            &["Diviner's Portent", "Druid of the Emerald Grove", "Song of Inspiration", "Wyll's Reversal"][..]),
    ] {
        let rows: Vec<serde_json::Value> = serde_json::from_str(fixture).unwrap();
        for name in names {
            let row = rows.iter().find(|row| row["name"].as_str() == Some(*name)).unwrap();
            let faces = row["card_faces"].as_array().map(|faces| faces.iter().collect::<Vec<_>>())
                .unwrap_or_else(|| vec![row]);
            for face in faces {
                let name = face["name"].as_str().unwrap();
                let artifact = compile_source(name, &source(face));
                assert_old_envelopes_refused(&artifact);
            }
        }
    }
    // Independently transcribed complete frozen bodies, as in the dedicated
    // copy-entry semantic suite. No shortened copy-only source substitutes here.
    for (name, text) in [
        ("Chameleon, Master of Disguise", "Mana cost: {3}{U}\nType: Legendary Creature — Human Shapeshifter Villain\nPower/Toughness: 2/3\nYou may have Chameleon enter as a copy of a creature you control, except his name is Chameleon, Master of Disguise.\nMayhem {2}{U} (You may cast this card from your graveyard for {2}{U} if you discarded it this turn. Timing rules still apply.)"),
        ("Moritte of the Frost", "Mana cost: {2}{G}{U}{U}\nType: Legendary Snow Creature — Shapeshifter\nPower/Toughness: 0/0\nChangeling (This card is every creature type.)\nYou may have Moritte enter as a copy of a permanent you control, except it's legendary and snow in addition to its other types and, if it's a creature, it enters with two additional +1/+1 counters on it and has changeling."),
    ] {
        assert_old_envelopes_refused(&compile_source(name, text));
    }
}

#[test]
fn a_structurally_decodable_v13_payload_never_bypasses_envelope_refusal() {
    let current = compile_source("Synthetic cache boundary", "Type: Artifact");
    let mut stale = current.clone();
    stale.format_version = 13;
    stale.engine_schema_hash = PUBLISHED_13_SCHEMA.into();
    stale.refresh_checksum();
    assert_eq!(stale.compiler_version, current.compiler_version);
    assert_eq!(stale.payload, current.payload);
    assert!(matches!(materialize_artifact(&stale),
        Err(ArtifactMaterializationError::InvalidArtifact(
            ArtifactValidationError::UnsupportedFormat { found: 13, expected: 14 }))));
    let mut registry = ironsmith::cards::CardRegistry::new();
    assert!(matches!(registry.register_compiled_artifact(&stale),
        Err(ArtifactRegistrationError::Invalid(
            ArtifactValidationError::UnsupportedFormat { found: 13, expected: 14 }))));
    assert!(registry.get("Synthetic cache boundary").is_none());
    // Do not recover with materialize_definition(stale.payload.definition).
    // It has no envelope provenance and cannot distinguish these release owners.
}
