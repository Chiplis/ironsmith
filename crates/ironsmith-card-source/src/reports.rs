//! Card input selection for the offline coverage reports.
use super::{basic_metadata_lines, get_first_face, pick_field};
use serde_json::Value;
use std::{collections::HashMap, error::Error, fs};

#[derive(Debug)]
pub struct ReportCardInput {
    pub parse_input: String,
    pub oracle_text: String,
}

impl ReportCardInput {
    fn from_card(card: &Value) -> Option<Self> {
        let face = get_first_face(card);
        let field = |key: &str| pick_field(card, face, key);
        let oracle_text = field("oracle_text").unwrap_or_default();
        let mut lines = basic_metadata_lines(field);
        if !oracle_text.is_empty() {
            lines.push(oracle_text.clone());
        }
        (!lines.is_empty()).then(|| Self {
            parse_input: lines.join("\n"),
            oracle_text,
        })
    }
}

/// Load the first printable record for each exact top-level name, retaining source whitespace.
pub fn load_report_card_inputs(
    path: &str,
) -> Result<HashMap<String, ReportCardInput>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let cards: Vec<Value> = serde_json::from_str(&raw)?;
    let mut inputs = HashMap::new();
    for card in cards {
        let Some(name) = card.get("name").and_then(Value::as_str) else {
            continue;
        };
        if inputs.contains_key(name) {
            continue;
        }
        if let Some(input) = ReportCardInput::from_card(&card) {
            inputs.insert(name.to_owned(), input);
        }
    }
    Ok(inputs)
}

fn contains_marker_with_boundaries(text: &str, marker: &str) -> bool {
    text.match_indices(marker).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let end = start + marker.len();
        let after = text[end..].chars().next();
        !before.is_some_and(|ch| ch.is_ascii_alphabetic())
            && !after.is_some_and(|ch| ch.is_ascii_alphabetic())
    })
}

fn has_digital_only_oracle_marker(oracle_text: &str) -> bool {
    let lower = oracle_text.to_ascii_lowercase();
    [
        "boon",
        "conjure",
        "double team",
        "draft",
        "heist",
        "incorporate",
        "intensity",
        "intensify",
        "perpetually",
        "seek",
        "specialize",
        "spellbook",
    ]
    .iter()
    .any(|marker| contains_marker_with_boundaries(&lower, marker))
}

pub fn excludes_from_paper_report(
    layout: Option<&str>,
    type_line: Option<&str>,
    oracle_text: Option<&str>,
) -> bool {
    if layout.is_some_and(|layout| {
        matches!(
            layout,
            "token"
                | "double_faced_token"
                | "emblem"
                | "planar"
                | "scheme"
                | "vanguard"
                | "art_series"
                | "reversible_card"
        )
    }) {
        return true;
    }
    if let Some(type_line) = type_line {
        let disallowed = [
            "Token",
            "Emblem",
            "Plane",
            "Scheme",
            "Vanguard",
            "Phenomenon",
            "Conspiracy",
            "Dungeon",
            "Attraction",
            "Contraption",
        ];
        if disallowed.iter().any(|needle| type_line.contains(needle)) {
            return true;
        }
        if type_line.trim() == "Card" {
            return true;
        }
    }
    if oracle_text.is_some_and(|oracle| oracle.contains("Theme color")) {
        return true;
    }
    if oracle_text.is_some_and(has_digital_only_oracle_marker) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryCardInput;

    #[test]
    fn registry_and_report_keep_their_face_precedence_and_whitespace() {
        let card = serde_json::json!({
            "name": "Front // Back", "mana_cost": "", "type_line": " Root ",
            "oracle_text": " Root text (gloss). ", "power": 1, "toughness": null,
            "card_faces": [{
                "mana_cost": "{W}", "type_line": " Face ", "oracle_text": "Face text",
                "color_indicator": ["R"], "power": 2, "toughness": " 3 "
            }]
        });
        let report = ReportCardInput::from_card(&card).unwrap();
        assert_eq!(report.oracle_text, " Root text (gloss). ");
        assert_eq!(
            report.parse_input,
            "Type:  Root \nPower/Toughness: 1/ 3 \n Root text (gloss). "
        );
        let registry = RegistryCardInput::from_card(&card, &Default::default())
            .unwrap()
            .record;
        assert_eq!(registry.oracle_text, "Root text.");
        assert_eq!(
            registry.parse_input,
            "Mana cost: {W}\nType: Face\nColor indicator: Red\nPower/Toughness: 2/3\nRoot text (gloss)."
        );
    }
}
