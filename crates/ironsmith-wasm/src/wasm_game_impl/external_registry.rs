fn external_card_lookup_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn external_card_score(score: Option<f32>) -> Option<f32> {
    score.map(|value| value.clamp(0.0, 1.0))
}

impl WasmGame {
    fn materialize_compiled_artifact_batch(
        artifacts: &[CompiledCardArtifact],
    ) -> Result<Vec<CardDefinition>, String> {
        let mut runtime_ids = HashMap::with_capacity(artifacts.len());
        for artifact in artifacts {
            artifact.validate().map_err(|error| error.to_string())?;
            if runtime_ids
                .insert(artifact.card.local_id, CardId::new())
                .is_some()
            {
                return Err(format!(
                    "duplicate artifact-local card id {}",
                    artifact.card.local_id.0
                ));
            }
        }

        let mut definitions = Vec::with_capacity(artifacts.len());
        for artifact in artifacts {
            let mut definition =
                ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(artifact)
                    .map_err(|error| error.to_string())?;
            if !artifact
                .card
                .name
                .eq_ignore_ascii_case(&definition.card.name)
            {
                return Err(format!(
                    "artifact name {:?} does not match definition name {:?}",
                    artifact.card.name, definition.card.name
                ));
            }
            definition.card.id = runtime_ids[&artifact.card.local_id];
            definition.card.other_face = artifact
                .card
                .other_face
                .map(|local_id| {
                    runtime_ids.get(&local_id).copied().ok_or_else(|| {
                        format!(
                            "artifact {} references missing local face id {}",
                            artifact.card.name, local_id.0
                        )
                    })
                })
                .transpose()?;
            if let Some(error) =
                ironsmith::cards::unsupported_generated_definition_error(&definition)
            {
                return Err(error);
            }
            definitions.push(definition);
        }
        Ok(definitions)
    }

    fn external_source_definition_names(source: &ExternalCardSourceFile) -> Vec<&str> {
        match &source.group {
            ExternalCardSourceGroup::Single { name, .. } => vec![name.as_str()],
            ExternalCardSourceGroup::Linked { faces, .. } => {
                faces.iter().map(|face| face.name.as_str()).collect()
            }
        }
    }

    fn register_resolvable_external_aliases(&mut self, aliases: &[ExternalCardAliasSource]) {
        for alias in aliases {
            if self.registry.get(&alias.canonical).is_some() {
                self.registry
                    .register_alias(alias.alias.clone(), alias.canonical.clone());
            }
        }
    }

    fn remember_external_parse_source(&mut self, name: &str, source_name: &str, block: &str) {
        let key = external_card_lookup_key(name);
        if key.is_empty() {
            return;
        }
        self.external_parse_sources
            .insert(key, (source_name.to_string(), block.to_string()));
    }

    fn remember_external_score(&mut self, name: &str, score: Option<f32>) {
        let Some(score) = external_card_score(score) else {
            return;
        };
        let key = external_card_lookup_key(name);
        if key.is_empty() {
            return;
        }
        self.external_semantic_scores
            .entry(key)
            .and_modify(|existing| *existing = (*existing).max(score))
            .or_insert(score);
    }

    fn remember_external_error(&mut self, name: &str, error: &str) {
        let key = external_card_lookup_key(name);
        if key.is_empty() {
            return;
        }
        self.external_compile_errors
            .insert(key, error.trim().to_string());
    }

    fn clear_external_error(&mut self, name: &str) {
        let key = external_card_lookup_key(name);
        if !key.is_empty() {
            self.external_compile_errors.remove(&key);
        }
    }

    fn external_compile_error_for_name(&self, name: &str) -> Option<String> {
        let key = external_card_lookup_key(name);
        if key.is_empty() {
            return None;
        }
        self.external_compile_errors.get(&key).cloned()
    }

    fn external_parse_source_for_name(&self, name: &str) -> Option<(String, String)> {
        let key = external_card_lookup_key(name);
        if key.is_empty() {
            return None;
        }
        self.external_parse_sources.get(&key).cloned()
    }

    fn external_semantic_score_for_name(&self, name: &str) -> Option<f32> {
        let key = external_card_lookup_key(name);
        if key.is_empty() {
            return None;
        }
        self.external_semantic_scores.get(&key).copied()
    }

    fn compile_external_single_card(
        &self,
        name: &str,
        block: &str,
    ) -> Result<CardDefinition, String> {
        let definition = Self::compile_definition_from_parse_source(name, block)?;
        if let Some(error) = ironsmith::cards::unsupported_generated_definition_error(&definition) {
            return Err(error);
        }
        Ok(definition)
    }

