use crate::diagnostics::{CardTextError, ParseAnnotations};
use crate::front_end::{
    LineInfo, MetadataLine, NormalizedLine, OwnedLexToken, SentenceSplitResult, lex_line,
    make_line_info, normalize_trimmed_line, parse_metadata_line, split_text_for_parse,
    split_text_for_parse_with_restrictions,
};
use crate::model::{ParsedRestrictions, RestrictionBucket};
use std::hash::Hash;

fn fallback_static_ability_id_name(
    id: crate::static_abilities::StaticAbilityId,
) -> Option<&'static str> {
    use crate::static_abilities::StaticAbilityId;

    match id {
        StaticAbilityId::KeywordFallbackText => Some("KeywordFallbackText"),
        StaticAbilityId::RuleFallbackText => Some("RuleFallbackText"),
        StaticAbilityId::UnsupportedParserLine => Some("UnsupportedParserLine"),
        _ => None,
    }
}

fn fallback_static_ability_issue(
    ability: &crate::static_abilities::StaticAbility,
    context: &str,
) -> Option<String> {
    if let Some(id) = ability.id {
        if let Some(id_name) = fallback_static_ability_id_name(id) {
            return Some(format!(
                "{context} compiled to unsupported static ability fallback {id_name}: {}",
                ability.display()
            ));
        }
    } else {
        return Some(format!(
            "{context} compiled to an unsupported static ability without a semantic id: {}",
            ability.display()
        ));
    }

    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbilityPayload;
    use ironsmith_core::Grantable;

    match &ability.payload {
        StaticAbilityPayload::AttachedAbilityGrant(grant) => {
            fallback_ability_issue(&grant.ability, "attached granted ability")
        }
        StaticAbilityPayload::Conditional { ability, .. } => {
            fallback_static_ability_issue(ability, "conditional static ability")
        }
        StaticAbilityPayload::GrantAbility(grant) => {
            fallback_ability_issue(&grant.ability, "granted ability")
        }
        StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
            fallback_ability_issue(&grant.ability, "granted object ability")
        }
        StaticAbilityPayload::LevelAbility(level) => level
            .abilities
            .iter()
            .find_map(|ability| fallback_static_ability_issue(ability, "level static ability")),
        StaticAbilityPayload::EquipmentGrant(abilities) => abilities.iter().find_map(|ability| {
            fallback_static_ability_issue(ability, "equipment granted static ability")
        }),
        StaticAbilityPayload::SoulbondSharedAbility(ability) => {
            fallback_static_ability_issue(ability, "soulbond shared static ability")
        }
        StaticAbilityPayload::SoulbondSharedObjectAbility(ability) => match &ability.kind {
            AbilityKind::Static(static_ability) => {
                fallback_static_ability_issue(static_ability, "soulbond shared object ability")
            }
            _ => None,
        },
        StaticAbilityPayload::Grants(spec) => match &spec.grantable {
            Grantable::Ability(static_ability) => {
                fallback_static_ability_issue(static_ability, "static grant spec ability")
            }
            _ => None,
        },
        _ => None,
    }
}

fn fallback_ability_issue(ability: &crate::ability::Ability, context: &str) -> Option<String> {
    match &ability.kind {
        crate::ability::AbilityKind::Static(static_ability) => {
            fallback_static_ability_issue(static_ability, context)
        }
        _ => None,
    }
}

fn reject_compiled_parser_fallbacks(
    definition: &crate::cards::CardDefinition,
) -> Result<(), CardTextError> {
    for ability in &definition.abilities {
        if let Some(issue) = fallback_ability_issue(ability, "card ability") {
            return Err(CardTextError::UnsupportedLine(issue));
        }
    }

    let debug = format!("{definition:#?}");
    if debug.contains("RemoveAbility")
        && debug.contains("KeywordMarker")
        && debug.to_ascii_lowercase().contains("soulbond")
    {
        return Err(CardTextError::UnsupportedLine(
            "compiled card removes soulbond through a keyword marker without real semantics"
                .to_string(),
        ));
    }
    for marker in [
        "KeywordFallbackText",
        "RuleFallbackText",
        "UnsupportedParserLine",
        "KeywordAction::Marker",
        "KeywordAction::MarkerText",
        "RewriteLineCst::Unsupported",
        "RewriteSemanticItem::Unsupported",
        "RewriteUnsupportedLine",
    ] {
        if debug.contains(marker) {
            return Err(CardTextError::UnsupportedLine(format!(
                "compiled card still contains unsupported parser fallback marker {marker}"
            )));
        }
    }

    Ok(())
}

