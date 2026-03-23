use std::fs;

use ironsmith_tools::{
    CardStatusDb, SCRYFALL_TAGGER_TAGS_URL, default_db_path, fetch_functional_oracle_tags_from_url,
    read_functional_oracle_tags_from_html,
};

#[derive(Debug)]
struct Args {
    db_path: String,
    html_path: Option<String>,
    url: String,
}

fn parse_args() -> Result<Args, String> {
    let mut db_path = default_db_path().display().to_string();
    let mut html_path = None;
    let mut url = SCRYFALL_TAGGER_TAGS_URL.to_string();

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--html" => {
                html_path = Some(
                    iter.next()
                        .ok_or_else(|| "--html requires a path".to_string())?,
                );
            }
            "--url" => {
                url = iter
                    .next()
                    .ok_or_else(|| "--url requires a value".to_string())?;
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run -p ironsmith-tools --bin sync_oracle_tags -- [--db-path <path>] [--html <path> | --url <url>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --db-path/--html/--url"
                ));
            }
        }
    }

    Ok(Args {
        db_path,
        html_path,
        url,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let tags = if let Some(path) = &args.html_path {
        let html = fs::read_to_string(path)?;
        read_functional_oracle_tags_from_html(&html)?
    } else {
        fetch_functional_oracle_tags_from_url(&args.url)?
    };

    let mut db = CardStatusDb::open(&args.db_path)?;
    let summary = db.replace_oracle_tags(&tags)?;

    println!("Functional oracle tag sync complete");
    println!("- Tags discovered: {}", tags.len());
    println!("- Existing rows replaced: {}", summary.tags_replaced);
    println!("- Oracle tags inserted: {}", summary.rows_inserted);
    if let Some(path) = &args.html_path {
        println!("- Source HTML: {}", path);
    } else {
        println!("- Source URL: {}", args.url);
    }
    println!("- DB: {}", args.db_path);

    Ok(())
}