    #[cfg(feature = "dynamic-compile")]
    fn compile_external_linked_group(
        &self,
        layout: &str,
        faces: &[ExternalCardFaceSource],
        has_fuse: bool,
    ) -> Result<Vec<CardDefinition>, String> {
        if faces.len() < 2 {
            return Err("linked card source requires at least two faces".to_string());
        }

        let front = &faces[0];
        let back = &faces[1];
        let linked_layout = match layout {
            "split" => ironsmith::card::LinkedFaceLayout::Split,
            _ => ironsmith::card::LinkedFaceLayout::TransformLike,
        };
        let front_id = CardId::new();
        let back_id = CardId::new();
        let front_builder = ironsmith_dynamic_compile::CompilerCardDefinitionBuilder::new(front_id, &front.name);
        let back_builder = ironsmith_dynamic_compile::CompilerCardDefinitionBuilder::new(back_id, &back.name);
        let mut front_definition = ironsmith_dynamic_compile::compile_builder_to_runtime_definition(
            front_builder,
            front.block.clone(),
            false,
        )
        .map_err(|err| format!("front face: {err}"))?;
        let mut back_definition = ironsmith_dynamic_compile::compile_builder_to_runtime_definition(
            back_builder,
            back.block.clone(),
            false,
        )
        .map_err(|err| format!("back face: {err}"))?;

        front_definition.card.other_face = Some(back_id);
        front_definition.card.other_face_name = Some(back.name.clone());
        front_definition.card.linked_face_layout = linked_layout;
        back_definition.card.other_face = Some(front_id);
        back_definition.card.other_face_name = Some(front.name.clone());
        back_definition.card.linked_face_layout = linked_layout;
        if linked_layout == ironsmith::card::LinkedFaceLayout::Split {
            front_definition.has_fuse = has_fuse;
            back_definition.has_fuse = has_fuse;
        }

        if let Some(error) =
            ironsmith::cards::unsupported_generated_definition_error(&front_definition)
        {
            return Err(error);
        }
        if let Some(error) =
            ironsmith::cards::unsupported_generated_definition_error(&back_definition)
        {
            return Err(error);
        }

        Ok(vec![front_definition, back_definition])
    }

    #[cfg(not(feature = "dynamic-compile"))]
    fn compile_external_linked_group(
        &self,
        _layout: &str,
        _faces: &[ExternalCardFaceSource],
        _has_fuse: bool,
    ) -> Result<Vec<CardDefinition>, String> {
        Err("source compilation is provided by ironsmith-compiler-wasm; register compiled artifacts with the lean engine".to_string())
    }

    fn register_external_source_metadata(&mut self, source: &ExternalCardSourceFile) {
        match &source.group {
            ExternalCardSourceGroup::Single { name, block, score } => {
                self.remember_external_parse_source(name, name, block);
                self.remember_external_score(name, *score);
                if !source.canonical_name.trim().is_empty()
                    && !source.canonical_name.eq_ignore_ascii_case(name)
                {
                    self.remember_external_parse_source(&source.canonical_name, name, block);
                    self.remember_external_score(&source.canonical_name, *score);
                }
                for alias in &source.aliases {
                    if alias.canonical.eq_ignore_ascii_case(name) {
                        self.remember_external_parse_source(&alias.alias, name, block);
                        self.remember_external_score(&alias.alias, *score);
                    }
                }
            }
            ExternalCardSourceGroup::Linked {
                combined_name,
                faces,
                ..
            } => {
                for face in faces {
                    self.remember_external_parse_source(&face.name, &face.name, &face.block);
                    self.remember_external_score(&face.name, face.score);
                }
                if let Some(front) = faces.first() {
                    let combined_score = faces.iter().filter_map(|face| face.score).fold(
                        None,
                        |best: Option<f32>, score| {
                            Some(best.map_or(score, |existing| existing.max(score)))
                        },
                    );
                    self.remember_external_parse_source(combined_name, &front.name, &front.block);
                    self.remember_external_score(combined_name, combined_score);
                    if !source.canonical_name.trim().is_empty()
                        && !source.canonical_name.eq_ignore_ascii_case(combined_name)
                    {
                        self.remember_external_parse_source(
                            &source.canonical_name,
                            &front.name,
                            &front.block,
                        );
                        self.remember_external_score(&source.canonical_name, combined_score);
                    }
                }
                for alias in &source.aliases {
                    if let Some(face) = faces
                        .iter()
                        .find(|face| face.name.eq_ignore_ascii_case(&alias.canonical))
                    {
                        self.remember_external_parse_source(&alias.alias, &face.name, &face.block);
                        self.remember_external_score(&alias.alias, face.score);
                    } else if alias.canonical.eq_ignore_ascii_case(combined_name)
                        && let Some(front) = faces.first()
                    {
                        self.remember_external_parse_source(
                            &alias.alias,
                            &front.name,
                            &front.block,
                        );
                        self.remember_external_score(&alias.alias, front.score);
                    }
                }
            }
        }
    }