/// Prepared compiler source document with line-oriented views for future parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerSourceDocument {
    pub original_lines: Vec<String>,
    pub normalized_lines: Vec<String>,
}

/// Compiler options for parse/compile entrypoints.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CompilePolicy {
    pub allow_unsupported: bool,
}

/// Compiler-owned request shape for a parse/compile operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerCompileRequest<Context> {
    pub context: Context,
    pub text: String,
    pub policy: CompilePolicy,
}

impl<Context> CompilerCompileRequest<Context> {
    pub fn new(context: Context, text: impl Into<String>, policy: CompilePolicy) -> Self {
        Self {
            context,
            text: text.into(),
            policy,
        }
    }
}

impl<Context: Clone + Eq + Hash> CompilerCompileRequest<Context> {
    pub fn cache_key(&self) -> ParseCacheKey<Context> {
        ParseCacheKey::new(self.context.clone(), self.text.clone(), self.policy)
    }
}

/// Compiler-owned cache key shape. Runtime still owns the concrete cache backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseCacheKey<Context> {
    pub context: Context,
    pub text: String,
    pub allow_unsupported: bool,
}

impl<Context> ParseCacheKey<Context> {
    pub fn new(context: Context, text: impl Into<String>, policy: CompilePolicy) -> Self {
        Self {
            context,
            text: text.into(),
            allow_unsupported: policy.allow_unsupported,
        }
    }
}

/// Compiler-owned output envelope for parsed card text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledCardText<Definition> {
    pub definition: Definition,
    pub annotations: ParseAnnotations,
}

/// Backend contract for the remaining compiler pipeline.
///
/// `ironsmith-compiler` now owns the request and result shapes, while concrete
/// parser/lowering/postpass implementations can migrate behind this trait
/// incrementally without changing callers again.
pub trait CompilerBackend<Context, Definition, SemanticDocument> {
    fn compile(
        &self,
        request: CompilerCompileRequest<Context>,
    ) -> Result<CompiledCardText<Definition>, CardTextError>;

    fn analyze(
        &self,
        request: CompilerCompileRequest<Context>,
    ) -> Result<SemanticDocument, CardTextError>;
}

impl CompilerSourceDocument {
    pub fn is_empty(&self) -> bool {
        self.original_lines.is_empty()
    }
}

/// Workspace-facing compiler entry point for parser-front-end services.
#[derive(Debug, Clone, Default)]
pub struct CompilerFacade;

impl CompilerFacade {
    pub fn new() -> Self {
        Self
    }

    /// Prepare a line-oriented source document and diagnostic annotations.
    ///
    /// This keeps ownership of basic source preprocessing in `ironsmith-compiler`
    /// even before the full parser/lowering pipeline is extracted.
    pub fn prepare_source(&self, text: &str) -> (CompilerSourceDocument, ParseAnnotations) {
        let mut annotations = ParseAnnotations::default();
        let mut original_lines = Vec::new();
        let mut normalized_lines = Vec::new();

        for (line_index, raw_line) in text.split('\n').enumerate() {
            let original = raw_line.trim_end_matches('\r').to_string();
            let normalized = normalize_line(&original);
            let char_map = build_char_map(&original, &normalized);

            annotations.record_original_line(line_index, original.clone());
            annotations.record_normalized_line(line_index, normalized.clone());
            annotations.record_char_map(line_index, char_map);

            original_lines.push(original);
            normalized_lines.push(normalized);
        }

        (
            CompilerSourceDocument {
                original_lines,
                normalized_lines,
            },
            annotations,
        )
    }

    /// Lex a single normalized source line using the compiler-owned lexer.
    pub fn lex_line(
        &self,
        line: &str,
        line_index: usize,
    ) -> Result<Vec<OwnedLexToken>, CardTextError> {
        lex_line(line, line_index)
    }

    /// Split a raw/normalized line pair into parser-facing sentence fragments.
    pub fn split_text_for_parse(
        &self,
        raw_text: &str,
        normalized_text: &str,
        line_index: usize,
    ) -> SentenceSplitResult {
        split_text_for_parse(raw_text, normalized_text, line_index)
    }

