use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::cards::CardDefinition;
use crate::cards::builders::{CardDefinitionBuilder, CardTextError, ParseAnnotations};

use super::model::SemanticDocument;
use super::postpasses;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CompilePolicy {
    pub(crate) allow_unsupported: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledCardText {
    pub(crate) definition: CardDefinition,
    pub(crate) annotations: ParseAnnotations,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ParseCacheKey {
    pub(crate) builder_context: String,
    pub(crate) text: String,
    pub(crate) allow_unsupported: bool,
}

impl ParseCacheKey {
    pub(crate) fn new(
        builder: &CardDefinitionBuilder,
        text: &str,
        allow_unsupported: bool,
    ) -> Self {
        Self {
            builder_context: format!("{builder:?}"),
            text: text.to_string(),
            allow_unsupported,
        }
    }
}

pub(crate) type CachedParseResult = Result<CompiledCardText, CardTextError>;

fn parse_result_cache() -> &'static Mutex<HashMap<ParseCacheKey, CachedParseResult>> {
    static PARSE_RESULT_CACHE: OnceLock<Mutex<HashMap<ParseCacheKey, CachedParseResult>>> =
        OnceLock::new();
    PARSE_RESULT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn lookup_cached_parse(key: &ParseCacheKey) -> Option<CachedParseResult> {
    parse_result_cache()
        .lock()
        .expect("parse result cache mutex poisoned")
        .get(key)
        .cloned()
}

fn store_cached_parse(key: ParseCacheKey, result: CachedParseResult) -> CachedParseResult {
    parse_result_cache()
        .lock()
        .expect("parse result cache mutex poisoned")
        .insert(key, result.clone());
    result
}

pub(crate) struct CardTextCompiler;

impl CardTextCompiler {
    pub(crate) fn compile(
        builder: CardDefinitionBuilder,
        text: String,
        policy: CompilePolicy,
    ) -> CachedParseResult {
        let cache_key = ParseCacheKey::new(&builder, &text, policy.allow_unsupported);
        if let Some(cached) = lookup_cached_parse(&cache_key) {
            return cached;
        }

        let original_builder = builder.clone();
        let result =
            super::parse_text_with_annotations(builder, text.clone(), policy.allow_unsupported)
                .and_then(|(definition, annotations)| {
                    postpasses::apply(definition, &original_builder, &text).map(|definition| {
                        CompiledCardText {
                            definition,
                            annotations,
                        }
                    })
                });

        store_cached_parse(cache_key, result)
    }

    #[allow(dead_code)]
    pub(crate) fn analyze(
        builder: CardDefinitionBuilder,
        text: String,
        policy: CompilePolicy,
    ) -> Result<SemanticDocument, CardTextError> {
        let (doc, _) =
            super::parse_text_to_semantic_document(builder, text, policy.allow_unsupported)?;
        Ok(doc)
    }
}
