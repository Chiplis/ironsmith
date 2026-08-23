#![recursion_limit = "256"]
#![expect(clippy::type_complexity, clippy::too_many_arguments)]
#![allow(dead_code, unused_imports, ambiguous_glob_reexports)]

//! Oracle grammar implementation layer.
//!
//! This crate owns the parser grammar and its local recognition helpers.  The
//! public `ironsmith-compiler` package remains the document/lowering facade.

#[path = "../../ironsmith-compiler/src/model/card_document.rs"]
pub mod card_document;

pub mod model {
    pub use crate::card_document::*;
    pub use ironsmith_compiler_semantic::model::*;
}
pub use ironsmith_compiler_semantic::model::{
    canonical_references, compiler_semantic, provenance, symbols,
};
pub use ironsmith_compiler_semantic::*;

pub mod diagnostics {
    pub use ironsmith_compiler_api::*;
}

pub mod front_end {
    pub use crate::front_end_parser_support::*;
    pub use ironsmith_compiler_source::*;
    pub use ironsmith_compiler_syntax::*;

    pub mod grammar {
        pub use crate::grammar::*;
    }

    pub mod semantic_domain_migration {
        pub use crate::semantic_domain_migration::*;
    }
}
pub use front_end::*;

pub mod lexer {
    pub use ironsmith_compiler_syntax::lexer::*;
    pub use ironsmith_grammar_common::lexical::{
        LexedClause, locate_token_kind, locate_token_word, locate_token_word_choice,
        token_slice_all_are_kind, token_slice_last_is,
    };
}

pub use ironsmith_compiler_resolve::effect_ast_normalization;
pub use ironsmith_compiler_resolve::effect_ast_traversal;
pub use ironsmith_compiler_resolve::reference_helpers;
pub use ironsmith_compiler_resolve::reference_resolution;
pub use ironsmith_grammar_common::recognition;

pub use alternative_cast::TrapCondition;
pub use card::{PowerToughness, PtValue};
pub use color::{Color, ColorSet};
pub use cost::{OptionalCost, TotalCost};
pub use diagnostics::{CardTextError, ParseAnnotations, TextSpan};
pub use effect::{ChoiceCount, DelayedTriggerSpec, Effect, EffectId, Until, Value};
pub use ids::{CardId, ObjectId, PlayerId, StableId};
pub use ironsmith_core::{
    AttachmentConditionHost, Condition as ConditionExpr, PermanentLeftBattlefieldControlSurface,
    SourceCounterThresholdSurface, WorkspaceSplitMarker,
};
pub use object::{AuraAttachmentFilter, CounterType};
pub use tag::TagKey;
pub use target::{
    ChooseSpec, ObjectCharacteristic, ObjectCharacteristicRelation,
    ObjectCharacteristicRelationKind, ObjectFilter, ObjectRef, PlayerFilter,
    TaggedObjectConstraint, TaggedOpbjectRelation,
};
pub use types::{CardType, Subtype, Supertype};
pub use zone::Zone;

#[path = "../../ironsmith-compiler/src/slice_primitives.rs"]
pub mod slice_primitives;
#[path = "../../ironsmith-compiler/src/string_primitives.rs"]
pub mod string_primitives;
#[path = "../../ironsmith-compiler/src/front_end/token_primitives.rs"]
pub mod token_primitives;
#[path = "../../ironsmith-compiler/src/word_primitives.rs"]
pub mod word_primitives;

#[path = "../../ironsmith-compiler/src/facade.rs"]
pub mod facade;
#[path = "../../ironsmith-compiler/src/model/semantic_document.rs"]
pub mod ir;
#[path = "../../ironsmith-compiler/src/oracle_grammar.rs"]
pub mod oracle_grammar;
#[path = "../../ironsmith-compiler/src/parse_trace.rs"]
pub mod parse_trace;
#[path = "../../ironsmith-compiler/src/pipeline.rs"]
pub mod pipeline;
#[path = "../../ironsmith-compiler/src/registry.rs"]
pub mod registry;
#[path = "../../ironsmith-compiler/src/front_end/rule_engine.rs"]
pub mod rule_engine;
#[path = "../../ironsmith-compiler/src/front_end/semantic_domain_migration.rs"]
pub mod semantic_domain_migration;

#[path = "../../ironsmith-compiler/src/front_end/grammar/mod.rs"]
pub mod grammar;

#[path = "../../ironsmith-compiler/src/front_end/grammar/effect_clauses/effect_sentences/mod.rs"]
pub mod effect_sentences;

#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/activation_and_restrictions/mod.rs"]
pub mod activation_and_restrictions;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/activation_helpers.rs"]
pub mod activation_helpers;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/clause_support.rs"]
pub mod clause_support;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/keyword_families.rs"]
pub mod keyword_families;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/keyword_payloads.rs"]
pub mod keyword_payloads;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/keyword_registry.rs"]
pub mod keyword_registry;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/keyword_static/mod.rs"]
pub mod keyword_static;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/keyword_static_helpers.rs"]
pub mod keyword_static_helpers;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/modal_helpers.rs"]
pub mod modal_helpers;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/object_filters.rs"]
pub mod object_filters;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/permission_helpers.rs"]
pub mod permission_helpers;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/restriction_support.rs"]
pub mod restriction_support;
#[path = "../../ironsmith-compiler/src/front_end/grammar/effect_clauses/search_library_support.rs"]
pub mod search_library_support;
#[path = "../../ironsmith-compiler/src/front_end/grammar/ability_rules/static_ability_helpers.rs"]
pub mod static_ability_helpers;