    /// Split text into parseable clauses while bucketing restriction sentences.
    pub fn split_text_for_parse_with_restrictions(
        &self,
        raw_text: &str,
        normalized_text: &str,
        line_index: usize,
        classify: impl FnMut(&str, &[OwnedLexToken]) -> Option<RestrictionBucket>,
    ) -> (Vec<String>, ParsedRestrictions) {
        split_text_for_parse_with_restrictions(raw_text, normalized_text, line_index, classify)
    }

    /// Parse a compiler metadata line such as `Mana Cost:` or `Type Line:`.
    pub fn parse_metadata_line(&self, line: &str) -> Result<Option<MetadataLine>, CardTextError> {
        parse_metadata_line(line)
    }

    /// Normalize a single non-empty source line for parser consumption.
    pub fn normalize_trimmed_line(&self, line: &str) -> Option<NormalizedLine> {
        normalize_trimmed_line(line)
    }

    /// Build compiler line info from a normalized source line.
    pub fn make_line_info(
        &self,
        line_index: usize,
        raw_line: impl Into<String>,
        normalized: NormalizedLine,
    ) -> LineInfo {
        make_line_info(line_index, raw_line, normalized)
    }

    /// Canonical compiler-owned compile entrypoint over a backend implementation.
    pub fn compile_with_backend<Context, Definition, SemanticDocument, Backend>(
        &self,
        backend: &Backend,
        request: CompilerCompileRequest<Context>,
    ) -> Result<CompiledCardText<Definition>, CardTextError>
    where
        Backend: CompilerBackend<Context, Definition, SemanticDocument>,
    {
        backend.compile(request)
    }

    /// Canonical compiler-owned definition compiler.
    ///
    /// This is the public entrypoint for the native backend now compiled inside
    /// `ironsmith-compiler`. Adapters should call this instead of importing the
    /// backend module tree or runtime-hosted parser wrappers.
    pub fn compile_definition(
        &self,
        builder: crate::cards::CardDefinitionBuilder,
        text: impl Into<String>,
        policy: CompilePolicy,
    ) -> Result<CompiledCardText<crate::cards::CardDefinition>, CardTextError> {
        let compiled =
            crate::compile_card_text_with_policy(builder, text, policy.allow_unsupported)?;
        reject_compiled_parser_fallbacks(&compiled.definition)?;
        Ok(CompiledCardText {
            definition: compiled.definition,
            annotations: compiled.annotations,
        })
    }

    /// Canonical compiler-owned analyze entrypoint over a backend implementation.
    pub fn analyze_with_backend<Context, Definition, SemanticDocument, Backend>(
        &self,
        backend: &Backend,
        request: CompilerCompileRequest<Context>,
    ) -> Result<SemanticDocument, CardTextError>
    where
        Backend: CompilerBackend<Context, Definition, SemanticDocument>,
    {
        backend.analyze(request)
    }
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_char_map(original: &str, normalized: &str) -> Vec<usize> {
    if normalized.is_empty() {
        return Vec::new();
    }

    let original_chars: Vec<char> = original.chars().collect();
    let normalized_chars: Vec<char> = normalized.chars().collect();
    let mut map = Vec::with_capacity(normalized_chars.len());
    let mut original_idx = 0usize;

    for normalized_char in normalized_chars {
        while original_idx < original_chars.len()
            && original_chars[original_idx].is_whitespace()
            && normalized_char != ' '
        {
            original_idx += 1;
        }

        if normalized_char == ' ' {
            while original_idx < original_chars.len()
                && !original_chars[original_idx].is_whitespace()
            {
                original_idx += 1;
            }
            while original_idx < original_chars.len()
                && original_chars[original_idx].is_whitespace()
            {
                original_idx += 1;
            }
            map.push(original_idx.saturating_sub(1));
            continue;
        }

        map.push(original_idx);
        original_idx += 1;
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_source_normalizes_whitespace_and_tracks_lines() {
        let facade = CompilerFacade::new();
        let (document, annotations) =
            facade.prepare_source(" Draw   a  card.\r\nThen\tdiscard one.");

        assert_eq!(document.original_lines.len(), 2);
        assert_eq!(document.normalized_lines[0], "Draw a card.");
        assert_eq!(document.normalized_lines[1], "Then discard one.");
        assert_eq!(annotations.original_lines[&0], " Draw   a  card.");
        assert_eq!(annotations.normalized_lines[&1], "Then discard one.");
        assert!(!annotations.normalized_char_maps[&0].is_empty());
    }

    #[test]
    fn prepare_source_handles_empty_input() {
        let facade = CompilerFacade::new();
        let (document, annotations) = facade.prepare_source("");

        assert_eq!(document.original_lines, vec![String::new()]);
        assert_eq!(document.normalized_lines, vec![String::new()]);
        assert_eq!(annotations.normalized_char_maps[&0], Vec::<usize>::new());
    }

    #[test]
    fn facade_can_lex_a_prepared_line() {
        let facade = CompilerFacade::new();
        let tokens = facade
            .lex_line("Draw a card.", 0)
            .expect("prepared source line should lex");

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].parser_text(), "draw");
        assert!(tokens[3].is_period());
    }

