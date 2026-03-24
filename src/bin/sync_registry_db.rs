use ironsmith_tools::{CardStatusDb, default_db_path, load_registry_cards};

#[derive(Debug)]
struct Args {
    cards_path: String,
    db_path: String,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut db_path = default_db_path().display().to_string();

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
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --release -p ironsmith-tools --bin sync_registry_db -- [--cards <path>] [--db-path <path>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --cards/--db-path"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        db_path,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let cards = load_registry_cards(&args.cards_path)?;
    if cards.is_empty() {
        return Err(format!("no canonical registry cards found in {}", args.cards_path).into());
    }

    let mut db = CardStatusDb::open(&args.db_path)?;
    let summary = db.replace_registry_cards(&cards.values().cloned().collect::<Vec<_>>())?;
    let prune = db.prune_cards_not_in_names(&cards.keys().cloned().collect::<Vec<_>>())?;

    println!("Registry DB sync complete");
    println!("- Canonical cards processed: {}", cards.len());
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
