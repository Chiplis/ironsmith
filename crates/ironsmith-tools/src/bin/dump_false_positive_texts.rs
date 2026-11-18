use std::collections::{HashMap, HashSet};
use std::fs;

use ironsmith_card_source::{
    basic_metadata_lines, get_first_face, reports::excludes_from_paper_report,
};
use serde_json::Value;

use ironsmith::compiled_text::unprocessed_compiled_lines;
use ironsmith_tools::parse_card_definition_with_runtime_builder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <names-file> <cards.json> <out-file>",
            args.first()
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
        let parse_result = parse_card_definition_with_runtime_builder(
            &input.name,
            input.parse_input.clone(),
            false,
        );
        out.push_str(&format!("Name: {}\n", input.name));
        out.push_str("Oracle:\n");
        out.push_str(&input.oracle_text);
        out.push('\n');
        out.push_str("Compiled:\n");
        match parse_result {
            Ok(definition) => {
                let lines = unprocessed_compiled_lines(&definition);
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
    let layout = card
        .get("layout")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    excludes_from_paper_report(layout.as_deref(), type_line, oracle_text)
}

fn build_parse_input(card: &Value) -> Option<CardInput> {
    let face = get_first_face(card);

    let name = pick_field(card, face, "name")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let type_line = pick_field(card, face, "type_line");
    let oracle_text = pick_field(card, face, "oracle_text");
    if is_non_playable(type_line, oracle_text, card) {
        return None;
    }
    let oracle_text = oracle_text?.trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }

    let mut lines = basic_metadata_lines(|key| {
        pick_field(card, face, key).map(|value| value.trim().to_owned())
    });
    lines.push(oracle_text.clone());

    Some(CardInput {
        name,
        oracle_text,
        parse_input: lines.join("\n"),
    })
}
