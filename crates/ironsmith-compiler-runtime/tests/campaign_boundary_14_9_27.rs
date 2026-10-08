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
    assert_eq!(FORMAT_VERSION, 17);
    assert_eq!(artifact.format_version, 17);
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
    let direct_bytes = serde_json::to_vec(&wire).unwrap();
    let direct_roundtrip: ironsmith_compiled_artifact::WireCardDefinition =
        serde_json::from_slice(&direct_bytes).unwrap();
    assert_eq!(direct_roundtrip, wire);
    assert_eq!(serde_json::to_vec(&direct_roundtrip).unwrap(), direct_bytes);
    // Each public compile call allocates a fresh CardId, which is retained in
    // the complete definition. These are within-route roundtrip assertions,
    // not direct/artifact identity equality or an ad hoc ID-normalizing codec.
    // Independent full-body semantic suites exercise both runtime definitions.
    materialize_definition(direct_roundtrip).unwrap();
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
    for version in [11, 12, 13, 14, 15, 16] {
        let mut stale = artifact.clone();
        stale.format_version = version;
        stale.refresh_checksum();
        assert!(matches!(stale.validate(), Err(ArtifactValidationError::UnsupportedFormat {
            found, expected: 17,
        }) if found == version));
        assert!(CompiledCardArtifact::from_json(&stale.to_json().unwrap()).is_err());
        assert!(matches!(materialize_artifact(&stale),
            Err(ArtifactMaterializationError::InvalidArtifact(
                ArtifactValidationError::UnsupportedFormat { found, expected: 17 }
            )) if found == version));
        let mut registry = ironsmith::cards::CardRegistry::new();
        assert!(matches!(registry.register_compiled_artifact(&stale),
            Err(ArtifactRegistrationError::Invalid(
                ArtifactValidationError::UnsupportedFormat { found, expected: 17 }
            )) if found == version));
        assert!(registry.get(&artifact.card.name).is_none());
    }
    for schema in [PUBLISHED_13_SCHEMA,
        "51232067d473dfff42849248b163193463ac225d2861f3bfe693e8d586b8f3b6",
        "ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7",
        "292e6db310f90613f13024fb4d135e405483443b81ef85b38f04e6755f6fdd7f",
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
            ArtifactValidationError::UnsupportedFormat { found: 13, expected: 17 }))));
    let mut registry = ironsmith::cards::CardRegistry::new();
    assert!(matches!(registry.register_compiled_artifact(&stale),
        Err(ArtifactRegistrationError::Invalid(
            ArtifactValidationError::UnsupportedFormat { found: 13, expected: 17 }))));
    assert!(registry.get("Synthetic cache boundary").is_none());
    // Do not recover with materialize_definition(stale.payload.definition).
    // It has no envelope provenance and cannot distinguish these release owners.
}

// NEXT06 bodies retain their model semantics; fresh output follows the current
// source-cache boundary. This does not claim an old engine is compatible.
#[test]
fn step_local_native_history_uses_the_current_compiled_definition_boundary() {
    assert_eq!(FORMAT_VERSION, 17);
    let rows: Vec<serde_json::Value> = serde_json::from_str(
        include_str!("../../../fixtures/combat_blocked_status.json.fixture")).unwrap();
    for name in ["Deep Wood", "Heavy Fog"] {
        let row = rows.iter().find(|row| row["name"].as_str() == Some(name)).unwrap();
        let artifact = compile_source(name, &source(row));
        assert_old_envelopes_refused(&artifact);
    }
}

