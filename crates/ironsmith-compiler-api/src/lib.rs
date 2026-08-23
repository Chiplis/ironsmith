//! Stable, low-level compiler diagnostics shared by parser leaves.

use std::collections::HashMap;

/// Span of source text within a line-oriented oracle text block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

impl TextSpan {
    /// Synthetic span used for compiler-generated nodes without a source anchor.
    pub fn synthetic() -> Self {
        Self {
            line: 0,
            start: 0,
            end: 0,
        }
    }
}

/// Compiler annotations captured while preparing or parsing card text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseAnnotations {
    pub tag_spans: HashMap<String, Vec<TextSpan>>,
    pub normalized_lines: HashMap<usize, String>,
    pub original_lines: HashMap<usize, String>,
    pub normalized_char_maps: HashMap<usize, Vec<usize>>,
}

impl ParseAnnotations {
    pub fn record_tag_span(&mut self, tag: impl Into<String>, span: TextSpan) {
        self.tag_spans.entry(tag.into()).or_default().push(span);
    }

    pub fn record_normalized_line(&mut self, line_index: usize, line: impl Into<String>) {
        self.normalized_lines
            .entry(line_index)
            .or_insert(line.into());
    }

    pub fn record_original_line(&mut self, line_index: usize, line: impl Into<String>) {
        self.original_lines.entry(line_index).or_insert(line.into());
    }

    pub fn record_char_map(&mut self, line_index: usize, map: Vec<usize>) {
        self.normalized_char_maps.entry(line_index).or_insert(map);
    }
}

/// Compiler-facing parse/validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardTextError {
    UnsupportedLine(String),
    ParseError(String),
    InvariantViolation(String),
}

impl std::fmt::Display for CardTextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedLine(message)
            | Self::ParseError(message)
            | Self::InvariantViolation(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for CardTextError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotations_and_synthetic_spans_are_stable() {
        let mut annotations = ParseAnnotations::default();
        annotations.record_tag_span(
            "it",
            TextSpan {
                line: 1,
                start: 3,
                end: 5,
            },
        );
        annotations.record_normalized_line(1, "draw a card");
        annotations.record_original_line(1, "Draw a card.");
        annotations.record_char_map(1, vec![0, 1, 2]);

        assert_eq!(annotations.tag_spans["it"][0].start, 3);
        assert_eq!(TextSpan::synthetic().end, 0);
    }
}
