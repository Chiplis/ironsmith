use crate::diagnostics::CardTextError;

use super::lexer::lex_line;
use super::source_model::{
    LineInfo, MetadataLine, NormalizedLine, NormalizedSourceMap, NormalizedSourceSegment,
};

pub fn parse_metadata_line(line: &str) -> Result<Option<MetadataLine>, CardTextError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let Some((label, value)) = trimmed.split_once(':') else {
        return Ok(None);
    };

    let label_tokens = match lex_line(format!("{}:", label.trim()).as_str(), 0) {
        Ok(tokens) => tokens,
        Err(_) => return Ok(None),
    };

    let label_words = label_tokens
        .iter()
        .filter_map(|token| token.as_word())
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let kind = match label_words.as_slice() {
        [mana, cost] if mana == "mana" && cost == "cost" => Some(MetadataKind::ManaCost),
        [kind] if kind == "type" => Some(MetadataKind::TypeLine),
        [type_word, line_word] if type_word == "type" && line_word == "line" => {
            Some(MetadataKind::TypeLine)
        }
        [first, printed, set] if first == "first" && printed == "printed" && set == "set" => {
            Some(MetadataKind::FirstPrintedSet)
        }
        [attraction, lights] if attraction == "attraction" && lights == "lights" => {
            Some(MetadataKind::AttractionLights)
        }
        [pt] if pt == "power/toughness" => Some(MetadataKind::PowerToughness),
        [power, toughness] if power == "power" && toughness == "toughness" => {
            Some(MetadataKind::PowerToughness)
        }
        [loyalty] if loyalty == "loyalty" => Some(MetadataKind::Loyalty),
        [defense] if defense == "defense" => Some(MetadataKind::Defense),
        _ => None,
    };

    let value = value.trim().to_string();
    let metadata = match kind {
        Some(MetadataKind::ManaCost) => MetadataLine::ManaCost(value),
        Some(MetadataKind::TypeLine) => MetadataLine::TypeLine(value),
        Some(MetadataKind::FirstPrintedSet) => MetadataLine::FirstPrintedSet(value),
        Some(MetadataKind::AttractionLights) => MetadataLine::AttractionLights(value),
        Some(MetadataKind::PowerToughness) => MetadataLine::PowerToughness(value),
        Some(MetadataKind::Loyalty) => MetadataLine::Loyalty(value),
        Some(MetadataKind::Defense) => MetadataLine::Defense(value),
        None => return Ok(None),
    };

    Ok(Some(metadata))
}

pub fn make_line_info(
    line_index: usize,
    raw_line: impl Into<String>,
    normalized: NormalizedLine,
) -> LineInfo {
    LineInfo {
        line_index,
        display_line_index: line_index,
        raw_line: raw_line.into(),
        normalized,
    }
}

pub fn normalize_trimmed_line(line: &str) -> Option<NormalizedLine> {
    let source_start = line.find(|ch: char| !ch.is_whitespace())?;
    let source_end = line
        .rfind(|ch: char| !ch.is_whitespace())
        .and_then(|byte| line[byte..].chars().next().map(|ch| byte + ch.len_utf8()))?;
    if source_start >= source_end {
        return None;
    }

    let mut normalized = String::new();
    let mut char_map = Vec::new();
    let mut segments = Vec::new();
    let mut pending_whitespace = None;

    for (relative_byte, ch) in line[source_start..source_end].char_indices() {
        let source_byte = source_start + relative_byte;
        if ch.is_whitespace() {
            pending_whitespace.get_or_insert(source_byte);
            continue;
        }

        if let Some(whitespace_start) = pending_whitespace.take()
            && !normalized.is_empty()
        {
            let normalized_start = normalized.len();
            normalized.push(' ');
            segments.push(NormalizedSourceSegment {
                normalized_bytes: normalized_start..normalized.len(),
                source_bytes: whitespace_start..source_byte,
            });
            char_map.push(line[..whitespace_start].chars().count());
        }

        let normalized_start = normalized.len();
        normalized.push(ch);
        segments.push(NormalizedSourceSegment {
            normalized_bytes: normalized_start..normalized.len(),
            source_bytes: source_byte..source_byte + ch.len_utf8(),
        });
        char_map.push(line[..source_byte].chars().count());
    }

    let mut omitted_source_bytes = Vec::new();
    if source_start > 0 {
        omitted_source_bytes.push(0..source_start);
    }
    if source_end < line.len() {
        omitted_source_bytes.push(source_end..line.len());
    }

    Some(NormalizedLine {
        original: line.to_string(),
        normalized,
        char_map,
        source_map: NormalizedSourceMap {
            segments,
            omitted_source_bytes,
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataKind {
    ManaCost,
    TypeLine,
    FirstPrintedSet,
    AttractionLights,
    PowerToughness,
    Loyalty,
    Defense,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_metadata_line_recognizes_supported_labels() {
        assert!(matches!(
            parse_metadata_line("Mana Cost: {2}{W}"),
            Ok(Some(MetadataLine::ManaCost(value))) if value == "{2}{W}"
        ));
        assert!(matches!(
            parse_metadata_line("Type Line: Legendary Creature — Human"),
            Ok(Some(MetadataLine::TypeLine(value))) if value == "Legendary Creature — Human"
        ));
        assert!(matches!(
            parse_metadata_line("First printed set: Antiquities"),
            Ok(Some(MetadataLine::FirstPrintedSet(value))) if value == "Antiquities"
        ));
        assert!(matches!(
            parse_metadata_line("Power/Toughness: */*"),
            Ok(Some(MetadataLine::PowerToughness(value))) if value == "*/*"
        ));
        assert!(matches!(
            parse_metadata_line("Loyalty: 4"),
            Ok(Some(MetadataLine::Loyalty(value))) if value == "4"
        ));
        assert!(matches!(
            parse_metadata_line("Defense: 5"),
            Ok(Some(MetadataLine::Defense(value))) if value == "5"
        ));
    }

    #[test]
    fn normalize_trimmed_line_collapses_whitespace_and_tracks_chars() {
        let normalized = normalize_trimmed_line("  Draw   a\tcard. ").expect("normalized line");

        assert_eq!(normalized.original, "  Draw   a\tcard. ");
        assert_eq!(normalized.normalized, "Draw a card.");
        assert!(!normalized.char_map.is_empty());
    }

    #[test]
    fn make_line_info_preserves_indexes() {
        let normalized = normalize_trimmed_line("Draw a card.").expect("normalized line");
        let info = make_line_info(2, "Draw a card.", normalized);

        assert_eq!(info.line_index, 2);
        assert_eq!(info.display_line_index, 2);
        assert_eq!(info.raw_line, "Draw a card.");
    }
}
