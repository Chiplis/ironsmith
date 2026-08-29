use crate::lexer::OwnedLexToken;
use crate::model::facts::LineSemanticFacts;
use ironsmith_compiler_source::NormalizedLine;

/// Source-rich line context owned only by document classification and grammar.
/// Conversion to the canonical semantic model deliberately drops text and
/// tokens; authored presentation remains available through `ParseContext`'s
/// provenance store.
#[derive(Debug, Clone)]
pub struct GrammarLineInfo {
    pub line_index: usize,
    pub display_line_index: usize,
    pub raw_line: String,
    pub source_tokens: Vec<OwnedLexToken>,
    pub normalized: NormalizedLine,
    pub semantic_facts: LineSemanticFacts,
}

impl GrammarLineInfo {
    pub fn semantic_info(&self) -> crate::model::facts::LineInfo {
        crate::model::facts::LineInfo {
            line_index: self.line_index,
            display_line_index: self.display_line_index,
            provenance: None,
        }
    }
}

pub type LineInfo = GrammarLineInfo;
