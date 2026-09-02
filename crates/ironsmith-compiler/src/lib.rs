//! The compiler: Oracle text in, runtime card definition out.
//!
//! This crate owns no phase. It composes them — recognition in
//! `ironsmith-compiler-grammar`, lowering in `ironsmith-compiler-lowering` —
//! and is the only place that may name both. That is what keeps the phase
//! graph a line rather than a cycle: neither phase crate can reach the other,
//! so the pipeline can only be assembled here.

#![allow(ambiguous_glob_reexports)]

pub mod canonical_pipeline;
pub mod compiler_pipeline;
pub mod facade;
pub mod pipeline;

pub use ironsmith_compiler_grammar::*;
pub use ironsmith_compiler_lowering::{
    CardDefinitionBuilder, battlefield_entry_counter_fusion, card_builders, card_tokens, cards,
    compile_support, condition_antecedent, effect_pipeline, lower, lowering, lowering_support,
    runtime_static_ability_helpers,
};

pub use facade::{
    CompilePolicy, CompiledCardText, CompilerBackend, CompilerCompileRequest, CompilerFacade,
    CompilerSourceDocument,
};
pub use pipeline::{LoweringPipeline, PostpassProcessor};

pub fn compile_card_text_with_policy(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<facade::CompiledCardText<CardDefinition>, CardTextError> {
    let text = text.into();
    util::with_cached_parser_trace(move || {
        let mut context =
            parse_context_for_builder(&builder.card_builder, &text, allow_unsupported);
        compiler_pipeline::parse_text_with_annotations_lowered_with_facts_context(
            &mut context,
            builder,
            text,
        )
        .map(|lowered| facade::CompiledCardText {
            definition: lowered.definition,
            annotations: lowered.annotations,
        })
    })
}

pub fn compile_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<facade::CompiledCardText<CardDefinition>, CardTextError> {
    compile_card_text_with_policy(builder, text, allow_unsupported)
}

pub fn parse_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    compile_card_text(builder, text, false).map(|compiled| compiled.definition)
}

pub fn parse_card_text_allow_unsupported(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    compile_card_text(builder, text, true).map(|compiled| compiled.definition)
}

/// Parsing a card's text from a builder.
///
/// Parsing is the whole pipeline — recognition then lowering — so it is not
/// something the definition builder can do by itself. This restores the
/// convenience for callers that already have the assembled compiler in scope,
/// without putting a pipeline call back inside the builder.
pub trait ParseCardText: Sized {
    fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError>;

    fn parse_text_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError>;
}

impl ParseCardText for CardDefinitionBuilder {
    fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        parse_card_text(self, text)
    }

    fn parse_text_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        parse_card_text_allow_unsupported(self, text)
    }
}