// Oct8 compiler-only successor. These fixture additions are supplied by the
// coordinated source corrections; the boundary is not independently deployable.
#[test]
fn oct8_full_bodies_require_fresh_independent_compilation_and_v16_admission() {
    let chosen: Vec<serde_json::Value> = serde_json::from_str(
        include_str!("../../../fixtures/chosen_type_domain_regressions.json.fixture")).unwrap();
    let untap: Vec<serde_json::Value> = serde_json::from_str(
        include_str!("../../../fixtures/plural_controller_untap.json.fixture")).unwrap();
    for row in &chosen {
        let text = format!("{}{}", row["metadata"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
        assert_old_envelopes_refused(&compile_source(row["name"].as_str().unwrap(), &text));
    }
    for name in ["Send to Sleep", "Icy Blast"] {
        let row = untap.iter().find(|row| row["name"].as_str() == Some(name)).unwrap();
        assert_old_envelopes_refused(&compile_source(name, &source(row)));
    }
    // Printed metadata and complete Oracle body; cohort semantics separately
    // exercise die results, activation costs and off-battlefield destinations.
    assert_old_envelopes_refused(&compile_source("Loathsome Troll",
        "Mana cost: {3}{G}{G}\nType: Creature — Troll\nPower/Toughness: 6/2\n{3}{G}: Roll a d20. Activate only if this card is in your graveyard.\n1—9 | Put this card on top of your library.\n10—19 | Return this card to your hand.\n20 | Return this card to the battlefield tapped."));
}

#[test]
fn same_session_refusals_and_permissive_compiles_do_not_contaminate_fresh_routes() {
    let name = "Session source isolation";
    let clean = "Type: Enchantment\nAs this enchantment enters, choose a creature type.\nCreatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.";
    let unsupported = clean.replace("aren't on the battlefield.", "aren't on the battlefield mystery.");
    for _ in 0..2 {
        // Exercise a permissive request before a strict request for the exact
        // same name and text. A permissive result cannot satisfy strict admission.
        let _ = compile_to_artifact(name, &unsupported, true);
        let (result, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_artifact(name, &unsupported, false));
        assert!(result.is_err() || loss.is_lossy());
        let current = compile_source(name, clean);
        assert_old_envelopes_refused(&current);
        // Registration checks the envelope on every call, even when the same
        // name already exists in this registry. Refusal must preserve its entry.
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&current).unwrap();
        let before = encode_runtime_definition(registry.get(name).unwrap().clone()).unwrap();
        let mut stale = current.clone();
        stale.format_version = 15;
        stale.engine_schema_hash =
            "ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7".into();
        stale.refresh_checksum();
        for _ in 0..2 {
            assert!(matches!(registry.register_compiled_artifact(&stale),
                Err(ArtifactRegistrationError::Invalid(
                    ArtifactValidationError::UnsupportedFormat { found: 15, expected: 17 }))));
            assert_eq!(encode_runtime_definition(registry.get(name).unwrap().clone()).unwrap(), before);
        }
        compile_source(name, clean);
    }
}

// Source-authored, UNRUN. The three fixture files and source repairs are supplied
// by the separately reviewed second Oct8 cohort. This boundary branch alone is
// deliberately not a standalone passing/deployable integration.
#[test]
fn second_oct8_complete_bodies_require_independent_fresh_v16_routes() {
    for (fixture, names) in [
        (include_str!("../../../fixtures/source_must_be_blocked.json.fixture"),
            &["Anzrag, the Quake-Mole", "Glorfindel, Dauntless Rescuer", "Loathsome Catoblepas"][..]),
        (include_str!("../../../fixtures/temporary_additional_land_caps.json.fixture"),
            &["Summer Bloom", "Journey of Discovery"][..]),
    ] {
        let rows: Vec<serde_json::Value> = serde_json::from_str(fixture).unwrap();
        for name in names {
            let row = rows.iter().find(|row| row["name"].as_str() == Some(*name)).unwrap();
            assert_old_envelopes_refused(&compile_source(name, &source(row)));
        }
    }
    let fixture: serde_json::Value = serde_json::from_str(
        include_str!("../../../fixtures/titania_alternative_cost.json.fixture")).unwrap();
    let row = &fixture["card"];
    assert_eq!(row["name"], "Titania, Rugged Rumbler");
    assert_old_envelopes_refused(&compile_source("Titania, Rugged Rumbler", &source(row)));
}

#[test]
fn second_oct8_same_name_strict_and_permissive_requests_keep_source_admission() {
    for (name, clean, unsupported) in [
        ("Source requirement isolation",
            "Type: Creature — Beast\nPower/Toughness: 2/2\n{G}: This creature must be blocked this turn if able.",
            "Type: Creature — Beast\nPower/Toughness: 2/2\n{G}: This creature with flying must be blocked this turn if able."),
        ("Land ceiling isolation",
            "Type: Sorcery\nYou may play up to three additional lands this turn.",
            "Type: Sorcery\nYou may play up to three nonsense additional lands this turn."),
    ] {
        for _ in 0..2 {
            let _ = compile_to_artifact(name, unsupported, true);
            let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_artifact(name, unsupported, false));
            assert!(artifact.is_err() || loss.is_lossy(), "artifact {name}");
            let _ = compile_to_runtime_definition(name, unsupported, true);
            let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_runtime_definition(name, unsupported, false));
            assert!(direct.is_err() || loss.is_lossy(), "direct {name}");
            assert_old_envelopes_refused(&compile_source(name, clean));
            compile_source(name, clean);
        }
    }
}