    fn register_external_card_source(
        &mut self,
        source: ExternalCardSourceFile,
    ) -> Result<usize, String> {
        if !source.replace_existing {
            let definition_names = Self::external_source_definition_names(&source);
            self.ensure_card_definitions_loaded(definition_names.iter().copied());

            // Linked definitions are an atomic group: mixing an embedded face
            // with a newly compiled face would leave their CardIds pointing at
            // different groups. Preserve the whole existing group if any face
            // (or the single card) is already present.
            if definition_names
                .iter()
                .any(|name| self.registry.get(name).is_some())
            {
                self.register_resolvable_external_aliases(&source.aliases);
                return Ok(0);
            }
        }

        self.register_external_source_metadata(&source);

        let definitions = match &source.group {
            ExternalCardSourceGroup::Single { name, block, .. } => {
                vec![self.compile_external_single_card(name, block)?]
            }
            ExternalCardSourceGroup::Linked {
                layout,
                faces,
                has_fuse,
                ..
            } => self.compile_external_linked_group(layout, faces, *has_fuse)?,
        };

        let mut loaded = 0usize;
        for definition in definitions {
            self.clear_external_error(definition.name());
            if self.registry.get(definition.name()).is_none() {
                loaded += 1;
            }
            self.registry.register(definition.clone());
            self.game.register_linked_face_definition(&definition);
        }

        for alias in source.aliases {
            self.registry.register_alias(alias.alias, alias.canonical);
        }

        Ok(loaded)
    }

    fn register_external_card_sources_input(
        &mut self,
        input: ExternalCardSourcesInput,
    ) -> ExternalCardRegistrationSummary {
        let sources = match input {
            ExternalCardSourcesInput::Single(source) => vec![source],
            ExternalCardSourcesInput::Many(sources) => sources,
        };

        let mut loaded = 0usize;
        let mut failed = Vec::new();
        for source in sources {
            let failure_name = match &source.group {
                ExternalCardSourceGroup::Single { name, .. } => name.clone(),
                ExternalCardSourceGroup::Linked {
                    combined_name,
                    faces,
                    ..
                } => faces
                    .first()
                    .map(|face| face.name.clone())
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| combined_name.clone()),
            };
            match self.register_external_card_source(source) {
                Ok(count) => loaded += count,
                Err(error) => {
                    self.remember_external_error(&failure_name, &error);
                    failed.push(ExternalCardRegistrationFailure {
                        name: failure_name,
                        error,
                    });
                }
            }
        }

        ExternalCardRegistrationSummary { loaded, failed }
    }
}

#[cfg(test)]
mod external_registry_tests {
    use super::*;
    use ironsmith::types::CardType;
    use serde_json::json;

    #[test]
    fn preserves_embedded_definitions_unless_replacement_is_explicit() {
        let _id_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

        let replacement = json!({
            "canonicalName": "Lightning Bolt",
            "group": {
                "kind": "single",
                "name": "Lightning Bolt",
                "block": "Type: Creature — Impostor\nPower/Toughness: 1/1",
                "score": 1.0
            }
        });
        let summary: serde_json::Value = serde_json::from_str(
            &wasm
                .register_external_card_sources_json(replacement.to_string())
                .expect("external source should be accepted"),
        )
        .expect("registration summary should decode");
        assert_eq!(summary["loaded"], 0);
        let preserved = wasm
            .registry
            .get("Lightning Bolt")
            .expect("registration should lazily load the embedded definition");
        assert!(!preserved.card.card_types.contains(&CardType::Creature));

        let replacement = json!({
            "canonicalName": "Lightning Bolt",
            "replaceExisting": true,
            "group": {
                "kind": "single",
                "name": "Lightning Bolt",
                "block": "Type: Creature — Impostor\nPower/Toughness: 1/1",
                "score": 1.0
            }
        });
        wasm.register_external_card_sources_json(replacement.to_string())
            .expect("explicit replacement should be accepted");
        let replaced = wasm
            .registry
            .get("Lightning Bolt")
            .expect("replacement definition should be registered");
        assert!(replaced.card.card_types.contains(&CardType::Creature));
    }
}

