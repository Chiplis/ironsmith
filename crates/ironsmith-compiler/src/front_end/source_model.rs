/// Metadata lines that sit above rules text in the compiler input model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

/// A normalized compiler line plus its source mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedLine {
    pub original: String,
    pub normalized: String,
    pub char_map: Vec<usize>,
}

/// Per-line source information carried through the compiler pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineInfo {
    pub line_index: usize,
    pub display_line_index: usize,
    pub raw_line: String,
    pub normalized: NormalizedLine,
}
