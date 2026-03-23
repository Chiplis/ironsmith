use ironsmith_tools::{
    CardStatusDb, compile_snapshot_from_payload, default_db_path, load_canonical_cards,
};

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
                    "usage: cargo run -p ironsmith-tools --bin sync_card_status_db -- [--cards <path>] [--db-path <path>]"
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
    let cards = load_canonical_cards(&args.cards_path)?;
    let db = CardStatusDb::open(&args.db_path)?;

    let mut inserted = 0usize;
    for payload in cards.values() {
        let snapshot = compile_snapshot_from_payload(payload);
        if db.insert_snapshot_if_changed(&snapshot)? {
            inserted += 1;
        }
    }

    println!("Card status DB sync complete");
    println!("- Canonical cards processed: {}", cards.len());
    println!("- New compilation rows inserted: {inserted}");
    println!("- DB: {}", args.db_path);

    Ok(())
}
