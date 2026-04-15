use ironsmith_tools::{CardStatusDb, default_db_path};

#[derive(Debug)]
struct Args {
    db_path: String,
}

fn parse_args() -> Result<Args, String> {
    let mut db_path = default_db_path().display().to_string();

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --release -p ironsmith-tools --bin cleanup_compilation_history -- [--db-path <path>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!("unknown argument '{arg}'. expected --db-path"));
            }
        }
    }

    Ok(Args { db_path })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let mut db = CardStatusDb::open(&args.db_path)?;
    let summary = db.prune_compilation_history_to_latest()?;

    println!("Card compilation history cleanup complete");
    println!("- Cards retained: {}", summary.distinct_cards_retained);
    println!(
        "- Historical compilation rows deleted: {}",
        summary.compilation_rows_deleted
    );
    println!("- DB: {}", args.db_path);

    Ok(())
}
