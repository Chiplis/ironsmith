use crate::diagnostics::{CardTextError, ParseAnnotations};
use crate::facade::CompiledCardText;

/// Compiler-owned contract for the semantic/lowering pipeline.
///
/// The concrete implementation still lives outside this crate for now, but the
/// stage boundaries now have a stable compiler-owned home.
pub trait LoweringPipeline<Context, SemanticDocument, ParsedDocument, PreparedDocument, Definition>
{
    fn parse_text_to_semantic_document(
        &self,
        context: Context,
        text: String,
        allow_unsupported: bool,
    ) -> Result<(SemanticDocument, ParseAnnotations), CardTextError>;

    fn parse_semantic_document(
        &self,
        document: SemanticDocument,
    ) -> Result<ParsedDocument, CardTextError>;

    fn prepare_parsed_document(
        &self,
        document: ParsedDocument,
    ) -> Result<PreparedDocument, CardTextError>;

    fn lower_prepared_document(
        &self,
        document: PreparedDocument,
    ) -> Result<CompiledCardText<Definition>, CardTextError>;
}

/// Compiler-owned postpass contract for compiled definitions.
pub trait PostpassProcessor<Context, Definition> {
    fn apply(
        &self,
        definition: Definition,
        context: &Context,
        original_text: &str,
    ) -> Result<Definition, CardTextError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakePipeline;

    impl LoweringPipeline<String, String, String, String, String> for FakePipeline {
        fn parse_text_to_semantic_document(
            &self,
            context: String,
            text: String,
            allow_unsupported: bool,
        ) -> Result<(String, ParseAnnotations), CardTextError> {
            Ok((
                format!("{context}:{text}:{allow_unsupported}"),
                ParseAnnotations::default(),
            ))
        }

        fn parse_semantic_document(&self, document: String) -> Result<String, CardTextError> {
            Ok(format!("parsed<{document}>"))
        }

        fn prepare_parsed_document(&self, document: String) -> Result<String, CardTextError> {
            Ok(format!("prepared<{document}>"))
        }

        fn lower_prepared_document(
            &self,
            document: String,
        ) -> Result<CompiledCardText<String>, CardTextError> {
            Ok(CompiledCardText {
                definition: format!("lowered<{document}>"),
                annotations: ParseAnnotations::default(),
            })
        }
    }

    struct FakePostpass;

    impl PostpassProcessor<String, String> for FakePostpass {
        fn apply(
            &self,
            definition: String,
            context: &String,
            original_text: &str,
        ) -> Result<String, CardTextError> {
            Ok(format!("{definition}|{context}|{original_text}"))
        }
    }

    #[test]
    fn lowering_pipeline_trait_supports_staged_compilation() {
        let pipeline = FakePipeline;
        let (semantic, _annotations) = pipeline
            .parse_text_to_semantic_document(
                "Divination".to_string(),
                "Draw two cards.".to_string(),
                false,
            )
            .expect("semantic stage should succeed");
        let parsed = pipeline
            .parse_semantic_document(semantic)
            .expect("parse stage should succeed");
        let prepared = pipeline
            .prepare_parsed_document(parsed)
            .expect("prepare stage should succeed");
        let lowered = pipeline
            .lower_prepared_document(prepared)
            .expect("lowering stage should succeed");

        assert_eq!(
            lowered.definition,
            "lowered<prepared<parsed<Divination:Draw two cards.:false>>>"
        );
    }

    #[test]
    fn postpass_processor_trait_can_wrap_compiled_definitions() {
        let postpass = FakePostpass;
        let applied = postpass
            .apply(
                "compiled".to_string(),
                &"builder".to_string(),
                "Draw two cards.",
            )
            .expect("postpass should succeed");

        assert_eq!(applied, "compiled|builder|Draw two cards.");
    }
}
