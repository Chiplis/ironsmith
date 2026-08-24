//! Stable, deterministic boundary between card compilation and engine loading.

use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const FORMAT_VERSION: u32 = 3;
pub const ENGINE_SCHEMA_HASH: &str =
    "1bd9697f2fe94a47ca56213313de037949dfc7e3df8ea5db33e92d35b82fa99a";

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
    fn golden_json_is_stable() {
        let actual = String::from_utf8(fixture().to_json().unwrap()).unwrap();
        assert_eq!(actual.trim(), include_str!("../fixtures/v3.json").trim());
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
