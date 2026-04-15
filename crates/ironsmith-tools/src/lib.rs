use ironsmith::cards::builders::CardTextError;
use ironsmith::cards::{CardDefinition, CardDefinitionBuilder};
use ironsmith::ids::CardId;
use ironsmith_compiler::{
    CardTextError as CompilerCardTextError, CompilePolicy, CompiledCardText, CompilerBackend,
    CompilerCompileRequest, CompilerFacade,
};

pub use ironsmith::tooling::*;

struct RuntimeBuilderBackend;

impl CompilerBackend<String, CardDefinition, ()> for RuntimeBuilderBackend {
    fn compile(
        &self,
        request: CompilerCompileRequest<String>,
    ) -> Result<CompiledCardText<CardDefinition>, CompilerCardTextError> {
        let compiler = CompilerFacade::new();
        let _ = compiler.prepare_source(&request.text);

        let builder = CardDefinitionBuilder::new(CardId::new(), &request.context);
        let definition = if request.policy.allow_unsupported {
            ironsmith::cards::builders::parse_card_text_allow_unsupported(builder, request.text.as_str())
        } else {
            ironsmith::cards::builders::parse_card_text(builder, request.text.as_str())
        }
        .map_err(|err| match err {
            CardTextError::UnsupportedLine(message) => {
                CompilerCardTextError::UnsupportedLine(message)
            }
            CardTextError::ParseError(message) => CompilerCardTextError::ParseError(message),
            CardTextError::InvariantViolation(message) => {
                CompilerCardTextError::InvariantViolation(message)
            }
        })?;

        Ok(CompiledCardText {
            definition,
            annotations: Default::default(),
        })
    }

    fn analyze(&self, _request: CompilerCompileRequest<String>) -> Result<(), CompilerCardTextError> {
        Ok(())
    }
}

pub fn make_compiler_request(
    name: impl Into<String>,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> CompilerCompileRequest<String> {
    CompilerCompileRequest::new(
        name.into(),
        text,
        CompilePolicy { allow_unsupported },
    )
}

pub fn parse_card_definition_with_runtime_builder(
    name: &str,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<CardDefinition, CardTextError> {
    let request = make_compiler_request(name.to_string(), text, allow_unsupported);
    CompilerFacade::new()
        .compile_with_backend(&RuntimeBuilderBackend, request)
        .map(|compiled| compiled.definition)
        .map_err(|err| match err {
            CompilerCardTextError::UnsupportedLine(message) => CardTextError::UnsupportedLine(message),
            CompilerCardTextError::ParseError(message) => CardTextError::ParseError(message),
            CompilerCardTextError::InvariantViolation(message) => {
                CardTextError::InvariantViolation(message)
            }
        })
}
