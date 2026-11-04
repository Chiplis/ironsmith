// native-test-suite: helper-only
//! Strict source-scoped admission, followed by independent native and wire routes.
use ironsmith::CardDefinition;
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
use ironsmith_core::custom_subtypes::CustomSubtypeDefinition;
use serde_json::Value;
type Model = ironsmith_compiler::cards::CardDefinition;
type Builder = ironsmith_compiler::CardDefinitionBuilder;
type Id = ironsmith_compiler::ids::CardId;

fn validate_arrays(source: &Value) -> Result<(), String> {
    if !source.is_object() {
        return Err("source input must be an object".into());
    }
    for field in [
        "custom_subtypes",
        "supplied_card_definitions",
        "custom_keyword_identities",
        "custom_keyword_payments",
        "custom_keyword_counter_definitions",
        "custom_token_definitions",
        "custom_tokens",
    ] {
        if source
            .get(field)
            .is_some_and(|v| !v.is_null() && !v.is_array())
        {
            return Err(format!("invalid source metadata array {field}"));
        }
    }
    if source
        .get("has_fuse")
        .is_some_and(|v| !v.is_null() && !v.is_boolean())
    {
        return Err("source fuse flag must be boolean".into());
    }
    if source
        .get("token")
        .is_some_and(|v| !v.is_null() && !v.is_boolean())
    {
        return Err("source token flag must be boolean".into());
    }
    Ok(())
}
fn strings<'a>(source: &'a Value, field: &str) -> Result<Vec<&'a str>, String> {
    source[field]
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| {
            v.as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| format!("invalid source {field} entry"))
        })
        .collect()
}
fn strict_compile(builder: Builder, text: &str) -> Result<Model, String> {
    eprintln!("SOURCE_TRACE strict_compile BEGIN text={text:?}");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                builder,
                text.to_string(),
                ironsmith_compiler::CompilePolicy {
                    allow_unsupported: false,
                },
            )
        })
    });
    if loss.is_lossy() {
        return Err(loss.reasons_text());
    }
    result
        .map(|result| result.definition)
        .map_err(|error| format!("{error:?}"))
}
fn counter_programs(source: &Value, model: &mut Model) -> Result<(), String> {
    for input in source["custom_keyword_counter_definitions"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let name = input["name"]
            .as_str()
            .ok_or("missing ability-counter name")?;
        let reminder = input["reminder"]
            .as_str()
            .ok_or("missing ability-counter program")?;
        let key = ironsmith_core::counter::normalize_named_counter_name(name)?;
        if key.is_empty() {
            return Err("empty ability-counter name".into());
        }
        // Counter programs share the already admitted source vocabulary/token
        // scope, exactly as the production source-counter adapter does.
        let program = strict_compile(
            Builder::new(Id::new(), "Ability counter program").with_custom_keyword(key.clone()),
            &format!("Type: Creature\nPower/Toughness: 1/1\n{reminder}"),
        )?;
        if program.spell_effect.is_some()
            || program.abilities.iter().all(|ability| {
                matches!(&ability.kind, ironsmith_compiler::ability::AbilityKind::Static(value)
                if matches!(&value.payload, ironsmith_core::StaticAbilityPayload::CustomKeywordIdentity { .. }))
            })
        {
            return Err(format!(
                "ability counter {name:?} must define permanent abilities"
            ));
        }
        for subtype in &program.card.custom_subtypes {
            ironsmith_core::custom_subtypes::merge_subtype_associations(
                &mut model.card.custom_subtypes,
                *subtype,
            );
        }
        if let Some(previous) = model.ability_counter_definitions.get(&key) {
            if serde_json::to_value(previous).map_err(|e| e.to_string())?
                != serde_json::to_value(&program.abilities).map_err(|e| e.to_string())?
            {
                return Err(format!("conflicting ability-counter definition {name:?}"));
            }
        } else {
            model
                .ability_counter_definitions
                .insert(key, program.abilities);
        }
    }
    Ok(())
}
fn compile_model(source: &Value, builder: Builder) -> Result<Model, String> {
    compile_model_internal(source, builder, false)
}
fn compile_model_internal(source: &Value, mut builder: Builder, graph_active: bool) -> Result<Model, String> {
    validate_arrays(source)?;
    let text = source["executable_text"]
        .as_str()
        .ok_or("unread source face")?;
    let vocabulary: Vec<CustomSubtypeDefinition> = serde_json::from_value(
        source
            .get("custom_subtypes")
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or(serde_json::json!([])),
    )
    .map_err(|e| e.to_string())?;
    ironsmith_compiler::with_custom_subtypes(&vocabulary, || {
        if source["token"].as_bool() == Some(true) {
            builder = builder.token();
        }
        if source["has_fuse"].as_bool() == Some(true) {
            builder = builder.has_fuse();
        }
        for identity in strings(source, "custom_keyword_identities")? {
            builder = builder.with_custom_keyword(identity);
        }
        let supplied=if graph_active {Vec::new()} else {source.get("supplied_card_definitions").and_then(Value::as_array).into_iter().flatten().map(|definition|{
            ironsmith_compiled_artifact::validate_standalone_supplied_definition_metadata(definition["text"].as_str().unwrap_or(""),definition.as_object().ok_or("supplied definition must be an object")?)?;
            let subtypes:Vec<CustomSubtypeDefinition>=serde_json::from_value(definition.get("customSubtypes").or_else(||definition.get("custom_subtypes")).cloned().unwrap_or(serde_json::json!([]))).map_err(|error|error.to_string())?;
            let names=|key:&str|definition.get(key).or_else(||definition.get(match key {"customKeywordIdentities"=>"custom_keyword_identities","customKeywordPayments"=>"custom_keyword_payments",_=>key})).and_then(Value::as_array).map(|array|array.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()).unwrap_or_default();
            ironsmith_compiler::compile_supplied_card_definition(definition["name"].as_str().unwrap_or(""),definition["text"].as_str().unwrap_or(""),&names("customKeywordIdentities"),&names("customKeywordPayments"),&subtypes)
        }).collect::<Result<Vec<_>,String>>()?};
        let mut model=if graph_active {strict_compile(builder,text)?} else {ironsmith_compiler::with_supplied_card_definitions(supplied,||strict_compile(builder,text))?};
        for payment in strings(source, "custom_keyword_payments")? {
            if !model.attribute_custom_keyword_payment(payment) {
                return Err(format!(
                    "source payment {payment:?} has no unique attributable price"
                ));
            }
        }
        counter_programs(source, &mut model)?;
        Ok(model)
    })
    .map_err(|e| format!("{e:?}"))?
}
type CreationGraph = Option<(Id, Vec<Model>)>;
fn compile_model_for_routes(source: &Value, builder: Builder) -> Result<(Model, CreationGraph), String> {
    let Some(inputs) = source["supplied_card_definitions"].as_array().filter(|inputs| !inputs.is_empty()) else {
        return compile_model(source, builder).map(|model| (model, None));
    };
    let inputs = inputs.iter().map(|input| {
        ironsmith_compiled_artifact::validate_standalone_supplied_definition_metadata(input["text"].as_str().unwrap_or(""),input.as_object().ok_or("supplied definition must be an object")?)?;
        let names = |key: &str| input.get(key).or_else(||input.get(match key {"customKeywordIdentities"=>"custom_keyword_identities","customKeywordPayments"=>"custom_keyword_payments",_=>key})).and_then(Value::as_array).map(|array| array.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default();
        Ok(ironsmith_compiler::SuppliedCardDefinitionSource {
            name: input["name"].as_str().ok_or("supplied graph name missing")?.into(),
            text: input["text"].as_str().ok_or("supplied graph text missing")?.into(),
            identities: names("customKeywordIdentities"), payments: names("customKeywordPayments"),
            subtypes: serde_json::from_value(input.get("customSubtypes").or_else(||input.get("custom_subtypes")).cloned().unwrap_or(serde_json::json!([]))).map_err(|error| error.to_string())?,
        })
    }).collect::<Result<Vec<_>, String>>()?;
    let declared = builder.clone().card_builder.build();
    let bundle = ironsmith_compiler::compile_supplied_card_definition_bundle(
        declared.id, &declared.name, source["executable_text"].as_str().ok_or("unread root source")?, source["token"].as_bool() != Some(true), &inputs,
        || compile_model_internal(source, builder, true),
    )?;
    Ok((bundle.root, Some((bundle.scope, bundle.catalog))))
}
type TokenCatalog = Vec<(Value, Model)>;
/// Original supplied inputs declare dependencies independently from rendered
/// creation text. A missing blueprint must not become a named blank token.
fn require_expected_tokens(source:&Value,catalog:&TokenCatalog)->Result<(),String> {
    let mut seen=std::collections::BTreeSet::new();
    for dependency in source["custom_tokens"].as_array().into_iter().flatten() {
        let input=dependency.get("input").filter(|value|value.is_object())
            .ok_or("expected supplied token dependency must contain an input object")?;
        let name=input["name"].as_str().filter(|name|!name.trim().is_empty())
            .ok_or("expected supplied token dependency must have a nonempty input name")?;
        let key=name.trim().to_ascii_lowercase();
        if !seen.insert(key) {return Err(format!("duplicate expected supplied token {name:?}"));}
        if let Some(face)=dependency.get("face") {
            let face_name=face["name"].as_str().filter(|name|!name.trim().is_empty())
                .ok_or("expected supplied token face must have a nonempty name")?;
            if !face_name.trim().eq_ignore_ascii_case(name.trim()) {
                return Err("expected supplied token input/face names disagree".into());
            }
        }
        if !catalog.iter().any(|(_,model)|model.card.name.trim().eq_ignore_ascii_case(name.trim())) {
            return Err(format!("missing supplied token definition for expected dependency {name:?}"));
        }
    }
    Ok(())
}
fn token_scope<T>(
    source: &Value,
    inherited: &TokenCatalog,
    action: impl FnOnce(TokenCatalog) -> Result<T, String>,
) -> Result<T, String> {
    validate_arrays(source)?;
    let mut catalog = inherited.clone();
    let mut local_names = std::collections::BTreeSet::new();
    for token in source["custom_token_definitions"]
        .as_array()
        .into_iter()
        .flatten()
    {
        validate_arrays(token)?;
        if token["token"].as_bool() != Some(true) {
            return Err("supplied token must be explicitly marked as a token".into());
        }
        if token["custom_token_definitions"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
        {
            return Err("supplied token dependencies must be flattened in dependency order".into());
        }
        if token["executable_faces"]
            .as_array()
            .is_some_and(|faces| faces.len() > 1)
        {
            return Err(
                "supplied token catalog entries must be individual compiled source faces".into(),
            );
        }
        let name = token["name"]
            .as_str()
            .ok_or("missing supplied token name")?;
        let key = name.trim().to_ascii_lowercase();
        if !local_names.insert(key.clone()) {
            return Err("duplicate supplied token definition".into());
        }
        if let Some((prior, _)) = catalog
            .iter()
            .find(|(_, model)| model.card.name.trim().eq_ignore_ascii_case(name.trim()))
        {
            if prior != token {
                return Err(format!("conflicting inherited token definition {name:?}"));
            }
            continue;
        }
        // Explicit token-to-token dependencies must already be compiled in
        // this flattened catalog; neither self nor forward references qualify.
        require_expected_tokens(token,&catalog)?;
        let previous = catalog.iter().map(|(_, model)| model.clone()).collect();
        let model = ironsmith_compiler::with_custom_tokens(previous, || {
            compile_model(token, Builder::new(Id::new(), name))
        })?;
        if model
            .card
            .mana_cost
            .as_ref()
            .is_some_and(|cost| !cost.is_empty())
        {
            return Err("supplied token cannot have a mana cost".into());
        }
        catalog.push((token.clone(), model));
    }
    require_expected_tokens(source,&catalog)?;
    let models = catalog.iter().map(|(_, model)| model.clone()).collect();
    ironsmith_compiler::with_custom_tokens(models, || action(catalog))
}
fn routes(model: Model, text: &str, linked: bool, graph: CreationGraph) -> Result<[CardDefinition; 2], String> {
    eprintln!("SOURCE_TRACE routes BEGIN name={:?}", model.card.name);
    // Native interpretation never obtains its result from artifact materialization.
    let direct = if let Some((scope, catalog)) = &graph {
        ironsmith_compiler_runtime::into_runtime_definition_bundle(ironsmith_compiler::SuppliedCardDefinitionBundle {
            root: model.clone(), scope: *scope, catalog: catalog.clone(),
        })
    } else { ironsmith_compiler_runtime::into_runtime_definition(model.clone()) }.map_err(|error| format!("{error:?}"))?;
    eprintln!("SOURCE_TRACE native DONE name={:?}", direct.card.name);
    let wire = wire_definition_from_serializable(&model).map_err(|e| e.to_string())?;
    let wire = if let Some((scope, catalog)) = graph {
        let catalog = catalog.iter().map(wire_definition_from_serializable).collect::<Result<Vec<_>, _>>().map_err(|error|error.to_string())?;
        ironsmith_runtime_catalog::artifact_materializer::bind_card_creation_scope(wire, scope, &catalog)?
    } else { wire };
    let artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(direct.card.id.0),
            name: direct.card.name.clone(),
            face_name: linked.then(|| direct.card.name.clone()),
            other_face: direct.card.other_face.map(|id| ArtifactCardId(id.0)),
            linked_face_layout: linked.then(|| format!("{:?}", direct.card.linked_face_layout)),
        },
        CompiledCardPayload {
            definition: wire,
            canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"),
            ability_labels: ironsmith_text::ability_surface_texts(&direct),
        },
        "complete-source-next-wave",
        text.as_bytes(),
    );
    eprintln!("SOURCE_TRACE artifact BUILT name={:?}", direct.card.name);
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if artifact != restored {
        return Err("artifact roundtrip changed source program".into());
    }
    let materialized =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
            .map_err(|e| format!("{e:?}"))?;
    eprintln!("SOURCE_TRACE routes DONE name={:?}", direct.card.name);
    Ok([direct, materialized])
}
/// Standalone input. A supplied physical multiface card must use linked_definitions.
pub fn definitions(source: &Value) -> Result<[CardDefinition; 2], String> {
    if source["executable_faces"]
        .as_array()
        .is_some_and(|faces| faces.len() == 1)
    {
        return linked_definitions(source)?
            .pop()
            .ok_or("missing physical source face".into());
    }
    if source["executable_faces"]
        .as_array()
        .is_some_and(|faces| faces.len() > 1)
    {
        return Err(
            "physical linked source requires linked_definitions; sibling faces cannot be discarded"
                .into(),
        );
    }
    token_scope(source, &Vec::new(), |_catalog| {
        let name = source["name"].as_str().ok_or("missing source name")?;
        let (model, graph) = compile_model_for_routes(source, Builder::new(Id::new(), name))?;
        routes(
            model,
            source["executable_text"]
                .as_str()
                .ok_or("unread source face")?,
            false, graph,
        )
    })
}
/// Every complete physical face is returned on both routes, sharing the exact
/// linked IDs and layout. Callers can register all returned siblings in a game.
pub fn linked_definitions(card: &Value) -> Result<Vec<[CardDefinition; 2]>, String> {
    eprintln!("SOURCE_TRACE linked BEGIN key={:?} name={:?}", card["source_card_key"], card["name"]);
    let Some(faces) = card["executable_faces"].as_array() else {
        return definitions(card).map(|pair| vec![pair]);
    };
    if faces.is_empty() {
        return Err("physical source has no executable faces".into());
    }
    if let Some(originals) = card["source_faces"].as_array() {
        if originals.len() != faces.len() {
            return Err("linked source face count changed".into());
        }
        for (original, face) in originals.iter().zip(faces) {
            if original["name"] != face["name"] {
                return Err("linked source face order/name changed".into());
            }
        }
    }
    if faces.len() > 2 {
        return Err("native linked source requires one or two faces".into());
    }
    let layout = card["source_layout"]
        .as_str()
        .or_else(|| card["source_shape"].as_str())
        .unwrap_or("normal");
    use ironsmith_compiler::card::LinkedFaceLayout as L;
    let (kind, default_transforming) = match layout {
        "normal" | "single" if faces.len() == 1 => (L::None, false),
        "doubleface" | "transform" => (L::TransformLike, true),
        "modal_dfc" | "transform_like" | "transformLike" => (L::TransformLike, false),
        "split" | "aftermath" => (L::Split, false),
        "flip" => (L::Flip, false),
        "prepare" => (L::Prepare, false),
        "adventure" => (L::None, false),
        _ => return Err(format!("unknown or incomplete source layout {layout:?}")),
    };
    if card
        .get("transforming_dfc")
        .is_some_and(|value| !value.is_null() && !value.is_boolean())
    {
        return Err("invalid transforming_dfc metadata".into());
    }
    let transforming = card["transforming_dfc"]
        .as_bool()
        .unwrap_or(default_transforming);
    if transforming && kind != L::TransformLike {
        return Err("transforming flag requires a double-faced layout".into());
    }
    if faces.len() == 1 && kind != L::None {
        return Err("linked layout is missing its sibling face".into());
    }
    let ids: Vec<_> = faces.iter().map(|_| Id::new()).collect();
    token_scope(card, &Vec::new(), |catalog| {
        faces
            .iter()
            .enumerate()
            .map(|(index, face)| {
                let mut input = face.clone();
                let mut inherited = vec![
                    "custom_subtypes",
                    "supplied_card_definitions",
                    "custom_token_definitions",
                    "custom_keyword_counter_definitions",
                    "token",
                ];
                if faces.len() == 1 {
                    inherited.extend(["custom_keyword_identities", "custom_keyword_payments"]);
                }
                for field in inherited {
                    if input.get(field).is_none_or(Value::is_null) {
                        if let Some(value) = card.get(field) {
                            input[field] = value.clone();
                        }
                    }
                }
                let name = input["name"].as_str().ok_or("missing linked source name")?;
                let mut builder = Builder::new(ids[index], name);
                if card["has_fuse"].as_bool() == Some(true) {
                    builder = builder.has_fuse();
                }
                if faces.len() == 2 {
                    let other = 1 - index;
                    builder = builder
                        .other_face(ids[other])
                        .other_face_name(
                            faces[other]["name"]
                                .as_str()
                                .ok_or("missing sibling name")?,
                        )
                        .linked_face_layout(kind)
                        .transforming_dfc(transforming);
                }
                token_scope(&input, &catalog, |_catalog| {
                    let (model, graph) = compile_model_for_routes(&input, builder)?;
                    routes(
                        model,
                        input["executable_text"]
                            .as_str()
                            .ok_or("unread linked source face")?,
                        faces.len() == 2, graph,
                    )
                })
            })
            .collect()
    })
}
