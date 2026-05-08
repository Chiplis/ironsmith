use std::collections::BTreeSet;
use std::fs;

use ironsmith_tools::{
    CardStatusDb, default_cards_path, default_db_path,
    load_registry_cards_with_explicit_includes_and_cards,
};
use serde_json::Value;

#[derive(Debug)]
struct Args {
    cards_path: String,
    db_path: String,
    included_card_names: BTreeSet<String>,
    extra_cards: Vec<Value>,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = default_cards_path().display().to_string();
    let mut db_path = default_db_path().display().to_string();
    let mut included_card_names = BTreeSet::new();
    let mut extra_cards = Vec::new();

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = iter
                    .next()
                    .ok_or_else(|| "--cards requires a path".to_string())?;
            }
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--include-card" => {
                let name = iter
                    .next()
                    .ok_or_else(|| "--include-card requires a card name".to_string())?;
                let name = name.trim();
                if !name.is_empty() {
                    included_card_names.insert(name.to_string());
                }
            }
            "--include-cards" => {
                let path = iter
                    .next()
                    .ok_or_else(|| "--include-cards requires a path".to_string())?;
                let raw = fs::read_to_string(&path)
                    .map_err(|err| format!("failed to read --include-cards {path}: {err}"))?;
                for line in raw.lines() {
                    let name = line.trim();
                    if !name.is_empty() {
                        included_card_names.insert(name.to_string());
                    }
                }
            }
            "--extra-card-json" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--extra-card-json requires a JSON object".to_string())?;
                let card = parse_extra_card_json(&raw)?;
                extra_cards.push(card);
            }
            "--extra-cards-json" => {
                let path = iter
                    .next()
                    .ok_or_else(|| "--extra-cards-json requires a path".to_string())?;
                let raw = fs::read_to_string(&path)
                    .map_err(|err| format!("failed to read --extra-cards-json {path}: {err}"))?;
                let cards: Vec<Value> = serde_json::from_str(&raw)
                    .map_err(|err| format!("failed to parse --extra-cards-json {path}: {err}"))?;
                for card in cards {
                    ensure_extra_card_object(&card)?;
                    extra_cards.push(card);
                }
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --release -p ironsmith-tools --bin sync_registry_db -- [--cards <path>] [--db-path <path>] [--include-card <name>] [--include-cards <path>] [--extra-card-json <json-object>] [--extra-cards-json <json-array-path>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --cards/--db-path/--include-card/--include-cards/--extra-card-json/--extra-cards-json"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        db_path,
        included_card_names,
        extra_cards,
    })
}

fn parse_extra_card_json(raw: &str) -> Result<Value, String> {
    let card: Value = serde_json::from_str(raw)
        .map_err(|err| format!("failed to parse --extra-card-json: {err}"))?;
    ensure_extra_card_object(&card)?;
    Ok(card)
}

fn ensure_extra_card_object(card: &Value) -> Result<(), String> {
    if card.is_object() {
        Ok(())
    } else {
        Err("extra card JSON must be an object".to_string())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let cards = load_registry_cards_with_explicit_includes_and_cards(
        &args.cards_path,
        &args.included_card_names,
        args.extra_cards.clone(),
    )?;
    if cards.is_empty() {
        return Err(format!("no canonical registry cards found in {}", args.cards_path).into());
    }

    let mut db = CardStatusDb::open(&args.db_path)?;
    let summary = db.replace_registry_cards(&cards.values().cloned().collect::<Vec<_>>())?;
    let prune = db.prune_cards_not_in_names(&cards.keys().cloned().collect::<Vec<_>>())?;

    println!("Registry DB sync complete");
    println!("- Canonical cards processed: {}", cards.len());
    println!(
        "- Explicit card includes requested: {}",
        args.included_card_names.len()
    );
    println!("- Extra card records provided: {}", args.extra_cards.len());
    println!("- Registry rows inserted: {}", summary.inserted);
    println!("- Registry rows updated: {}", summary.updated);
    println!("- Registry rows unchanged: {}", summary.unchanged);
    println!("- Registry rows deleted: {}", summary.deleted);
    println!(
        "- Compilation rows deleted while pruning: {}",
        prune.compilation_rows_deleted
    );
    println!(
        "- Tag rows deleted while pruning: {}",
        prune.tag_rows_deleted
    );
    println!("- DB: {}", args.db_path);

    Ok(())
}
