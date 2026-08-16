use std::ops::Range;

/// Metadata lines that sit above rules text in the compiler input model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    FirstPrintedSet(String),
    AttractionLights(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

impl From<crate::model::facts::MetadataLine> for MetadataLine {
    fn from(value: crate::model::facts::MetadataLine) -> Self {
        match value {
            crate::model::facts::MetadataLine::ManaCost(text) => Self::ManaCost(text),
            crate::model::facts::MetadataLine::TypeLine(text) => Self::TypeLine(text),
            crate::model::facts::MetadataLine::FirstPrintedSet(text) => Self::FirstPrintedSet(text),
            crate::model::facts::MetadataLine::AttractionLights(text) => {
                Self::AttractionLights(text)
            }
            crate::model::facts::MetadataLine::PowerToughness(text) => Self::PowerToughness(text),
            crate::model::facts::MetadataLine::Loyalty(text) => Self::Loyalty(text),
            crate::model::facts::MetadataLine::Defense(text) => Self::Defense(text),
        }
    }
}

/// One contiguous relationship between a normalized view and the authored
/// source. Both ranges are byte ranges and therefore remain safe for Unicode
/// slicing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedSourceSegment {
    pub normalized_bytes: Range<usize>,
    pub source_bytes: Range<usize>,
}

/// Lossless mapping for one normalized view. A normalization may collapse or
/// replace text, but it never discards the authored line: every normalized
/// segment points back to an exact source range and omitted source ranges are
/// retained explicitly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NormalizedSourceMap {
    pub segments: Vec<NormalizedSourceSegment>,
    pub omitted_source_bytes: Vec<Range<usize>>,
}

impl NormalizedSourceMap {
    pub fn source_range_for_normalized(
        &self,
        normalized_bytes: Range<usize>,
    ) -> Option<Range<usize>> {
        let mut matching = self.segments.iter().filter(|segment| {
            segment.normalized_bytes.start < normalized_bytes.end
                && normalized_bytes.start < segment.normalized_bytes.end
        });
        let first = matching.next()?;
        let mut source = first.source_bytes.clone();
        for segment in matching {
            source.start = source.start.min(segment.source_bytes.start);
            source.end = source.end.max(segment.source_bytes.end);
        }
        Some(source)
    }

    pub fn source_slice<'a>(
        &self,
        original: &'a str,
        normalized_bytes: Range<usize>,
    ) -> Option<&'a str> {
        original.get(self.source_range_for_normalized(normalized_bytes)?)
    }
}

/// A normalized compiler line plus an exact, reversible source view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedLine {
    pub original: String,
    pub normalized: String,
    /// Compatibility character map retained for callers that have not moved
    /// to byte-precise source segments yet.
    pub char_map: Vec<usize>,
    pub source_map: NormalizedSourceMap,
}

impl NormalizedLine {
    pub fn reconstruct_source(&self) -> &str {
        &self.original
    }

    pub fn source_slice(&self, normalized_bytes: Range<usize>) -> Option<&str> {
        self.source_map
            .source_slice(&self.original, normalized_bytes)
    }
}

/// Per-line source information carried through the compiler pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineInfo {
    pub line_index: usize,
    pub display_line_index: usize,
    pub raw_line: String,
    pub normalized: NormalizedLine,
}
