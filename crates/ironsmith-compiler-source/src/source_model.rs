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
    /// Builds a lossless normalized view from the compatibility character map.
    /// Each entry maps one normalized character to its source character index.
    pub fn from_char_map(
        original: impl Into<String>,
        normalized: impl Into<String>,
        mut char_map: Vec<usize>,
    ) -> Self {
        let original = original.into();
        let normalized = normalized.into();
        let source_boundaries = original
            .char_indices()
            .map(|(byte, _)| byte)
            .chain(std::iter::once(original.len()))
            .collect::<Vec<_>>();
        let normalized_chars = normalized.char_indices().collect::<Vec<_>>();

        if char_map.len() != normalized_chars.len() {
            char_map = normalized_chars
                .iter()
                .enumerate()
                .map(|(index, _)| index.min(source_boundaries.len().saturating_sub(2)))
                .collect();
        }

        let mut segments = Vec::with_capacity(normalized_chars.len());
        for (index, (normalized_start, ch)) in normalized_chars.iter().copied().enumerate() {
            let source_char = char_map[index].min(source_boundaries.len().saturating_sub(2));
            let source_start = source_boundaries.get(source_char).copied().unwrap_or(0);
            let default_source_end = source_boundaries
                .get(source_char + 1)
                .copied()
                .unwrap_or(original.len());
            let next_source_start = char_map
                .get(index + 1)
                .and_then(|next| source_boundaries.get(*next))
                .copied();
            let source_end = next_source_start
                .filter(|next| *next > source_start)
                .unwrap_or(default_source_end);
            segments.push(NormalizedSourceSegment {
                normalized_bytes: normalized_start..normalized_start + ch.len_utf8(),
                source_bytes: source_start..source_end,
            });
        }

        let mut covered = segments
            .iter()
            .map(|segment| segment.source_bytes.clone())
            .collect::<Vec<_>>();
        covered.sort_by_key(|range| range.start);
        let mut omitted_source_bytes = Vec::new();
        let mut cursor = 0;
        for range in covered {
            if cursor < range.start {
                omitted_source_bytes.push(cursor..range.start);
            }
            cursor = cursor.max(range.end);
        }
        if cursor < original.len() {
            omitted_source_bytes.push(cursor..original.len());
        }

        Self {
            original,
            normalized,
            char_map,
            source_map: NormalizedSourceMap {
                segments,
                omitted_source_bytes,
            },
        }
    }

    pub fn identity(text: impl Into<String>) -> Self {
        let text = text.into();
        let char_map = (0..text.chars().count()).collect();
        Self::from_char_map(text.clone(), text, char_map)
    }

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

#[cfg(test)]
mod tests {
    use super::NormalizedLine;

    #[test]
    fn identity_normalization_maps_each_character_back_to_source() {
        let line = NormalizedLine::identity("Draŵ.");

        assert_eq!(line.reconstruct_source(), "Draŵ.");
        assert_eq!(line.source_slice(0..6), Some("Draŵ."));
        assert!(line.source_map.omitted_source_bytes.is_empty());
    }

    #[test]
    fn compatibility_character_map_preserves_collapsed_source_ranges() {
        let line =
            NormalizedLine::from_char_map("Draw   two", "Draw two", vec![0, 1, 2, 3, 4, 7, 8, 9]);

        assert_eq!(line.source_slice(4..5), Some("   "));
        assert_eq!(line.source_slice(5..8), Some("two"));
        assert!(line.source_map.omitted_source_bytes.is_empty());
    }
}
