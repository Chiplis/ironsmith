fn external_card_lookup_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn external_card_score(score: Option<f32>) -> Option<f32> {
    score.map(|value| value.clamp(0.0, 1.0))
}

impl WasmGame {
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
        let front_builder = ironsmith_compiler::CardDefinitionBuilder::new(front_id, &front.name);
        let back_builder = ironsmith_compiler::CardDefinitionBuilder::new(back_id, &back.name);
        let mut front_definition = ironsmith_registry::compile_builder_to_runtime_definition(
            front_builder,
            front.block.clone(),
            false,
        )
        .map_err(|err| format!("front face: {err}"))?;
        let mut back_definition = ironsmith_registry::compile_builder_to_runtime_definition(
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

#[wasm_bindgen]
impl WasmGame {
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
