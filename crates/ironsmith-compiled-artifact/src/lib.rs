//! Stable, deterministic boundary between card compilation and engine loading.

use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

// Version 18 admits the CF8 payload models and main color-identity exclusions.
// Historical artifacts remain unchanged and require recompilation for admission.
// See architecture/cf8-main-integrated-schema.descriptor.
pub const FORMAT_VERSION: u32 = 18;
pub const ENGINE_SCHEMA_HASH: &str =
    "d3498df224a2c21459f6daac5d8714195702b19406d82b4983fca3d7622ce25d";

/// A compiler effect transported without linking compiler code into the
/// engine. The payload is decoded lazily into the exact canonical schema type
/// requested by the engine's generic effect-model interpreter.
#[derive(Clone)]
pub struct WireEffect {
    kind: String,
    payload: Value,
    decoded: Arc<OnceLock<Result<Box<dyn Any + Send + Sync>, String>>>,
}

impl WireEffect {
    pub fn new(kind: impl Into<String>, payload: Value) -> Self {
        Self {
            kind: kind.into(),
            payload,
            decoded: Arc::new(OnceLock::new()),
        }
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn payload(&self) -> &Value {
        &self.payload
    }

    pub fn downcast_with<T, F>(&self, decode: F) -> Option<&T>
    where
        T: Any,
        F: FnOnce(Value) -> Result<Box<dyn Any + Send + Sync>, String>,
    {
        self.decoded
            .get_or_init(|| decode(self.payload.clone()))
            .as_ref()
            .ok()
            .and_then(|value| value.downcast_ref::<T>())
    }

    pub fn decode_error(&self) -> Option<&str> {
        self.decoded
            .get()
            .and_then(|decoded| decoded.as_ref().err().map(String::as_str))
    }
}

impl fmt::Debug for WireEffect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WireEffect")
            .field("kind", &self.kind)
            .field("payload", &self.payload)
            .finish()
    }
}

impl PartialEq for WireEffect {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.payload == other.payload
    }
}

impl Serialize for WireEffect {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct as _;
        let mut state = serializer.serialize_struct("CompiledEffect", 2)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("payload", &self.payload)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for WireEffect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Envelope {
            kind: String,
            payload: Value,
        }

        let envelope = Envelope::deserialize(deserializer)?;
        Ok(Self::new(envelope.kind, envelope.payload))
    }
}

pub fn stable_payload_kind(type_name: &str) -> &str {
    type_name
        .split('<')
        .next()
        .unwrap_or(type_name)
        .rsplit("::")
        .next()
        .unwrap_or(type_name)
}

pub type WireTrigger = ironsmith_core::trigger_model::Trigger;
pub type WireCost = ironsmith_core::Cost<WireEffect>;
pub type WireStaticAbility = ironsmith_core::StaticAbility<
    WireTrigger,
    WireEffect,
    WireCost,
    ironsmith_core::ThisSpellCostCondition,
>;
pub type WireAbility =
    ironsmith_core::Ability<WireStaticAbility, WireTrigger, WireEffect, WireCost>;
pub type WireAlternativeCastingMethod = ironsmith_core::AlternativeCastingMethod<
    WireEffect,
    WireCost,
    ironsmith_core::ThisSpellCostCondition,
>;
pub type WireOptionalCost = ironsmith_core::OptionalCost<WireCost>;
pub type WireCardDefinition = ironsmith_core::CardDefinition<
    WireAbility,
    WireEffect,
    WireCost,
    WireAlternativeCastingMethod,
    WireOptionalCost,
>;
pub type WireContinuousTarget = ironsmith_core::CompiledContinuousEffectTarget;
pub type WireContinuousModification =
    ironsmith_core::CompiledContinuousModification<WireStaticAbility, WireAbility>;
pub type WireGrantable = ironsmith_core::Grantable<
    WireStaticAbility,
    WireEffect,
    WireCost,
    ironsmith_core::ThisSpellCostCondition,
>;
pub type WireGrantSpec = ironsmith_core::GrantSpec<
    WireStaticAbility,
    WireEffect,
    WireCost,
    ironsmith_core::ThisSpellCostCondition,