// Source-only successor gates. This is independent direct compilation, never a
// decoder fallback taking the payload from a refused v16 envelope.
#[test]
fn exact_permission_complete_raw_and_normalized_sources_require_v17() {
    let frozen: serde_json::Value = serde_json::from_str(include_str!(
        "../../../reports/countered-spell-durable-permission-20261008/frozen-inputs.json"
    )).unwrap();
    for row in frozen["actual_card_metadata"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let diagnostic = frozen["retained_results"].as_array().unwrap().iter()
            .find(|record| record["oracle_id"] == row["oracle_id"]).unwrap();
        for body in [row["oracle_text"].as_str().unwrap(),
            diagnostic["normalized_oracle_text"].as_str().unwrap()] {
            let mut complete = row.clone();
            complete["oracle_text"] = body.into();
            for _ in 0..2 {
                let current = compile_source(name, &source(&complete));
                assert_old_envelopes_refused(&current);
                let mut registry = ironsmith::cards::CardRegistry::new();
                registry.register_compiled_artifact(&current).unwrap();
                let before = encode_runtime_definition(registry.get(name).unwrap().clone()).unwrap();
                for _ in 0..2 {
                    let mut stale = current.clone();
                    stale.format_version = 16;
                    stale.refresh_checksum();
                    assert!(matches!(registry.register_compiled_artifact(&stale),
                        Err(ArtifactRegistrationError::Invalid(ArtifactValidationError::UnsupportedFormat {
                            found: 16, expected: 17,
                        }))));
                    assert_eq!(encode_runtime_definition(registry.get(name).unwrap().clone()).unwrap(), before);
                }
                // A refused cached card never supplies direct-route inputs.
                compile_source(name, &source(&complete));
            }
        }
    }
}

#[test]
fn exact_permission_same_session_loss_isolation_and_fresh_clean_sources() {
    let clean = "Type: Instant\nCounter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard. You may play it without paying its mana cost for as long as it remains exiled.";
    let unsupported = clean.replace("for as long as it remains exiled", "until your next turn");
    for _ in 0..2 {
        let name = "Exact permission cache isolation";
        let _ = compile_to_artifact(name, &unsupported, true);
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &unsupported, false));
        assert!(result.is_err() || loss.is_lossy());
        let _ = compile_to_runtime_definition(name, &unsupported, true);
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &unsupported, false));
        assert!(result.is_err() || loss.is_lossy());
        assert_old_envelopes_refused(&compile_source(name, clean));
        compile_source(name, clean);
    }
}

#[test]
fn lesson_full_bodies_require_fresh_v17_routes_after_normalization_prerequisite() {
    // Requires b38192bf and its separately reviewed test correction integrated.
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/intervening_predicate_cohort.json.fixture")).unwrap();
    for name in ["Dragonfly Swarm", "Walltop Sentries"] {
        let row = rows.iter().find(|row| row["name"].as_str() == Some(name)).unwrap();
        for text in [source(row), source(row).replace("there's", "there’s"),
            source(row).replace("there's", "there is")] {
            assert_old_envelopes_refused(&compile_source(name, &text));
        }
    }
}
