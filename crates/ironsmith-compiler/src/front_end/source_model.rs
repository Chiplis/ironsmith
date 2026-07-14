/// Metadata lines that sit above rules text in the compiler input model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    FirstPrintedSet(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

impl From<crate::runtime_backend::shared_types::MetadataLine> for MetadataLine {
    fn from(value: crate::runtime_backend::shared_types::MetadataLine) -> Self {
        match value {
            crate::runtime_backend::shared_types::MetadataLine::ManaCost(text) => {
                Self::ManaCost(text)
            }
            crate::runtime_backend::shared_types::MetadataLine::TypeLine(text) => {
                Self::TypeLine(text)
            }
            crate::runtime_backend::shared_types::MetadataLine::FirstPrintedSet(text) => {
                Self::FirstPrintedSet(text)
            }
            crate::runtime_backend::shared_types::MetadataLine::PowerToughness(text) => {
                Self::PowerToughness(text)
            }
            crate::runtime_backend::shared_types::MetadataLine::Loyalty(text) => {
                Self::Loyalty(text)
            }
            crate::runtime_backend::shared_types::MetadataLine::Defense(text) => {
                Self::Defense(text)
            }
        }
    }
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
