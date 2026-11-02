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
struct CustomKeywordCounterDefinition { name: String, reminder: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
struct SuppliedCardDefinitionInput {
    #[serde(flatten)] metadata:serde_json::Map<String,serde_json::Value>,
    name:String, text:String,
    #[serde(default,alias="custom_keyword_identities")] custom_keyword_identities:Vec<String>,
    #[serde(default,alias="custom_keyword_payments")] custom_keyword_payments:Vec<String>,
    #[serde(default,alias="custom_subtypes")] custom_subtypes:Vec<ironsmith_compiler::CustomSubtypeDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompileCardInput {
    #[serde(default)]
    supplied_card_definitions:Vec<SuppliedCardDefinitionInput>,
    name: String,
    text: String,
    #[serde(default)]
    custom_keyword_identities: Vec<String>,
    /// Custom keywords whose definition created this card's single generic
    /// optional or alternative payment ("a spell using <keyword>").
    #[serde(default)]
    custom_keyword_payments: Vec<String>,
    #[serde(default)]
    custom_subtypes: Vec<ironsmith_compiler::CustomSubtypeDefinition>,
    #[serde(default)]
    custom_keyword_counter_definitions: Vec<CustomKeywordCounterDefinition>,
    #[serde(default)]
    allow_unsupported: bool,
    #[serde(default = "default_enforcement")]
    enforce_filter_claims: bool,
    #[serde(default = "default_enforcement")]
    enforce_line_claims: bool,
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

fn default_enforcement() -> bool {
    true
}

fn default_local_id() -> u32 {
    1
}

fn compile_artifact(input: CompileCardInput) -> Result<CompiledCardArtifact, String> {
    let reserved = input.local_id.max(1).max(input.other_face_id.unwrap_or(0));
    CardId::reserve_generated_namespace_through(CardId::from_raw(reserved)).map_err(str::to_owned)?;
    let vocabulary = input.custom_subtypes.clone();
    ironsmith_compiler::with_custom_subtypes(&vocabulary, || {
        if input.supplied_card_definitions.is_empty() {
            let definition = compile_definition_in_scope(&input)?;
            return encode_artifact(input, definition, None);
        }
        let sources = input.supplied_card_definitions.iter().map(|source| {
            ironsmith_compiled_artifact::validate_standalone_supplied_definition_metadata(&source.text,&source.metadata)?;
            Ok(ironsmith_compiler::SuppliedCardDefinitionSource {
            name: source.name.clone(), text: source.text.clone(),
            identities: source.custom_keyword_identities.clone(), payments: source.custom_keyword_payments.clone(),
            subtypes: source.custom_subtypes.clone(),
        })}).collect::<Result<Vec<_>,String>>()?;
        let bundle = ironsmith_compiler::compile_supplied_card_definition_bundle(
            CardId::from_raw(input.local_id.max(1)), &input.name, &input.text, true, &sources,
            || compile_definition_in_scope(&input),
        )?;
        encode_artifact(input, bundle.root, Some((bundle.scope, bundle.catalog)))
    })?
}

fn compile_definition_in_scope(input: &CompileCardInput) -> Result<ironsmith_compiler::cards::CardDefinition, String> {
    let local_id = input.local_id.max(1);
    let mut builder = CardDefinitionBuilder::new(CardId::from_raw(local_id), input.name.clone());
    for identity in &input.custom_keyword_identities {
        builder = builder.with_custom_keyword(identity.clone());
    }
    let mut compiled = ironsmith_compiler::word_claims::with_enforcement_options(
        input.enforce_filter_claims,
        input.enforce_line_claims,
        || {
            CompilerFacade::new().compile_definition(
                builder,
                input.text.clone(),
                CompilePolicy {
                    allow_unsupported: input.allow_unsupported,
                },
            )
        },
    )
    .map_err(|error| error.to_string())?;
    for keyword in &input.custom_keyword_payments {
        compiled.definition.attribute_custom_keyword_payment(keyword);
    }
    for counter in &input.custom_keyword_counter_definitions {
        let kind = ironsmith_core::counter::normalize_named_counter_name(&counter.name)?;
        if counter.name.trim().is_empty() { return Err("ability-counter name is empty".into()); }
        let program = ironsmith_compiler::word_claims::with_enforcement_options(
            input.enforce_filter_claims, input.enforce_line_claims, || {
                CompilerFacade::new().compile_definition(
                    CardDefinitionBuilder::new(CardId::new(), "Ability counter program").with_custom_keyword(kind.clone()),
                    format!("Type: Creature\nPower/Toughness: 1/1\n{}", counter.reminder),
                    CompilePolicy { allow_unsupported: false },
                )
            },
        ).map_err(|error| format!("ability counter {}: {error}",counter.name))?;
        if program.definition.abilities.iter().all(|ability|matches!(&ability.kind,ironsmith_compiler::ability::AbilityKind::Static(value) if matches!(&value.payload, ironsmith_core::StaticAbilityPayload::CustomKeywordIdentity { .. }))) || program.definition.spell_effect.is_some() {
            return Err(format!("ability counter {} must define permanent abilities",counter.name));
        }
        for subtype in &program.definition.card.custom_subtypes {
            ironsmith_core::custom_subtypes::merge_subtype_associations(&mut compiled.definition.card.custom_subtypes, *subtype);
        }
        if let Some(previous) = compiled.definition.ability_counter_definitions.insert(kind, program.definition.abilities.clone()) {
            if serde_json::to_value(&previous).map_err(|error|error.to_string())? != serde_json::to_value(&program.definition.abilities).map_err(|error|error.to_string())? { return Err(format!("conflicting ability counter {}",counter.name)); }
        }
    }
    compiled.definition.card.id = CardId::from_raw(local_id);
    compiled.definition.card.other_face = input.other_face_id.map(CardId::from_raw);
    compiled.definition.card.other_face_name = input.other_face_name.clone();
    compiled.definition.card.linked_face_layout = match input.linked_face_layout.as_deref() {
        Some("split") => LinkedFaceLayout::Split,
        Some("prepare") => LinkedFaceLayout::Prepare,
        Some("flip") => LinkedFaceLayout::Flip,
        Some("transform") | Some("transform_like") | Some("transformLike") => {
            LinkedFaceLayout::TransformLike
        }
        _ => LinkedFaceLayout::None,
    };
    // Scryfall `transform` is a transforming DFC (CR 712.1), not a modal one.
    compiled.definition.card.transforming_dfc = input.linked_face_layout.as_deref() == Some("transform");
    Ok(compiled.definition)
}

fn encode_artifact(
    input: CompileCardInput,
    definition: ironsmith_compiler::cards::CardDefinition,
    graph: Option<(CardId, Vec<ironsmith_compiler::cards::CardDefinition>)>,
) -> Result<CompiledCardArtifact, String> {
    let local_id = input.local_id.max(1);
    let wire_definition = wire_definition_from_serializable(&definition)
        .map_err(|error| format!("failed to encode compiled definition: {error}"))?;
    let wire_definition = if let Some((scope, catalog)) = graph {
        let catalog = catalog.iter().map(wire_definition_from_serializable)
            .collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())?;
        ironsmith_runtime_catalog::artifact_materializer::bind_card_creation_scope(wire_definition, scope, &catalog)?
    } else { wire_definition };
    let wire_definition = ironsmith_text::with_choice_decision_labels(wire_definition, |wire|
        ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire)
            .map_err(|error| error.to_string()))?;
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
            name: definition.card.name.clone(),
            face_name: None,
            other_face: input.other_face_id.map(ArtifactCardId),
            linked_face_layout: Some(format!("{:?}", definition.card.linked_face_layout)),
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
    artifact.compiler_facts.insert(
        "enforceFilterClaims".into(),
        input.enforce_filter_claims.to_string(),
    );
    artifact.compiler_facts.insert(
        "enforceLineClaims".into(),
        input.enforce_line_claims.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_defaults_on_and_filter_can_be_disabled() {
        let input = |filter: Option<bool>, line: Option<bool>| {
            let mut value = serde_json::json!({
                "name": "Gilded Banner",
                "text": "Type: Enchantment\nBlorp creatures you control get +1/+1."
            });
            if let Some(filter) = filter {
                value["enforceFilterClaims"] = filter.into();
            }
            if let Some(line) = line {
                value["enforceLineClaims"] = line.into();
            }
            serde_json::from_value::<CompileCardInput>(value).unwrap()
        };
        let defaults = input(None, None);
        assert!(defaults.enforce_filter_claims && defaults.enforce_line_claims);
        assert!(
            compile_artifact(input(Some(true), Some(false)))
                .unwrap_err()
                .contains("does not read 'blorp'")
        );
        assert!(
            compile_artifact(input(Some(false), Some(true)))
                .unwrap_err()
                .contains("no reader reads")
        );
        let artifact = compile_artifact(input(Some(false), Some(false))).unwrap();
        assert_eq!(artifact.compiler_facts["enforceFilterClaims"], "false");
        assert_eq!(artifact.compiler_facts["enforceLineClaims"], "false");
    }
}

#[cfg(test)]
mod supplied_namespace_admission_tests {
    use super::*;
    #[test]
    fn exhausted_explicit_namespace_fails_before_supplied_source_compilation() {
        let input:CompileCardInput=serde_json::from_value(serde_json::json!({
            "name":"Root", "text":"Type: Sorcery\nDraw a card.", "localId":u32::MAX,
            "suppliedCardDefinitions":[{"name":"Broken dependency","text":"Not a card program"}],
        })).unwrap();
        assert!(compile_artifact(input).unwrap_err().contains("namespace"));
    }
    #[test]
    fn explicit_ids_cannot_truncate_from_larger_json_integers() {
        assert!(serde_json::from_value::<CompileCardInput>(serde_json::json!({
            "name":"Root","text":"Type: Sorcery\nDraw a card.","otherFaceId":u64::MAX,
        })).is_err());
    }
}

#[cfg(test)]
mod supplied_dependency_metadata_tests {
    use super::*;
    #[test]
    fn native_request_cannot_bypass_frontend_dependency_admission() {
        for (field,value) in [("token",serde_json::json!(true)),
            ("customKeywordCounterDefinitions",serde_json::json!([{"name":"gifted","reminder":"Flying"}])),
            ("customTokens",serde_json::json!([{"name":"Helper"}])),
            ("otherFaceId",serde_json::json!(2))] {
            let mut supplied=serde_json::json!({"name":"Design","text":"Type: Sorcery\nDraw a card."});
            supplied[field]=value;
            let input=serde_json::from_value(serde_json::json!({"name":"Maker","text":"Type: Sorcery\nCreate a real card named \"Design\" in your hand.","suppliedCardDefinitions":[supplied]})).unwrap();
            assert!(compile_artifact(input).unwrap_err().contains(field));
        }
    }
}
