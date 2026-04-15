use std::collections::{HashMap, HashSet};
use std::fs;

use serde_json::Value;

use ironsmith::CardDefinitionBuilder;
use ironsmith::compiled_text::compiled_lines;
use ironsmith::ids::CardId;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <names-file> <cards.json> <out-file>",
            args.get(0)
                .map(|s| s.as_str())
                .unwrap_or("dump_false_positive_texts")
        );
        std::process::exit(2);
    }
    let names_path = &args[1];
    let cards_path = &args[2];
    let out_path = &args[3];

    let names = read_name_set(names_path)?;
    if names.is_empty() {
        eprintln!("No names found in {}", names_path);
        std::process::exit(2);
    }

    let raw = fs::read_to_string(cards_path)?;
    let cards: Vec<Value> = serde_json::from_str(&raw)?;

    let mut inputs: HashMap<String, CardInput> = HashMap::new();
    for card in &cards {
        let Some(input) = build_parse_input(card) else {
            continue;
        };
        if names.contains(&input.name) {
            inputs.insert(input.name.clone(), input);
        }
    }

    let mut missing = Vec::new();
    let mut out = String::new();
    for name in sorted_names(&names) {
        let Some(input) = inputs.get(&name) else {
            missing.push(name);
            continue;
        };
        let parse_result = CardDefinitionBuilder::new(CardId::new(), &input.name)
            .parse_text(input.parse_input.clone());
        out.push_str(&format!("Name: {}\n", input.name));
        out.push_str("Oracle:\n");
        out.push_str(&input.oracle_text);
        out.push('\n');
        out.push_str("Compiled:\n");
        match parse_result {
            Ok(definition) => {
                let lines = compiled_lines(&definition);
                if lines.is_empty() {
                    out.push_str("<empty>\n");
                } else {
                    out.push_str(&lines.join("\n"));
                    out.push('\n');
                }
            }
            Err(err) => {
                out.push_str(&format!("<parse error: {err:?}>\n"));
            }
        }
        out.push('\n');
    }

    if !missing.is_empty() {
        out.push_str("Missing names:\n");
        for name in missing {
            out.push_str(&format!("- {name}\n"));
        }
    }

    fs::write(out_path, out)?;
    Ok(())
}

#[derive(Clone)]
struct CardInput {
    name: String,
    oracle_text: String,
    parse_input: String,
}

fn read_name_set(path: &str) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            if let Some(rest) = line.strip_prefix("Name:") {
                let name = rest.trim();
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            } else if line.contains(':') {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect())
}

fn sorted_names(names: &HashSet<String>) -> Vec<String> {
    let mut list: Vec<String> = names.iter().cloned().collect();
    list.sort();
    list
}

fn pick_field<'a>(card: &'a Value, face: Option<&'a Value>, key: &str) -> Option<&'a str> {
    card.get(key)
        .and_then(Value::as_str)
        .or_else(|| face.and_then(|f| f.get(key)).and_then(Value::as_str))
}

fn has_acorn(card: &Value) -> bool {
    card.get("has_acorn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn legalities_all_not_legal(card: &Value) -> bool {
    let Some(legalities) = card.get("legalities").and_then(Value::as_object) else {
        return false;
    };
    !legalities.is_empty()
        && legalities
            .values()
            .all(|value| value.as_str().is_some_and(|item| item == "not_legal"))
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

fn is_non_playable(type_line: Option<&str>, oracle_text: Option<&str>, card: &Value) -> bool {
    if card
        .get("border_color")
        .and_then(Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("silver"))
    {
        return true;
    }
    if has_acorn(card) {
        return true;
    }
    if legalities_all_not_legal(card) {
        return true;
    }
    if card
        .get("layout")
        .and_then(Value::as_str)
        .is_some_and(|layout| {
            matches!(
                layout.to_ascii_lowercase().as_str(),
                "token"
                    | "double_faced_token"
                    | "emblem"
                    | "planar"
                    | "scheme"
                    | "vanguard"
                    | "art_series"
                    | "reversible_card"
            )
        })
    {
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

fn build_parse_input(card: &Value) -> Option<CardInput> {
    let face = card
        .get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.first());

    let name = pick_field(card, face, "name")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mana_cost = pick_field(card, face, "mana_cost");
    let type_line = pick_field(card, face, "type_line");
    let oracle_text = pick_field(card, face, "oracle_text");
    if is_non_playable(type_line, oracle_text, card) {
        return None;
    }
    let oracle_text = oracle_text?.trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }

    let power = pick_field(card, face, "power");
    let toughness = pick_field(card, face, "toughness");
    let loyalty = pick_field(card, face, "loyalty");
    let defense = pick_field(card, face, "defense");

    let mut lines = Vec::new();
    if let Some(mana_cost) = mana_cost.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Mana cost: {}", mana_cost.trim()));
    }
    if let Some(type_line) = type_line.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Type: {}", type_line.trim()));
    }
    if let (Some(power), Some(toughness)) = (power, toughness) {
        if !power.trim().is_empty() && !toughness.trim().is_empty() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                power.trim(),
                toughness.trim()
            ));
        }
    }
    if let Some(loyalty) = loyalty.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Loyalty: {}", loyalty.trim()));
    }
    if let Some(defense) = defense.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Defense: {}", defense.trim()));
    }
    lines.push(oracle_text.clone());

    Some(CardInput {
        name,
        oracle_text,
        parse_input: lines.join("\n"),
    })
}
