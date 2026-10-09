//! Source-authored current admission gate. All compilation and runtime work UNRUN.
use ironsmith_compiled_artifact::{ArtifactValidationError, CompiledCardArtifact,
    ENGINE_SCHEMA_HASH, FORMAT_VERSION};
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::{materialize_artifact,
    encode_runtime_definition, materialize_definition};

const PUBLISHED_12_SCHEMA: &str = "fbc604c03fa9eed8de6567576052324b9c8bee8d9ddc319f2ae25bc0a62b9c5d";
const OLDER_SCHEMAS: &[&str] = &[
    PUBLISHED_12_SCHEMA,
    "cf9f06e2cea9c4facdfe9b4aad19eaa4bca1062e4e9c9f28d22de18920cbd401",
    "9d0e162e131ecfbaf850e2331cd33a54ea932fb48eecd55978e271a26bc3938c",
    "2b6fde5114ec4007309dcdb183ed453ec30033f59d1600319640ba45877a4c9f",
];

fn source(fixture: &str, name: &str) -> String {
    let fixture: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let rows = fixture.as_array().or_else(|| fixture["cards"].as_array()).unwrap();
    let row = rows.iter().map(|row| row.get("source").unwrap_or(row))
        .find(|row| row["name"] == name).unwrap();
    if let Some(text) = row["text"].as_str() { return text.into(); }
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() { text.push_str(&format!("Loyalty: {loyalty}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    text
}

#[test]
fn complete_candidate_sources_require_current_envelopes_on_every_materialization_route() {
    let cohorts: &[(&str, &[&str])] = &[
        (include_str!("../../../fixtures/conditional_damage_amounts.json.fixture"),
            &["Burst Lightning", "Roil Eruption", "Shivan Fire", "Frost Bite", "Burning Hands", "Voltage Surge", "Akoum Hellkite"]),
        (include_str!("../../../fixtures/delayed_player_attack_declarations.json.fixture"),
            &["Dalkovan Encampment", "Jaya, Fiery Negotiator", "Roads Go Ever, Ever On"]),
        (include_str!("../../../fixtures/player_counter_anthems.json.fixture"),
            &["Kalemne, Disciple of Iroas", "Kelsien, the Plague", "Minthara, Merciless Soul", "Mycosynth Fiend", "Vishgraz, the Doomhive"]),
        (include_str!("../../../fixtures/card-failure-campaign/repeat-process-boundaries.json"),
            &["Another Round", "Countryside Crusher", "Claim Jumper", "Grindstone", "Professor Onyx", "Scalpelexis", "Trade Secrets", "Zimone and Dina"]),
        (include_str!("../../../fixtures/next_step_durations.json.fixture"), &["Fatigue", "Misstep"]),
        (include_str!("../../../fixtures/residual_static_condition_cohort.json.fixture"),
            &["Deepway Navigator", "Essence Leak", "The Ur-Dragon"]),
    ];
    assert_eq!(FORMAT_VERSION, 18);
    for &(fixture, names) in cohorts {
        for &name in names {
            let text = source(fixture, name);
            let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_runtime_definition(name, &text, false));
            let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
            assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
            let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
            let (artifact, _) = result.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
            assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
            assert_eq!(artifact.format_version, 17);
            assert_eq!(artifact.engine_schema_hash, ENGINE_SCHEMA_HASH);
            artifact.validate().unwrap();
            let bytes = artifact.to_json().unwrap();
            let restored = CompiledCardArtifact::from_json(&bytes).unwrap();
            assert_eq!(restored, artifact);
            assert_eq!(restored.to_json().unwrap(), bytes);
            materialize_artifact(&restored).unwrap();
            // The independently compiled route may retain canonical models.
            // This is not a claim that every fresh native owner has an encoder.
            let encoded = encode_runtime_definition(direct).unwrap();
            materialize_definition(serde_json::from_slice(&serde_json::to_vec(&encoded).unwrap()).unwrap()).unwrap();

            for version in [11, 12] {
                let mut old = artifact.clone();
                old.format_version = version;
                old.refresh_checksum();
                assert!(matches!(old.validate(), Err(ArtifactValidationError::UnsupportedFormat {
                    found, expected: 17,
                }) if found == version));
                assert!(CompiledCardArtifact::from_json(&old.to_json().unwrap()).is_err());
                assert!(materialize_artifact(&old).is_err());
            }
            for &schema in OLDER_SCHEMAS {
                let mut relabeled = artifact.clone();
                relabeled.engine_schema_hash = schema.into();
                relabeled.refresh_checksum();
                assert!(matches!(relabeled.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
                assert!(CompiledCardArtifact::from_json(&relabeled.to_json().unwrap()).is_err());
                assert!(materialize_artifact(&relabeled).is_err());
            }
        }
    }
}

#[test]
fn source_admission_cannot_route_new_durations_to_an_unrepresented_effect_owner() {
    // These are whole-body negative sources, not successful fragments or
    // handcrafted model envelopes. Lowering must not emit an indefinite rule.
    for text in [
        "Type: Sorcery\nTarget player gains 1 life. Target creature has base power 5 during that player's next untap step.",
        "Type: Sorcery\nTarget creature gets +1/+1 during that player's next untap step.",
        "Type: Sorcery\nTarget creature gains flying during that player's next untap step.",
        "Type: Sorcery\nTarget creature doesn't untap until its controller's next untap step.",
        "Type: Sorcery\nTarget creature can't attack until its controller's next untap step.",
        "Type: Sorcery\nIf you control an Island, creatures target player controls don't untap during that player's next untap step.",
    ] {
        let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("Unrepresented duration owner", text, false));
        assert!(direct.is_err() || loss.is_lossy(), "direct route admitted an unsupported duration owner: {text}");
        let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_artifact("Unrepresented duration owner", text, false));
        assert!(artifact.is_err() || loss.is_lossy(), "artifact route admitted an unsupported duration owner: {text}");
    }
}