>;
pub type WireGrantDuration = ironsmith_core::GrantDuration;
pub type WireDerivedAlternativeCast = ironsmith_core::DerivedAlternativeCast<WireCost>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireEmblemDescription {
    pub name: String,
    pub text: String,
    pub abilities: Vec<WireAbility>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(
    clippy::large_enum_variant,
    reason = "wire variants preserve the canonical runtime value shapes without allocation-only schema wrappers"
)]
pub enum WireRuntimeModification {
    ModifyPowerToughness {
        power: ironsmith_core::Value,
        toughness: ironsmith_core::Value,
    },
    ChangeControllerToEffectController,
    ChangeControllerToPlayer(ironsmith_core::PlayerFilter),
    CopyOf {
        source: ironsmith_core::ChooseSpec,
        preserve_source_abilities: bool,
        name_override: Option<String>,
        name_override_surface: Option<ironsmith_core::SourceReferenceSurface>,
        add_supertypes: Vec<ironsmith_core::Supertype>,
        copy_exception_surface: Option<String>,
    },
    RemoveAllAbilities,
    RemoveThisAbility,
    SetAuraAttachmentFilter(ironsmith_core::AuraAttachmentFilter),
    /// Abilities added as copiable exceptions, applied in layer 1 rather than ordinary grants.
    CopyOfWithAbilities {
        source: ironsmith_core::ChooseSpec,
        preserve_source_abilities: bool,
        name_override: Option<String>,
        name_override_surface: Option<ironsmith_core::SourceReferenceSurface>,
        add_supertypes: Vec<ironsmith_core::Supertype>,
        copy_exception_surface: Option<String>,
        abilities: Vec<WireAbility>,
    },
    /// "except it doesn't copy that creature's color" (CR 707.9b).
    RetainSourceColors,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireScaleXValueEffect {
    pub target: ironsmith_core::ChooseSpec,
    pub multiplier: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireImprintFromHandEffect {
    pub filter: ironsmith_core::ObjectFilter,
}

/// Convert any structurally identical compiler-side schema instantiation into
/// the transport-owned wire instantiation. Trait-object effects serialize as
/// tagged payload envelopes, so no runtime/compiler dependency is required.
pub fn wire_definition_from_serializable<T>(
    definition: &T,
) -> Result<WireCardDefinition, serde_json::Error>
where
    T: Serialize,
{
    serde_json::from_value(serde_json::to_value(definition)?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactCardId(pub u32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactCardIdentity {
    pub local_id: ArtifactCardId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_face: Option<ArtifactCardId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_face_layout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDiagnostics {
    pub error_count: u32,
    pub warning_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledCardPayload {
    /// Schema-shaped lowering payload. Artifact producers must use stable field
    /// names and values; runtime-local IDs and function pointers are forbidden.
    pub definition: WireCardDefinition,
    pub canonical_text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ability_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledCardArtifact {
    pub format_version: u32,
    pub engine_schema_hash: String,
    pub card: ArtifactCardIdentity,
    pub payload: CompiledCardPayload,
    pub compiler_version: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub compiler_facts: BTreeMap<String, String>,
    pub diagnostics: ArtifactDiagnostics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_score: Option<f32>,
    pub source_checksum: String,
    pub payload_checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactValidationError {
    UnsupportedFormat { found: u32, expected: u32 },
    EngineSchemaMismatch { found: String, expected: String },
    ChecksumMismatch { found: String, expected: String },
}

impl fmt::Display for ArtifactValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat { found, expected } => {
                write!(
                    formatter,
                    "compiled-card format {found} is unsupported; expected {expected}"
                )
            }
            Self::EngineSchemaMismatch { found, expected } => {
                write!(
                    formatter,
                    "compiled-card engine schema {found} does not match {expected}"
                )
            }
            Self::ChecksumMismatch { found, expected } => {
                write!(
                    formatter,
                    "compiled-card checksum {found} does not match {expected}"
                )
            }
        }
    }
}

impl std::error::Error for ArtifactValidationError {}

impl CompiledCardArtifact {
    pub fn new(
        card: ArtifactCardIdentity,
        payload: CompiledCardPayload,
        compiler_version: impl Into<String>,
        source: &[u8],
    ) -> Self {
        let mut artifact = Self {
            format_version: FORMAT_VERSION,
            engine_schema_hash: ENGINE_SCHEMA_HASH.to_string(),
            card,
            payload,
            compiler_version: compiler_version.into(),
            compiler_facts: BTreeMap::new(),
            diagnostics: ArtifactDiagnostics::default(),
            semantic_score: None,
            source_checksum: sha256_hex(source),
            payload_checksum: String::new(),
        };
        artifact.refresh_checksum();
        artifact
    }

    pub fn refresh_checksum(&mut self) {
        self.payload_checksum.clear();
        let encoded = serde_json::to_vec(self).expect("compiled-card artifact must serialize");
        self.payload_checksum = sha256_hex(&encoded);
    }

    pub fn validate(&self) -> Result<(), ArtifactValidationError> {
        if self.format_version != FORMAT_VERSION {
            return Err(ArtifactValidationError::UnsupportedFormat {
                found: self.format_version,
                expected: FORMAT_VERSION,
            });
        }
        if self.engine_schema_hash != ENGINE_SCHEMA_HASH {
            return Err(ArtifactValidationError::EngineSchemaMismatch {
                found: self.engine_schema_hash.clone(),
                expected: ENGINE_SCHEMA_HASH.to_string(),
            });
        }
        let mut checksum_input = self.clone();
        checksum_input.payload_checksum.clear();
        let encoded = serde_json::to_vec(&checksum_input)
            .expect("validated compiled-card artifact must serialize");
        let expected = sha256_hex(&encoded);
        if self.payload_checksum != expected {
            return Err(ArtifactValidationError::ChecksumMismatch {
                found: self.payload_checksum.clone(),
                expected,
            });
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, ArtifactDecodeError> {
        let artifact: Self = serde_json::from_slice(bytes).map_err(ArtifactDecodeError::Json)?;
        artifact
            .validate()
            .map_err(ArtifactDecodeError::Validation)?;
        Ok(artifact)
    }
}

#[derive(Debug)]
pub enum ArtifactDecodeError {
    Json(serde_json::Error),
    Validation(ArtifactValidationError),
}

impl fmt::Display for ArtifactDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => error.fmt(formatter),
            Self::Validation(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ArtifactDecodeError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> CompiledCardArtifact {
        let mut artifact = CompiledCardArtifact::new(
            ArtifactCardIdentity {
                local_id: ArtifactCardId(1),
                name: "Cold Build Fixture".to_string(),
                face_name: None,
                other_face: None,
                linked_face_layout: None,
            },
            CompiledCardPayload {
                definition: WireCardDefinition::new(
                    ironsmith_core::CardBuilder::new(
                        ironsmith_core::CardId::from_raw(1),
                        "Cold Build Fixture",
                    )
                    .build(),
                ),
                canonical_text: "{T}: Add {C}.".to_string(),
                ability_labels: vec!["Add {C}".to_string()],
            },
            "ironsmith-compiler/0.1.0",
            b"{T}: Add {C}.",
        );
        artifact
            .compiler_facts
            .insert("allowUnsupported".to_string(), "false".to_string());
        artifact.refresh_checksum();
        artifact
    }

    #[test]
    fn current_golden_matches_the_integrated_source() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/v18.json");
        let actual = String::from_utf8(fixture().to_json().unwrap()).unwrap();
        if std::env::var_os("IRONSMITH_UPDATE_ARTIFACT_GOLDEN").is_some() {
            std::fs::write(&path, &actual).unwrap();
        }
        let expected = std::fs::read_to_string(path).expect("current artifact golden must be provisioned");
        let decoded = CompiledCardArtifact::from_json(expected.as_bytes()).unwrap();
        assert_eq!(decoded.format_version, FORMAT_VERSION);
        assert_eq!(decoded.engine_schema_hash, ENGINE_SCHEMA_HASH);
        assert_eq!(actual.trim(), expected.trim());
    }

    #[test]
    fn historical_v3_golden_keeps_original_evidence_and_is_refused() {
        // Retain the real historical bytes. The previously referenced v5
        // fixture does not exist in the repository or its available history.
        let bytes = include_bytes!("../fixtures/v3.json");
        let historical: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(historical["formatVersion"], 3);
        assert!(CompiledCardArtifact::from_json(bytes).is_err());
    }

    #[test]
    fn preceding_v17_descriptor_keeps_exact_historical_identity() {
        let descriptor = include_bytes!("../../../architecture/oct8-exact-permission-schema.descriptor");
        assert_eq!(sha256_hex(descriptor), "a4bb7964a2b6b477e4d128c655ca747453c74a9d5d1cf119ac7daf399fb840c0");
        let mut old = fixture();
        old.format_version = 17;
        old.refresh_checksum();
        assert!(matches!(old.validate(), Err(ArtifactValidationError::UnsupportedFormat {
            found: 17, expected: 18,
        })));
        old.format_version = FORMAT_VERSION;
        old.engine_schema_hash = sha256_hex(descriptor);
        old.refresh_checksum();
        assert!(matches!(old.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
    }

    #[test]
    fn current_descriptor_matches_the_declared_schema_fingerprint() {
        let descriptor = include_bytes!("../../../architecture/cf8-main-integrated-schema.descriptor");
        assert!(!descriptor.ends_with(b"\n"));
        assert_eq!(sha256_hex(descriptor), ENGINE_SCHEMA_HASH);
    }

    #[test]
    fn preceding_v16_descriptor_keeps_exact_historical_identity() {
        let descriptor = include_bytes!("../../../architecture/oct8-second-cardrepair-schema.descriptor");
        assert!(!descriptor.ends_with(b"\n"));
        assert_eq!(sha256_hex(descriptor),
            "51232067d473dfff42849248b163193463ac225d2861f3bfe693e8d586b8f3b6");
        let mut old = fixture();
        old.format_version = 16;
        old.refresh_checksum();
        assert!(matches!(old.validate(), Err(ArtifactValidationError::UnsupportedFormat {
            found: 16, expected: 18,
        })));
        old.format_version = FORMAT_VERSION;
        old.engine_schema_hash = "51232067d473dfff42849248b163193463ac225d2861f3bfe693e8d586b8f3b6".into();
        old.refresh_checksum();
        assert!(matches!(old.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
    }

    #[test]
    fn preceding_oct8_descriptor_keeps_its_exact_original_bytes() {
        let descriptor = include_bytes!("../../../architecture/oct8-cardrepair-schema.descriptor");
        assert!(!descriptor.ends_with(b"\n"));
        assert_eq!(sha256_hex(descriptor),
            "ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7");
    }

    #[test]
    fn published_third_series_descriptor_keeps_its_exact_original_bytes() {
        let descriptor = include_bytes!("../../../architecture/next-series-03-schema.descriptor");
        assert!(!descriptor.ends_with(b"\n"));
        assert_eq!(sha256_hex(descriptor),
            "292e6db310f90613f13024fb4d135e405483443b81ef85b38f04e6755f6fdd7f");
    }

    #[test]
    fn published_first_series_descriptor_keeps_its_exact_original_bytes() {
        let descriptor = include_bytes!("../../../architecture/next-series-01-schema.descriptor");
        assert!(!descriptor.ends_with(b"\n"));
        assert_eq!(sha256_hex(descriptor),
            "fbc604c03fa9eed8de6567576052324b9c8bee8d9ddc319f2ae25bc0a62b9c5d");
    }

    #[test]
    fn published_second_series_descriptor_keeps_its_exact_original_bytes() {
        let descriptor = include_bytes!("../../../architecture/next-series-02-schema.descriptor");
        assert!(!descriptor.ends_with(b"\n"));
        assert_eq!(sha256_hex(descriptor),
            "cbaf3a819cee97d5351ddc85c7789a85508caace7573a7f8aa3fe7af0b87854c");
    }

    #[test]
    fn compiler_version_is_not_provenance_and_cannot_admit_the_previous_cache() {
        // Synthetic envelope, not a generated historical v15 fixture. The same
        // compiler version is used on both sides of this source-only boundary.
        let mut old = fixture();
        assert_eq!(old.compiler_version, "ironsmith-compiler/0.1.0");
        old.format_version = 15;
        old.engine_schema_hash =
            "ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7".into();
        old.refresh_checksum();
        assert!(matches!(old.validate(),
            Err(ArtifactValidationError::UnsupportedFormat { found: 15, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&old.to_json().unwrap()).is_err());

        old.format_version = FORMAT_VERSION;
        old.refresh_checksum();
        assert!(matches!(old.validate(),
            Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(CompiledCardArtifact::from_json(&old.to_json().unwrap()).is_err());
    }

    #[test]
    fn rewritten_format_hash_and_checksum_are_not_artifact_authentication() {
        // Model-shaped synthetic bytes make the limit explicit: a checksum is
        // not a signature and validate cannot prove source recompilation.
        let mut synthetic = fixture();
        synthetic.format_version = 13;
        synthetic.engine_schema_hash =
            "cbaf3a819cee97d5351ddc85c7789a85508caace7573a7f8aa3fe7af0b87854c".into();
        synthetic.refresh_checksum();
        synthetic.format_version = FORMAT_VERSION;
        synthetic.engine_schema_hash = ENGINE_SCHEMA_HASH.into();
        assert!(matches!(synthetic.validate(),
            Err(ArtifactValidationError::ChecksumMismatch { .. })));
        synthetic.refresh_checksum();
        synthetic.validate().unwrap();
        assert_eq!(CompiledCardArtifact::from_json(&synthetic.to_json().unwrap()).unwrap(), synthetic);
        // Arbitrary compiler_version claims also pass when checksum-consistent.
        synthetic.compiler_version = "unverified producer claim".into();
        synthetic.refresh_checksum();
        synthetic.validate().unwrap();
    }

    #[test]
    fn published_v12_envelopes_require_regeneration_even_with_a_current_payload() {
        let mut old = fixture();
        old.format_version = 12;
        old.refresh_checksum();
        assert!(matches!(old.validate(),
            Err(ArtifactValidationError::UnsupportedFormat { found: 12, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&old.to_json().unwrap()).is_err());
    }

    #[test]
    fn previous_artifacts_require_source_regeneration_even_with_fresh_checksums() {
        let mut previous = fixture();
        previous.format_version = 11;
        previous.refresh_checksum();
        assert!(matches!(previous.validate(),
            Err(ArtifactValidationError::UnsupportedFormat { found: 11, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&previous.to_json().unwrap()).is_err());

        for schema in [
            "ce114b87adedd56e98d8472b5d4e3db436ab9408ff2dc3176a286c32132462b7",
            "292e6db310f90613f13024fb4d135e405483443b81ef85b38f04e6755f6fdd7f",
            "cbaf3a819cee97d5351ddc85c7789a85508caace7573a7f8aa3fe7af0b87854c",
            "fbc604c03fa9eed8de6567576052324b9c8bee8d9ddc319f2ae25bc0a62b9c5d",
            "cf9f06e2cea9c4facdfe9b4aad19eaa4bca1062e4e9c9f28d22de18920cbd401",
            // Earlier unpublished source proposal, never historical authority.
            "9d0e162e131ecfbaf850e2331cd33a54ea932fb48eecd55978e271a26bc3938c",
            // Superseded first-series draft before counter/static semantic integration.
            "2b6fde5114ec4007309dcdb183ed453ec30033f59d1600319640ba45877a4c9f",
        ] {
            let mut relabeled = fixture();
            relabeled.engine_schema_hash = schema.into();
            relabeled.refresh_checksum();
            assert!(matches!(relabeled.validate(),
                Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
            assert!(CompiledCardArtifact::from_json(&relabeled.to_json().unwrap()).is_err());
        }
    }

    #[test]
    fn previous_artifact_versions_are_rejected_instead_of_inventing_definition_metadata() {
        let mut previous = fixture();
        previous.format_version = 4;
        assert!(matches!(previous.validate(), Err(ArtifactValidationError::UnsupportedFormat { found: 4, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(include_bytes!("../fixtures/v3.json")).is_err());
        let mut missing_keyword_identity = fixture();
        missing_keyword_identity.format_version = 5;
        missing_keyword_identity.refresh_checksum();
        assert!(matches!(missing_keyword_identity.validate(), Err(ArtifactValidationError::UnsupportedFormat { found: 5, expected: 18 })));
        let mut ambiguous_mana_origin = fixture();
        ambiguous_mana_origin.format_version = 6;
        ambiguous_mana_origin.refresh_checksum();
        assert!(matches!(ambiguous_mana_origin.validate(), Err(ArtifactValidationError::UnsupportedFormat { found: 6, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&ambiguous_mana_origin.to_json().unwrap()).is_err());
        let mut missing_numeric_counter_owners = fixture();
        missing_numeric_counter_owners.format_version = 7;
        missing_numeric_counter_owners.refresh_checksum();
        assert!(matches!(missing_numeric_counter_owners.validate(), Err(ArtifactValidationError::UnsupportedFormat { found: 7, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&missing_numeric_counter_owners.to_json().unwrap()).is_err());
    }

    #[test]
    fn pre_prepared_artifacts_require_recompilation_even_with_valid_checksums() {
        let mut old_format = fixture();
        old_format.format_version = 8;
        old_format.refresh_checksum();
        assert!(matches!(old_format.validate(),
            Err(ArtifactValidationError::UnsupportedFormat { found: 8, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&old_format.to_json().unwrap()).is_err());

        let mut old_schema = fixture();
        old_schema.engine_schema_hash =
            "cb108f20f047d8702f593fc004a8f8e4cebc1c7cd29a9b0eef7b7590cf9adcad".into();
        old_schema.refresh_checksum();
        assert!(matches!(old_schema.validate(),
            Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(CompiledCardArtifact::from_json(&old_schema.to_json().unwrap()).is_err());
    }

    #[test]
    fn pre_protection_artifacts_require_recompilation_even_with_valid_checksums() {
        let mut old_format = fixture();
        old_format.format_version = 9;
        old_format.refresh_checksum();
        assert!(matches!(old_format.validate(),
            Err(ArtifactValidationError::UnsupportedFormat { found: 9, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&old_format.to_json().unwrap()).is_err());

        let mut old_schema = fixture();
        old_schema.engine_schema_hash =
            "b3895accd8d36443d5ec72a6985ebf4e96650975eaa53d8581e484040dd312d7".into();
        old_schema.refresh_checksum();
        assert!(matches!(old_schema.validate(),
            Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(CompiledCardArtifact::from_json(&old_schema.to_json().unwrap()).is_err());
    }

    #[test]
    fn pre_activation_combat_class_artifacts_require_source_regeneration() {
        let mut old_format = fixture();
        old_format.format_version = 10;
        old_format.refresh_checksum();
        assert!(matches!(old_format.validate(),
            Err(ArtifactValidationError::UnsupportedFormat { found: 10, expected: 18 })));
        assert!(CompiledCardArtifact::from_json(&old_format.to_json().unwrap()).is_err());
        let mut relabeled = fixture();
        relabeled.engine_schema_hash = "e27b521de2a44a2c1c3349a8b8cbf5392da6882c14270112de24faecced0adc9".into();
        relabeled.refresh_checksum();
        assert!(matches!(relabeled.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(CompiledCardArtifact::from_json(&relabeled.to_json().unwrap()).is_err());
    }

    #[test]
    fn current_format_with_previous_schema_is_rejected_even_with_a_fresh_checksum() {
        let mut artifact = fixture();
        artifact.engine_schema_hash =
            "f4872928326e3ea14e25666b90d7eb4ce9add2d18c628c2c1936c96f100e4d96".to_string();
        artifact.refresh_checksum();
        assert!(matches!(artifact.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).is_err());
    }

    #[test]
    fn staged_numeric_counter_schema_without_token_roles_is_rejected() {
        let mut artifact = fixture();
        artifact.engine_schema_hash =
            "3f9f096868c6ec7f6437ea2d249befbab23ffefe8604470d4ea4b0018f6c2b08".to_string();
        artifact.refresh_checksum();
        assert_eq!(artifact.format_version, FORMAT_VERSION);
        assert!(matches!(artifact.validate(), Err(ArtifactValidationError::EngineSchemaMismatch { .. })));
        assert!(CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).is_err());
    }

    #[test]
    fn round_trip_validates_checksum() {
        let bytes = fixture().to_json().unwrap();
        assert_eq!(CompiledCardArtifact::from_json(&bytes).unwrap(), fixture());
    }

    #[test]
    fn tampering_is_rejected() {
        let mut artifact = fixture();
        artifact.payload.canonical_text.push('!');
        assert!(matches!(
            artifact.validate(),
            Err(ArtifactValidationError::ChecksumMismatch { .. })
        ));
    }
}
