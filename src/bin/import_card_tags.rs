use std::collections::BTreeSet;

use ironsmith_tools::{
    CardStatusDb, TagImportRow, default_db_path, load_canonical_cards, normalize_lookup_name,
    read_tag_rows_from_research_csv_paths,
};

#[derive(Debug)]
struct Args {
    cards_path: String,
    db_path: String,
    csv_paths: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut db_path = default_db_path().display().to_string();
    let mut csv_paths = Vec::new();

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
            "--csv" => {
                csv_paths.push(
                    iter.next()
                        .ok_or_else(|| "--csv requires a path".to_string())?,
                );
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run -p ironsmith-tools --bin import_card_tags -- --csv <path> [--csv <path> ...] [--cards <path>] [--db-path <path>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --csv/--cards/--db-path"
                ));
            }
        }
    }

    if csv_paths.is_empty() {
        return Err("at least one --csv path is required".to_string());
    }

    Ok(Args {
        cards_path,
        db_path,
        csv_paths,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let allowed_cards = load_canonical_cards(&args.cards_path)?
        .into_keys()
        .collect::<BTreeSet<_>>();
    let mut rows: Vec<TagImportRow> = read_tag_rows_from_research_csv_paths(&args.csv_paths)?;
    let original_row_count = rows.len();
    rows.retain(|row| allowed_cards.contains(&normalize_lookup_name(&row.card_name)));
    let skipped_rows = original_row_count.saturating_sub(rows.len());
    let mut db = CardStatusDb::open(&args.db_path)?;
    let summary = db.replace_tag_rows(&rows)?;

    println!("Card tag import complete");
    println!("- CSV files processed: {}", args.csv_paths.len());
    println!("- Non-digital local cards allowed: {}", allowed_cards.len());
    println!("- Tags replaced: {}", summary.tags_replaced);
    println!("- Tag rows inserted: {}", summary.rows_inserted);
    println!(
        "- Rows skipped for digital/nonlocal cards: {}",
        skipped_rows
    );
    println!("- Cards source: {}", args.cards_path);
    println!("- DB: {}", args.db_path);

    Ok(())
}
