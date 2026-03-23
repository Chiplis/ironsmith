use ironsmith_tools::{
    CardStatusDb, TagImportRow, default_db_path, read_tag_rows_from_research_csv_paths,
};

#[derive(Debug)]
struct Args {
    db_path: String,
    csv_paths: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut db_path = default_db_path().display().to_string();
    let mut csv_paths = Vec::new();

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
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
                    "usage: cargo run -p ironsmith-tools --bin import_card_tags -- --csv <path> [--csv <path> ...] [--db-path <path>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --csv/--db-path"
                ));
            }
        }
    }

    if csv_paths.is_empty() {
        return Err("at least one --csv path is required".to_string());
    }

    Ok(Args { db_path, csv_paths })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let rows: Vec<TagImportRow> = read_tag_rows_from_research_csv_paths(&args.csv_paths)?;
    let mut db = CardStatusDb::open(&args.db_path)?;
    let summary = db.replace_tag_rows(&rows)?;

    println!("Card tag import complete");
    println!("- CSV files processed: {}", args.csv_paths.len());
    println!("- Tags replaced: {}", summary.tags_replaced);
    println!("- Tag rows inserted: {}", summary.rows_inserted);
    println!("- DB: {}", args.db_path);

    Ok(())
}
