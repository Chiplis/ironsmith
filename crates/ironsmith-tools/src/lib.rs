mod tooling;

pub use ironsmith::compiler_integration::{
    CompilerIntegrationError, compile_builder_to_runtime_definition, compile_to_runtime_definition,
    into_runtime_compiled_card_text, into_runtime_definition,
};
pub use tooling::*;

pub fn parse_card_definition_with_runtime_builder(
    name: &str,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<ironsmith::cards::CardDefinition, ironsmith_compiler::CardTextError> {
    compile_to_runtime_definition(name, text, allow_unsupported).map_err(|err| match err {
        CompilerIntegrationError::Parse(err) => err,
        other => ironsmith_compiler::CardTextError::InvariantViolation(other.to_string()),
    })
}
