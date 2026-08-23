use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
use ironsmith_compiler::card::LinkedFaceLayout;
use ironsmith_compiler::{CardDefinitionBuilder, CompilePolicy, CompilerFacade, ids::CardId};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompileCardInput {
    name: String,
    text: String,
    #[serde(default)]
    allow_unsupported: bool,
    #[serde(default = "default_local_id")]
    local_id: u32,
    #[serde(default)]
    other_face_id: Option<u32>,
    #[serde(default)]
    other_face_name: Option<String>,
    #[serde(default)]
    linked_face_layout: Option<String>,
    #[serde(default)]
    semantic_score: Option<f32>,
}

fn default_local_id() -> u32 {
    1
}

fn compile_artifact(input: CompileCardInput) -> Result<CompiledCardArtifact, String> {
    let local_id = input.local_id.max(1);
    let builder = CardDefinitionBuilder::new(CardId::from_raw(local_id), input.name.clone());
    let mut compiled = CompilerFacade::new()
        .compile_definition(
            builder,
            input.text.clone(),
            CompilePolicy {
                allow_unsupported: input.allow_unsupported,
            },
        )
        .map_err(|error| error.to_string())?;
    compiled.definition.card.id = CardId::from_raw(local_id);
    compiled.definition.card.other_face = input.other_face_id.map(CardId::from_raw);
    compiled.definition.card.other_face_name = input.other_face_name.clone();
    compiled.definition.card.linked_face_layout = match input.linked_face_layout.as_deref() {
        Some("split") => LinkedFaceLayout::Split,
        Some("transform") | Some("transform_like") | Some("transformLike") => {
            LinkedFaceLayout::TransformLike
        }
        _ => LinkedFaceLayout::None,
    };
    let wire_definition = wire_definition_from_serializable(&compiled.definition)
        .map_err(|error| format!("failed to encode compiled definition: {error}"))?;
    let runtime_definition =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(
            wire_definition.clone(),
        )
        .map_err(|error| format!("failed to materialize compiled definition: {error}"))?;
    let canonical_text = ironsmith_text::compiled_text_lines(&runtime_definition).join("\n");
    let ability_labels = ironsmith_text::ability_surface_texts(&runtime_definition);

    let mut artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(local_id),
            name: compiled.definition.card.name.clone(),
            face_name: None,
            other_face: input.other_face_id.map(ArtifactCardId),
            linked_face_layout: Some(format!("{:?}", compiled.definition.card.linked_face_layout)),
        },
        CompiledCardPayload {
            definition: wire_definition,
            canonical_text,
            ability_labels,
        },
        concat!("ironsmith-compiler/", env!("CARGO_PKG_VERSION")),
        input.text.as_bytes(),
    );
    artifact.compiler_facts.insert(
        "allowUnsupported".to_string(),
        input.allow_unsupported.to_string(),
    );
    artifact.semantic_score = input.semantic_score.map(|score| score.clamp(0.0, 1.0));
    artifact.refresh_checksum();
    Ok(artifact)
}

#[wasm_bindgen(js_name = compileCardArtifact)]
pub fn compile_card_artifact(input: JsValue) -> Result<JsValue, JsValue> {
    let input: CompileCardInput = serde_wasm_bindgen::from_value(input)
        .map_err(|error| JsValue::from_str(&format!("invalid compiler input: {error}")))?;
    let artifact = compile_artifact(input).map_err(|error| JsValue::from_str(&error))?;
    serde_wasm_bindgen::to_value(&artifact)
        .map_err(|error| JsValue::from_str(&format!("failed to encode artifact: {error}")))
}

#[wasm_bindgen(js_name = validateCompiledCardArtifact)]
pub fn validate_compiled_card_artifact(input: JsValue) -> Result<(), JsValue> {
    let artifact: CompiledCardArtifact = serde_wasm_bindgen::from_value(input)
        .map_err(|error| JsValue::from_str(&format!("invalid compiled artifact: {error}")))?;
    artifact
        .validate()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
