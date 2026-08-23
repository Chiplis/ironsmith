#![expect(clippy::type_complexity)]
#![allow(dead_code, unused_imports)]

//! Shared lexical grammar primitives and typed leaf recognizers.

pub use ironsmith_compiler_semantic::model;
pub use ironsmith_compiler_semantic::*;
pub use ironsmith_compiler_syntax::lexer;

pub mod util {
    use ironsmith_compiler_api::TextSpan;
    use ironsmith_compiler_semantic::color::ColorSet;
    use ironsmith_compiler_semantic::target::{SacrificedObjectKind, SourceReferenceSurface};
    use ironsmith_compiler_semantic::types::{CardType, Subtype};
    use ironsmith_compiler_semantic::zone::Zone;

    pub fn parser_trace_enabled() -> bool {
        std::env::var_os("IRONSMITH_PARSER_TRACE").is_some()
    }

    pub fn is_article(word: &str) -> bool {
        matches!(word, "a" | "an" | "the")
    }

    pub fn with_source_reference_context<T>(_card_name: &str, f: impl FnOnce() -> T) -> T {
        f()
    }

    pub fn current_source_reference_name() -> Option<String> {
        None
    }

    pub fn source_reference_surface_for_span(
        _span: Option<TextSpan>,
    ) -> Option<SourceReferenceSurface> {
        None
    }

    pub fn sacrificed_object_kind_for_span(
        _span: Option<TextSpan>,
    ) -> Option<SacrificedObjectKind> {
        None
    }

    pub fn this_source_surface_for_words(words: &[&str]) -> Option<SourceReferenceSurface> {
        crate::grammar::leaf::parse_leaf_this_source_reference_words(words)
    }

    pub fn is_source_reference_words(words: &[&str]) -> bool {
        this_source_surface_for_words(words).is_some()
    }

    pub fn parse_card_type(word: &str) -> Option<CardType> {
        crate::grammar::leaf::parse_leaf_card_type_complete(word).ok()
    }

    pub fn parse_color(word: &str) -> Option<ColorSet> {
        crate::grammar::leaf::parse_leaf_color_complete(word).ok()
    }

    pub fn parse_subtype_flexible(word: &str) -> Option<Subtype> {
        crate::grammar::leaf::parse_leaf_subtype_flexible_complete(word).ok()
    }

    pub fn parse_zone_word(word: &str) -> Option<Zone> {
        crate::grammar::leaf::parse_leaf_zone_complete(word).ok()
    }

    pub fn parse_number_word_i32(word: &str) -> Option<i32> {
        crate::grammar::leaf::parse_number_i32_complete(word).ok()
    }
}

#[path = "../../ironsmith-compiler/src/recognition.rs"]
pub mod recognition;

pub mod grammar;
pub use grammar::{leaf, lexical, primitives, targets};