    #[test]
    fn facade_can_split_text_for_parse() {
        let facade = CompilerFacade::new();
        let split = facade.split_text_for_parse(
            "Draw a card. (Activate only as a sorcery.)",
            "Draw a card. (Activate only as a sorcery.)",
            0,
        );

        assert_eq!(split.sentences, vec!["Draw a card"]);
        assert_eq!(
            split.parenthetical_sentences,
            vec!["Activate only as a sorcery"]
        );
    }

    #[test]
    fn facade_exposes_metadata_and_line_normalization_helpers() {
        let facade = CompilerFacade::new();
        let metadata = facade
            .parse_metadata_line("Type Line: Legendary Creature — Human")
            .expect("metadata parse should succeed");
        let normalized = facade
            .normalize_trimmed_line("  Draw   a card.  ")
            .expect("line should normalize");
        let info = facade.make_line_info(3, "Draw   a card.", normalized.clone());

        assert!(matches!(
            metadata,
            Some(MetadataLine::TypeLine(value)) if value == "Legendary Creature — Human"
        ));
        assert_eq!(normalized.normalized, "Draw a card.");
        assert_eq!(info.line_index, 3);
        assert_eq!(info.normalized, normalized);
    }

    #[test]
    fn facade_can_bucket_restrictions_during_parse_split() {
        let facade = CompilerFacade::new();
        let (parsed, restrictions) = facade.split_text_for_parse_with_restrictions(
            "Draw a card. Activate only as a sorcery.",
            "Draw a card. Activate only as a sorcery.",
            0,
            |sentence, _tokens| {
                sentence
                    .starts_with("Activate only")
                    .then_some(RestrictionBucket::Activation)
            },
        );

        assert_eq!(parsed, vec!["Draw a card"]);
        assert_eq!(restrictions.activation, vec!["Activate only as a sorcery"]);
    }

    #[test]
    fn compile_request_builds_stable_cache_key() {
        let request = CompilerCompileRequest::new(
            "builder:Divination".to_string(),
            "Draw two cards.",
            CompilePolicy {
                allow_unsupported: true,
            },
        );
        let key = request.cache_key();

        assert_eq!(key.context, "builder:Divination");
        assert_eq!(key.text, "Draw two cards.");
        assert!(key.allow_unsupported);
    }

    #[test]
    fn compile_definition_preserves_semantic_keyword_markers_even_when_allowed() {
        let facade = CompilerFacade::new();
        let builder =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Rampage Variant")
                .card_types(vec![crate::types::CardType::Creature]);

        let compiled = facade
            .compile_definition(
                builder,
                "Rampage 2",
                CompilePolicy {
                    allow_unsupported: true,
                },
            )
            .expect("keyword markers are semantic labels and should survive strict compilation");

        assert!(
            format!("{:#?}", compiled.definition).contains("KeywordMarker")
                && format!("{:#?}", compiled.definition).contains("rampage 2"),
            "expected retained KeywordMarker in {:#?}",
            compiled.definition
        );
    }

