use crate::diagnostics::CardTextError;

use super::lexer::lex_line;
use super::source_model::{LineInfo, MetadataLine, NormalizedLine};

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
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    let char_map = build_char_map(trimmed, &normalized);

    Some(NormalizedLine {
        original: trimmed.to_string(),
        normalized,
        char_map,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetadataKind {
    ManaCost,
    TypeLine,
    PowerToughness,
    Loyalty,
    Defense,
}

fn build_char_map(original: &str, normalized: &str) -> Vec<usize> {
    if normalized.is_empty() {
        return Vec::new();
    }

    let original_chars: Vec<char> = original.chars().collect();
    let normalized_chars: Vec<char> = normalized.chars().collect();
    let mut map = Vec::with_capacity(normalized_chars.len());
    let mut original_idx = 0usize;

    for normalized_char in normalized_chars {
        while original_idx < original_chars.len()
            && original_chars[original_idx].is_whitespace()
            && normalized_char != ' '
        {
            original_idx += 1;
        }

        if normalized_char == ' ' {
            while original_idx < original_chars.len() && !original_chars[original_idx].is_whitespace()
            {
                original_idx += 1;
            }
            while original_idx < original_chars.len() && original_chars[original_idx].is_whitespace() {
                original_idx += 1;
            }
            map.push(original_idx.saturating_sub(1));
            continue;
        }

        map.push(original_idx);
        original_idx += 1;
    }

    map
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

        assert_eq!(normalized.original, "Draw   a\tcard.");
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
