use std::env;
use std::fs;
use std::io::{self, Read};

use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::compiled_text::{compiled_lines, oracle_like_lines};
use ironsmith::ids::CardId;
use serde_json::Value;

#[derive(Debug, Clone)]
struct CardInput {
    name: String,
    oracle_text: String,
    metadata_lines: Vec<String>,
}

fn value_to_string(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(value) = value.as_str() {
        return Some(value.to_string());
    }
    Some(value.to_string())
}

fn get_first_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.first())
}

fn pick_field(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    if let Some(value) = card.get(key).and_then(value_to_string) {
        return Some(value);
    }
    face.and_then(|value| value.get(key))
        .and_then(value_to_string)
}

fn build_parse_input(metadata_lines: &[String], oracle_text: &str) -> String {
    let mut lines = metadata_lines.to_vec();
    if !oracle_text.trim().is_empty() {
        lines.push(oracle_text.trim().to_string());
    }
    lines.join("\n")
}

fn build_card_input(card: &Value) -> Option<CardInput> {
    let face = get_first_face(card);
    let name = pick_field(card, face, "name")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let oracle_text = pick_field(card, face, "oracle_text")?.trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }

    let mana_cost = pick_field(card, face, "mana_cost");
    let type_line = pick_field(card, face, "type_line");
    let power = pick_field(card, face, "power");
    let toughness = pick_field(card, face, "toughness");
    let loyalty = pick_field(card, face, "loyalty");
    let defense = pick_field(card, face, "defense");

    let mut metadata_lines = Vec::new();
    if let Some(mana_cost) = mana_cost.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Mana cost: {}", mana_cost.trim()));
    }
    if let Some(type_line) = type_line.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Type: {}", type_line.trim()));
    }
    if let (Some(power), Some(toughness)) = (power, toughness) {
        if !power.trim().is_empty() && !toughness.trim().is_empty() {
            metadata_lines.push(format!(
                "Power/Toughness: {}/{}",
                power.trim(),
                toughness.trim()
            ));
        }
    }
    if let Some(loyalty) = loyalty.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Loyalty: {}", loyalty.trim()));
    }
    if let Some(defense) = defense.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Defense: {}", defense.trim()));
    }

    Some(CardInput {
        name,
        oracle_text,
        metadata_lines,
    })
}

fn load_card_input_by_name(cards_path: &str, name: &str) -> Result<Option<CardInput>, String> {
    let contents = fs::read_to_string(cards_path)
        .map_err(|err| format!("failed to read cards file '{cards_path}': {err}"))?;
    let cards: Value = serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse cards file '{cards_path}': {err}"))?;
    let Some(entries) = cards.as_array() else {
        return Err(format!(
            "cards file '{cards_path}' is not a top-level JSON array"
        ));
    };

    Ok(entries.iter().find_map(|card| {
        let input = build_card_input(card)?;
        input
            .name
            .eq_ignore_ascii_case(name.trim())
            .then_some(input)
    }))
}

fn text_includes_metadata(text: &str) -> bool {
    text.lines().map(str::trim).any(|line| {
        line.starts_with("Mana cost:")
            || line.starts_with("Type:")
            || line.starts_with("Power/Toughness:")
            || line.starts_with("Loyalty:")
            || line.starts_with("Defense:")
    })
}

fn read_input_text(text_arg: Option<String>) -> Result<Option<String>, String> {
    if let Some(text) = text_arg {
        return Ok(Some(text));
    }
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("failed to read stdin: {err}"))?;
    if input.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(input))
}

fn main() -> Result<(), String> {
    let mut name = "Parser Probe".to_string();
    let mut cards_path = "cards.json".to_string();
    let mut text_arg: Option<String> = None;
    let mut stacktrace = false;
    let mut trace = false;
    let mut allow_unsupported = false;
    let mut detailed = false;
    let mut raw = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--name" => {
                name = args
                    .next()
                    .ok_or_else(|| "--name requires a value".to_string())?;
            }
            "--cards" => {
                cards_path = args
                    .next()
                    .ok_or_else(|| "--cards requires a value".to_string())?;
            }
            "--text" => {
                text_arg = Some(
                    args.next()
                        .ok_or_else(|| "--text requires a value".to_string())?,
                );
            }
            "--stacktrace" => {
                stacktrace = true;
            }
            "--trace" => {
                trace = true;
            }
            "--allow-unsupported" => {
                allow_unsupported = true;
            }
            "--detailed" => {
                detailed = true;
            }
            "--raw" => {
                raw = true;
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --name <value>, --cards <path>, --text <value>, --trace, --allow-unsupported, --detailed, --raw, and/or --stacktrace"
                ));
            }
        }
    }

    if trace {
        unsafe {
            env::set_var("IRONSMITH_PARSER_TRACE", "1");
        }
    }

    if stacktrace {
        unsafe {
            env::set_var("IRONSMITH_PARSER_STACKTRACE", "1");
        }
    }

    if allow_unsupported {
        unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", "1");
        }
    }

    let input_text = read_input_text(text_arg)?;
    let card_input = if name != "Parser Probe" {
        load_card_input_by_name(&cards_path, &name)?
    } else {
        None
    };

    let (name, oracle_text, parse_input) = match (input_text, card_input) {
        (Some(text), Some(card)) if !text_includes_metadata(&text) => {
            let parse_input = build_parse_input(&card.metadata_lines, &text);
            (card.name, text.trim().to_string(), parse_input)
        }
        (Some(text), Some(card)) => (card.name, card.oracle_text, text),
        (Some(text), None) => (name, text.clone(), text),
        (None, Some(card)) => {
            let parse_input = build_parse_input(&card.metadata_lines, &card.oracle_text);
            (card.name, card.oracle_text, parse_input)
        }
        (None, None) => {
            return Err(
                "missing oracle text (pass --text or stdin) and no matching card found via --name/--cards"
                    .to_string(),
            )
        }
    };

    let builder = CardDefinitionBuilder::new(CardId::new(), &name);
    let def = builder
        .parse_text(parse_input.clone())
        .map_err(|err| format!("parse failed: {err:?}"))?;

    println!("Name: {}", def.card.name);
    if detailed {
        println!("Oracle text:");
        println!("{}", oracle_text.trim());
        println!("Parse input:");
        println!("{}", parse_input.trim());
    }
    println!(
        "Type: {}",
        def.card
            .card_types
            .iter()
            .map(|t| format!("{t:?}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("Compiled abilities/effects");
    if raw {
        println!("- {:#?}", def);
        return Ok(());
    }
    let lines = if detailed {
        compiled_lines(&def)
    } else {
        oracle_like_lines(&def)
    };
    if lines.is_empty() {
        println!("- <none>");
    } else {
        for line in lines {
            println!("- {}", line.trim());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_parse_input_appends_oracle_text_after_metadata() {
        let parse_input = build_parse_input(
            &[
                "Mana cost: {U}{U}".to_string(),
                "Type: Creature — Merfolk Wizard".to_string(),
                "Power/Toughness: 1/3".to_string(),
            ],
            "When this creature enters, draw a card.",
        );

        assert_eq!(
            parse_input,
            "Mana cost: {U}{U}\nType: Creature — Merfolk Wizard\nPower/Toughness: 1/3\nWhen this creature enters, draw a card."
        );
    }

    #[test]
    fn metadata_detection_ignores_plain_oracle_lines() {
        assert!(!text_includes_metadata(
            "When Thassa's Oracle enters the battlefield, look at the top X cards of your library."
        ));
        assert!(text_includes_metadata(
            "Type: Creature — Merfolk Wizard\nWhen this creature enters, draw a card."
        ));
    }
}