    #[test]
    fn compile_definition_accepts_supported_craft_keyword_even_when_allowed() {
        let facade = CompilerFacade::new();
        let builder =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Craft Variant")
                .card_types(vec![crate::types::CardType::Artifact]);

        let compiled = facade
            .compile_definition(
                builder,
                "Craft with artifact {3}{W}{W}",
                CompilePolicy {
                    allow_unsupported: true,
                },
            )
            .expect("craft is now a supported activated keyword ability");

        let debug = format!("{:#?}", compiled.definition);
        assert!(
            debug.contains("Activated")
                && debug.contains("EmitKeywordActionEffect")
                && debug.contains("Craft")
                && debug.contains("TransformEffect"),
            "expected supported craft activated ability, got {debug}"
        );
    }

    #[test]
    fn compile_definition_rejects_unidentified_static_fallback_even_when_allowed() {
        let facade = CompilerFacade::new();
        let builder =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Fallback Variant")
                .with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::unsupported_parser_line(
                        "banding",
                        "test fallback",
                    ),
                ));

        let err = facade
            .compile_definition(
                builder,
                "",
                CompilePolicy {
                    allow_unsupported: true,
                },
            )
            .expect_err("unidentified fallback text should fail loudly");

        assert!(
            matches!(
                err,
                CardTextError::UnsupportedLine(ref message)
                    if message.contains("without a semantic id") && message.contains("banding")
            ),
            "expected unidentified fallback parse error, got {err:?}"
        );
    }

    #[test]
    fn compile_definition_rejects_unsupported_line_fallback_even_when_allowed() {
        let facade = CompilerFacade::new();
        let builder = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Unsupported Variant",
        );

        let err = facade
            .compile_definition(
                builder,
                "This line should not parse and must not compile as a placeholder.",
                CompilePolicy {
                    allow_unsupported: true,
                },
            )
            .expect_err("unsupported parser fallback should fail loudly");

        assert!(
            matches!(
                err,
                CardTextError::UnsupportedLine(ref message)
                    if message.contains("UnsupportedParserLine")
                        || message.contains("without a semantic id")
            ),
            "expected unsupported fallback parse error, got {err:?}"
        );
    }

    #[test]
    fn compiler_backend_trait_can_be_implemented_by_a_fake_backend() {
        #[derive(Default)]
        struct FakeBackend;

        impl CompilerBackend<String, String, usize> for FakeBackend {
            fn compile(
                &self,
                request: CompilerCompileRequest<String>,
            ) -> Result<CompiledCardText<String>, CardTextError> {
                Ok(CompiledCardText {
                    definition: format!("compiled:{}", request.text),
                    annotations: ParseAnnotations::default(),
                })
            }

            fn analyze(
                &self,
                request: CompilerCompileRequest<String>,
            ) -> Result<usize, CardTextError> {
                Ok(request.text.len())
            }
        }

        let backend = FakeBackend;
        let request = CompilerCompileRequest::new(
            "builder:Divination".to_string(),
            "Draw two cards.",
            CompilePolicy::default(),
        );

        let compiled = backend
            .compile(request.clone())
            .expect("fake backend should compile");
        let analyzed = backend
            .analyze(request)
            .expect("fake backend should analyze");

        assert_eq!(compiled.definition, "compiled:Draw two cards.");
        assert_eq!(analyzed, "Draw two cards.".len());
    }

    #[test]
    fn facade_can_route_compile_and_analyze_through_backend() {
        #[derive(Default)]
        struct EchoBackend;

        impl CompilerBackend<&'static str, &'static str, usize> for EchoBackend {
            fn compile(
                &self,
                request: CompilerCompileRequest<&'static str>,
            ) -> Result<CompiledCardText<&'static str>, CardTextError> {
                Ok(CompiledCardText {
                    definition: request.context,
                    annotations: ParseAnnotations::default(),
                })
            }

            fn analyze(
                &self,
                request: CompilerCompileRequest<&'static str>,
            ) -> Result<usize, CardTextError> {
                Ok(request.text.len())
            }
        }

        let facade = CompilerFacade::new();
        let backend = EchoBackend;
        let compile_request =
            CompilerCompileRequest::new("definition", "hello", CompilePolicy::default());
        let analyze_request =
            CompilerCompileRequest::new("definition", "hello", CompilePolicy::default());

        let compiled = facade
            .compile_with_backend(&backend, compile_request)
            .expect("backend compile should succeed");
        let analyzed = facade
            .analyze_with_backend(&backend, analyze_request)
            .expect("backend analyze should succeed");

        assert_eq!(compiled.definition, "definition");
        assert_eq!(analyzed, 5);
    }
}
