use std::collections::BTreeSet;
use std::fs;

use ironsmith_card_source::{
    default_cards_path, load_registry_cards_with_explicit_includes_and_cards,
};
use ironsmith_status_db::{StatusDb, default_db_path};
use serde_json::Value;

#[derive(Debug)]
struct Args {
    cards_path: String,
    db_path: String,
    included_card_names: BTreeSet<String>,
    extra_cards: Vec<Value>,
    insert_missing_only: bool,
    inserted_names_out: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        cards_path: default_cards_path().display().to_string(),
        db_path: default_db_path().display().to_string(),
        included_card_names: BTreeSet::new(),
        extra_cards: Vec::new(),
        insert_missing_only: false,
        inserted_names_out: None,
    };
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => args.cards_path = required_value(&mut iter, "--cards")?,
            "--db-path" => args.db_path = required_value(&mut iter, "--db-path")?,
            "--include-card" => {
                let value = required_value(&mut iter, "--include-card")?;
                if !value.trim().is_empty() {
                    args.included_card_names.insert(value.trim().to_string());
                }
            }
            "--include-cards" => {
                let path = required_value(&mut iter, "--include-cards")?;
                let raw = fs::read_to_string(&path)
                    .map_err(|err| format!("failed to read --include-cards {path}: {err}"))?;
                args.included_card_names.extend(
                    raw.lines()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(ToOwned::to_owned),
                );
            }
            "--extra-card-json" => {
                let raw = required_value(&mut iter, "--extra-card-json")?;
                args.extra_cards.push(parse_extra_card_json(&raw)?);
            }
            "--extra-cards-json" => {
                let path = required_value(&mut iter, "--extra-cards-json")?;
                let raw = fs::read_to_string(&path)
                    .map_err(|err| format!("failed to read --extra-cards-json {path}: {err}"))?;
                let cards: Vec<Value> = serde_json::from_str(&raw)
                    .map_err(|err| format!("failed to parse --extra-cards-json {path}: {err}"))?;
                for card in &cards {
                    ensure_extra_card_object(card)?;
                }
                args.extra_cards.extend(cards);
            }
            "--insert-missing-only" => args.insert_missing_only = true,
            "--inserted-names-out" => {
                args.inserted_names_out = Some(required_value(&mut iter, "--inserted-names-out")?);
            }
            "-h" | "--help" => return Err(usage().to_string()),
            _ => return Err(format!("unknown argument '{arg}'. {}", usage())),
        }
    }
    Ok(args)
}

fn required_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn usage() -> &'static str {
    "usage: cargo run --release -p ironsmith-registry-sync --bin sync_registry_db -- [--cards <path>] [--db-path <path>] [--include-card <name>] [--include-cards <path>] [--extra-card-json <json-object>] [--extra-cards-json <json-array-path>] [--insert-missing-only] [--inserted-names-out <path>]"
}

fn parse_extra_card_json(raw: &str) -> Result<Value, String> {
    let card: Value = serde_json::from_str(raw)
        .map_err(|err| format!("failed to parse --extra-card-json: {err}"))?;
    ensure_extra_card_object(&card)?;
    Ok(card)
}

fn ensure_extra_card_object(card: &Value) -> Result<(), String> {
    card.is_object()
        .then_some(())
        .ok_or_else(|| "extra card JSON must be an object".to_string())
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

    let mut db = StatusDb::open(&args.db_path)?;
    if args.insert_missing_only {
        let summary =
            db.insert_missing_registry_cards(&cards.values().cloned().collect::<Vec<_>>())?;
        if let Some(path) = &args.inserted_names_out {
            if let Some(parent) = std::path::Path::new(path).parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)?;
            }
            let mut output = summary.inserted_names.join("\n");
            if !output.is_empty() {
                output.push('\n');
            }
            fs::write(path, output)?;
        }
        println!("Registry DB missing-card sync complete");
        println!("- Canonical cards processed: {}", cards.len());
        println!(
            "- Explicit card includes requested: {}",
            args.included_card_names.len()
        );
        println!("- Extra card records provided: {}", args.extra_cards.len());
        println!("- Registry rows inserted: {}", summary.inserted_names.len());
        println!("- Registry rows already present: {}", summary.unchanged);
        println!("- Registry rows updated: 0");
        println!("- Registry rows deleted: 0");
        if let Some(path) = &args.inserted_names_out {
            println!("- Inserted names file: {path}");
        }
        println!("- DB: {}", args.db_path);
        return Ok(());
    }

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
