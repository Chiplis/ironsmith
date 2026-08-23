#![expect(clippy::too_many_arguments, clippy::type_complexity)]

//! Scoped reference resolution and compiler-AST normalization.

pub use ironsmith_compiler_semantic::model;
pub use ironsmith_compiler_semantic::model::facts::NormalizedLine;
pub use ironsmith_compiler_semantic::model::visit::{
    assert_effect_ast_variant_coverage, for_each_nested_effects,
};
pub use ironsmith_compiler_semantic::*;
pub use ironsmith_core::{ObjectFilter, PlayerFilter, Value};

pub mod util {
    pub fn source_reference_surface_for_span(
        _span: Option<crate::diagnostics::TextSpan>,
    ) -> Option<crate::target::SourceReferenceSurface> {
        None
    }

    pub fn sacrificed_object_kind_for_span(
        _span: Option<crate::diagnostics::TextSpan>,
    ) -> Option<crate::target::SacrificedObjectKind> {
        None
    }
}

pub use ironsmith_compiler_semantic::model::visit as effect_ast_traversal;

pub fn map_span_to_original(
    span: diagnostics::TextSpan,
    normalized_line: &str,
    original_line: &str,
    char_map: &[usize],
) -> diagnostics::TextSpan {
    fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
        text[..byte_idx.min(text.len())].chars().count()
    }
    let start_char = byte_to_char_index(normalized_line, span.start);
    let end_char = byte_to_char_index(normalized_line, span.end);
    if start_char >= char_map.len() {
        return span;
    }
    let start_orig = char_map[start_char];
    let end_orig = if end_char == 0 || end_char > char_map.len() {
        start_orig
    } else {
        let last_orig = char_map[end_char - 1];
        last_orig
            + original_line[last_orig..]
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or(0)
    };
    diagnostics::TextSpan {
        line: span.line,
        start: start_orig,
        end: end_orig,
    }
}

#[path = "../../ironsmith-compiler/src/lowering/compile_support/tag_support.rs"]
pub mod tag_support;

pub mod compile_support {
    pub use crate::tag_support::*;
}

#[path = "../../ironsmith-compiler/src/model/effect_ast_normalization.rs"]
pub mod effect_ast_normalization;
#[path = "../../ironsmith-compiler/src/model/reference_helpers.rs"]
pub mod reference_helpers;
#[path = "../../ironsmith-compiler/src/model/reference_resolution.rs"]
pub mod reference_resolution;