#[wasm_bindgen]
impl WasmGame {
    /// Register a compiler-produced card artifact in the parser-free engine.
    #[wasm_bindgen(js_name = registerCompiledCardArtifact)]
    pub fn register_compiled_card_artifact(
        &mut self,
        artifact_js: JsValue,
    ) -> Result<(), JsValue> {
        let artifact: CompiledCardArtifact = serde_wasm_bindgen::from_value(artifact_js)
            .map_err(|err| JsValue::from_str(&format!("invalid compiled card artifact: {err}")))?;
        let definitions = Self::materialize_compiled_artifact_batch(std::slice::from_ref(&artifact))
            .map_err(|err| {
                JsValue::from_str(&format!("compiled card registration failed: {err}"))
            })?;
        for definition in definitions {
            self.registry.register(definition.clone());
            self.game.register_linked_face_definition(&definition);
        }
        Ok(())
    }

    /// Register one source group after the compiler module has produced its
    /// typed artifacts. Artifact-local face IDs are remapped atomically into
    /// this engine session, so unrelated groups cannot collide.
    #[wasm_bindgen(js_name = registerCompiledCardSourceArtifacts)]
    pub fn register_compiled_card_source_artifacts(
        &mut self,
        source_js: JsValue,
        artifacts_js: JsValue,
    ) -> Result<JsValue, JsValue> {
        let source: ExternalCardSourceFile = serde_wasm_bindgen::from_value(source_js)
            .map_err(|err| JsValue::from_str(&format!("invalid card source payload: {err}")))?;
        let artifacts: Vec<CompiledCardArtifact> = serde_wasm_bindgen::from_value(artifacts_js)
            .map_err(|err| JsValue::from_str(&format!("invalid compiled artifact batch: {err}")))?;
        let definition_names = Self::external_source_definition_names(&source);
        if !source.replace_existing {
            self.ensure_card_definitions_loaded(definition_names.iter().copied());
            if definition_names
                .iter()
                .any(|name| self.registry.get(name).is_some())
            {
                self.register_resolvable_external_aliases(&source.aliases);
                return serde_wasm_bindgen::to_value(&ExternalCardRegistrationSummary {
                    loaded: 0,
                    failed: Vec::new(),
                })
                .map_err(|err| JsValue::from_str(&format!("card source summary encode failed: {err}")));
            }
        }

        if artifacts.len() != definition_names.len() {
            return Err(JsValue::from_str(&format!(
                "compiled artifact batch has {} card(s), but source group has {}",
                artifacts.len(),
                definition_names.len()
            )));
        }
        self.register_external_source_metadata(&source);
        let definitions = Self::materialize_compiled_artifact_batch(&artifacts)
            .map_err(|err| JsValue::from_str(&format!("compiled card registration failed: {err}")))?;
        let loaded = definitions.len();
        for definition in definitions {
            self.clear_external_error(definition.name());
            self.registry.register(definition.clone());
            self.game.register_linked_face_definition(&definition);
        }
        for alias in source.aliases {
            self.registry.register_alias(alias.alias, alias.canonical);
        }
        serde_wasm_bindgen::to_value(&ExternalCardRegistrationSummary {
            loaded,
            failed: Vec::new(),
        })
        .map_err(|err| JsValue::from_str(&format!("card source summary encode failed: {err}")))
    }

    #[wasm_bindgen(js_name = registerExternalCardSourcesJson)]
    pub fn register_external_card_sources_json(
        &mut self,
        sources_json: String,
    ) -> Result<String, JsValue> {
        let input: ExternalCardSourcesInput = serde_json::from_str(&sources_json)
            .map_err(|err| JsValue::from_str(&format!("invalid card source JSON: {err}")))?;
        let summary = self.register_external_card_sources_input(input);
        serde_json::to_string(&summary)
            .map_err(|err| JsValue::from_str(&format!("card source summary encode failed: {err}")))
    }

    #[wasm_bindgen(js_name = registerExternalCardSources)]
    pub fn register_external_card_sources(&mut self, sources: JsValue) -> Result<JsValue, JsValue> {
        let input: ExternalCardSourcesInput = serde_wasm_bindgen::from_value(sources)
            .map_err(|err| JsValue::from_str(&format!("invalid card source payload: {err}")))?;
        let summary = self.register_external_card_sources_input(input);
        serde_wasm_bindgen::to_value(&summary)
            .map_err(|err| JsValue::from_str(&format!("card source summary encode failed: {err}")))
    }
}