#[path = "../../ironsmith-compiler/src/front_end/cst.rs"]
pub mod cst;
#[path = "../../ironsmith-compiler/src/front_end/cst_lowering.rs"]
pub mod cst_lowering;
#[path = "../../ironsmith-compiler/src/front_end/parser_support.rs"]
pub mod front_end_parser_support;
#[path = "../../ironsmith-compiler/src/front_end/semantic_parser_support.rs"]
pub mod parser_support;
#[path = "../../ironsmith-compiler/src/front_end/semantic_preprocess.rs"]
pub mod preprocess;
#[path = "../../ironsmith-compiler/src/front_end/semantic_line_parsing/mod.rs"]
pub mod semantic_line_parsing;
#[path = "../../ironsmith-compiler/src/front_end/shared/util.rs"]
pub mod util;

#[path = "../../ironsmith-compiler/src/lowering/battlefield_entry_counter_fusion.rs"]
pub mod battlefield_entry_counter_fusion;
#[path = "../../ironsmith-compiler/src/front_end/canonical_pipeline.rs"]
pub mod canonical_pipeline;
#[path = "../../ironsmith-compiler/src/lowering/compile_support.rs"]
pub mod compile_support;
#[path = "../../ironsmith-compiler/src/lowering/pipeline.rs"]
pub mod compiler_pipeline;
#[path = "../../ironsmith-compiler/src/lowering/condition_antecedent.rs"]
pub mod condition_antecedent;
#[path = "../../ironsmith-compiler/src/front_end/document/mod.rs"]
pub mod document_parser;
#[path = "../../ironsmith-compiler/src/lowering/effect_pipeline.rs"]
pub mod effect_pipeline;
#[path = "../../ironsmith-compiler/src/lowering/lower/mod.rs"]
pub mod lower;
#[path = "../../ironsmith-compiler/src/lowering/mod.rs"]
pub mod lowering;
#[path = "../../ironsmith-compiler/src/lowering/lowering_support.rs"]
pub mod lowering_support;
#[path = "../../ironsmith-compiler/src/model/modal_support.rs"]
pub mod modal_support;
#[path = "../../ironsmith-compiler/src/parse_loss.rs"]
pub mod parse_loss;
#[path = "../../ironsmith-compiler/src/front_end/semantic_document.rs"]
pub mod semantic_document;
#[path = "../../ironsmith-compiler/src/stack.rs"]
pub mod stack;

#[path = "../../ironsmith-compiler/src/cards/builders.rs"]
pub mod card_builders;

pub mod cards {
    pub use crate::card_builders::CardDefinitionBuilder;
    pub use ironsmith_compiler_semantic::cards::*;

    pub mod builders {
        pub use crate::card_builders::*;
    }

    pub mod tokens {
        pub use crate::card_tokens::*;
    }
}

pub use card_builders::CardDefinitionBuilder;
pub use ironsmith_compiler_semantic::cards::CardDefinition;

#[path = "../../ironsmith-compiler/src/cards/tokens.rs"]
pub mod card_tokens;

pub mod host {
    pub use crate::cards::builders::{
        CardTextError, EffectAst, OwnedLexToken, PlayerAst, PredicateAst, SubjectAst, TagKey,
        TargetAst, TriggerSpec,
    };

    pub const IT_TAG: &str = "__it__";
    pub const ADDITIONAL_COST_OBJECT_TAG: &str = "__additional_cost_object__";
    pub const CHOSEN_OBJECTS_TAG: &str = ironsmith_core::CHOSEN_OBJECTS_TAG;
    pub const COPIED_STACK_OBJECT_TAG: &str = "__copied_stack_object__";
    pub const THIS_WAY_SACRIFICED_TAG: &str = "__this_way_sacrificed__";
}

pub fn parse_context_for_builder(
    builder: &CardDefinitionBuilder,
    text: &str,
    allow_unsupported: bool,
) -> ironsmith_compiler_ast::ParseContext {
    use ironsmith_compiler_ast::{
        CardFaceMetadata, ParseContext, ParseFeatures, ProvenanceStore, SourceIdentity,
        SourceUnitId,
    };
    let card_name = builder.card_builder.name_ref().trim().to_string();
    let mut context = ParseContext::new(
        SourceIdentity {
            unit: SourceUnitId(0),
            card_name: card_name.clone(),
            face_index: 0,
            source_len: text.len(),
            source_line_count: text.lines().count(),
        },
        CardFaceMetadata {
            supertypes: builder.card_builder.supertypes_ref().to_vec(),
            card_types: builder.card_builder.card_types_ref().to_vec(),
            subtypes: builder.card_builder.subtypes_ref().to_vec(),
            other_face_name: None,
        },
        ParseFeatures {
            allow_unsupported,
            preserve_reminder_text: false,
            capture_trace: parse_trace::is_enabled(),
        },
    );
    context.replace_provenance(ProvenanceStore::capture(SourceUnitId(0), text, &card_name));
    context
}

pub fn compile_card_text_with_policy(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<facade::CompiledCardText<CardDefinition>, CardTextError> {
    let text = text.into();
    stack::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, move || {
        let mut context = parse_context_for_builder(&builder, &text, allow_unsupported);
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

pub use facade::{
    CompilePolicy, CompiledCardText, CompilerBackend, CompilerCompileRequest, CompilerFacade,
    CompilerSourceDocument,
};
pub use oracle_grammar::{
    OracleGrammarDocument, OracleGrammarLevelItem, OracleGrammarLine, OracleGrammarLineInfo,
    OracleGrammarMode, parse_oracle_grammar_document,
};
pub use pipeline::{LoweringPipeline, PostpassProcessor};

#[cfg(test)]
#[path = "../../ironsmith-compiler/src/tests/mod.rs"]
mod tests;
