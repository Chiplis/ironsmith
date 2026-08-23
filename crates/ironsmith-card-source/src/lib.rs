//! Lightweight Scryfall card-source loading for build and data-maintenance tools.
//!
//! This crate intentionally has no dependency on the compiler, runtime, registry,
//! or canonical text renderer. Keep it on the data-preflight side of the graph.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

const SUPPORTED_PAPER_FORMATS: &[&str] = &[
    "commander",
    "standard",
    "modern",
    "pioneer",
    "legacy",
    "vintage",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryCardRecord {
    pub name: String,
    pub oracle_text: String,
    pub raw_oracle_text: String,
    pub parse_input: String,
    pub raw_card_json: String,
    pub mana_cost: Option<String>,
    pub type_line: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub loyalty: Option<String>,
    pub defense: Option<String>,
    pub layout: Option<String>,
    pub content_hash: String,
}

pub fn default_cards_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("cards.json")
}

pub fn normalize_lookup_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.contains(" / ") && !trimmed.contains(" // ") {
        return trimmed.replacen(" / ", " // ", 1);
    }
    trimmed.to_string()
}

pub fn strip_parenthetical_text(text: &str) -> String {
    text.lines()
        .map(strip_parenthetical_text_from_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_parenthetical_text_from_line(line: &str) -> String {
    let mut stripped = String::with_capacity(line.len());
    let mut depth = 0usize;
    for ch in line.chars() {
        match ch {
            '(' => {
                if depth == 0 {
                    while stripped.ends_with([' ', '\t']) {
                        stripped.pop();
                    }
                }
                depth += 1;
            }
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => stripped.push(ch),
            _ => {}
        }
    }
    tighten_spacing(&stripped)
}

fn tighten_spacing(line: &str) -> String {
    let collapsed = line
        .replace('\u{00a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut tightened = String::with_capacity(collapsed.len());
    for ch in collapsed.chars() {
        if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?') && tightened.ends_with(' ') {
            tightened.pop();
        }
        tightened.push(ch);
    }
    tightened.trim().to_string()
}

pub fn load_registry_cards_with_explicit_includes_and_cards(
    path: &str,
    included_names: &BTreeSet<String>,
    extra_cards: Vec<Value>,
) -> Result<BTreeMap<String, RegistryCardRecord>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let mut cards: Vec<Value> = serde_json::from_str(&raw)?;
    let mut included_names = included_names
        .iter()
        .map(|name| normalize_lookup_name(name))
        .collect::<BTreeSet<_>>();
    for card in extra_cards {
        if let Some(name) = supplemental_card_name(&card) {
            included_names.insert(name);
        }
        cards.push(card);
    }

    let mut out = BTreeMap::new();
    for card in cards {
        let Some(record) = build_registry_card_record(&card, &included_names) else {
            continue;
        };
        out.entry(record.name.clone()).or_insert(record);
    }
    Ok(out)
}

fn supplemental_card_name(card: &Value) -> Option<String> {
    let name = normalize_lookup_name(&pick_field(card, get_first_face(card), "name")?);
    (!name.is_empty()).then_some(name)
}

fn build_registry_card_record(
    card: &Value,
    included_names: &BTreeSet<String>,
) -> Option<RegistryCardRecord> {
    if card
        .get("digital")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let face = get_first_face(card);
    let name = normalize_lookup_name(&pick_field(card, face, "name")?);
    if name.is_empty()
        || (!card_is_legal_in_supported_paper_format(card) && !included_names.contains(&name))
    {
        return None;
    }

    let raw_oracle_text = pick_field(card, face, "oracle_text")
        .unwrap_or_default()
        .trim()
        .to_string();
    let oracle_text = strip_parenthetical_text(&raw_oracle_text);
    let mana_cost = nonempty(pick_field_preferring_face(card, face, "mana_cost"));
    let type_line = nonempty(pick_field_preferring_face(card, face, "type_line"));
    let power = nonempty(pick_field_preferring_face(card, face, "power"));
    let toughness = nonempty(pick_field_preferring_face(card, face, "toughness"));
    let loyalty = nonempty(pick_field_preferring_face(card, face, "loyalty"));
    let defense = nonempty(pick_field_preferring_face(card, face, "defense"));

    let mut metadata_lines = Vec::new();
    if let Some(value) = &mana_cost {
        metadata_lines.push(format!("Mana cost: {value}"));
    }
    if let Some(value) = &type_line {
        metadata_lines.push(format!("Type: {value}"));
    }
    if let Some(value) = card
        .get("first_printed_set_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        metadata_lines.push(format!("First printed set: {value}"));
    }
    if let Some(lights) = face
        .and_then(|value| value.get("attraction_lights"))
        .or_else(|| card.get("attraction_lights"))
        .and_then(Value::as_array)
        .map(|lights| {
            lights
                .iter()
                .filter_map(Value::as_u64)
                .map(|light| light.to_string())
                .collect::<Vec<_>>()
        })
        .filter(|lights| !lights.is_empty())
    {
        metadata_lines.push(format!("Attraction lights: {}", lights.join(", ")));
    }
    if let (Some(power), Some(toughness)) = (&power, &toughness) {
        metadata_lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(value) = &loyalty {
        metadata_lines.push(format!("Loyalty: {value}"));
    }
    if let Some(value) = &defense {
        metadata_lines.push(format!("Defense: {value}"));
    }

    let parse_input = build_parse_input(&metadata_lines, &raw_oracle_text);
    let layout = nonempty(
        card.get("layout")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    );
    let raw_card_json = serde_json::to_string(card).ok()?;
    let content_hash = registry_card_content_hash(
        &name,
        &oracle_text,
        &raw_oracle_text,
        &parse_input,
        &raw_card_json,
        layout.as_deref(),
    );

    Some(RegistryCardRecord {
        name,
        oracle_text,
        raw_oracle_text,
        parse_input,
        raw_card_json,
        mana_cost,
        type_line,
        power,
        toughness,
        loyalty,
        defense,
        layout,
        content_hash,
    })
}

fn build_parse_input(metadata_lines: &[String], oracle_text: &str) -> String {
    let mut lines = metadata_lines.to_vec();
    if !oracle_text.trim().is_empty() {
        lines.push(oracle_text.trim().to_string());
    }
    lines.join("\n")
}

fn registry_card_content_hash(
    name: &str,
    oracle_text: &str,
    raw_oracle_text: &str,
    parse_input: &str,
    raw_card_json: &str,
    layout: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    for value in [name, oracle_text, raw_oracle_text, parse_input] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    hasher.update(layout.unwrap_or("").as_bytes());
    hasher.update([0]);
    hasher.update(raw_card_json.as_bytes());
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn card_is_legal_in_supported_paper_format(card: &Value) -> bool {
    let Some(legalities) = card.get("legalities").and_then(Value::as_object) else {
        return true;
    };
    legalities.is_empty()
        || SUPPORTED_PAPER_FORMATS.iter().any(|format| {
            legalities
                .get(*format)
                .and_then(Value::as_str)
                .is_some_and(|status| status == "legal")
        })
}

fn get_first_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.first())
}

fn value_to_string(value: &Value) -> Option<String> {
    if value.is_null() {
        None
    } else if let Some(value) = value.as_str() {
        Some(value.to_string())
    } else {
        Some(value.to_string())
    }
}

fn pick_field(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    card.get(key).and_then(value_to_string).or_else(|| {
        face.and_then(|value| value.get(key))
            .and_then(value_to_string)
    })
}

fn pick_field_preferring_face(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    face.and_then(|value| value.get(key))
        .and_then(value_to_string)
        .or_else(|| card.get(key).and_then(value_to_string))
}

fn nonempty(value: Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_split_separator() {
        assert_eq!(normalize_lookup_name("Fire / Ice"), "Fire // Ice");
    }

    #[test]
    fn strips_nested_reminder_text() {
        assert_eq!(
            strip_parenthetical_text("Flying (This (really) flies.)"),
            "Flying"
        );
    }
}
