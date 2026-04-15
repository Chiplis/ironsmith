use std::collections::BTreeSet;

use ironsmith_tools::{
    CardStatusDb, TAGGER_BASE_URL, TaggerClient, build_local_tag_rows, default_db_path,
    fetch_all_oracle_tag_card_names, load_canonical_cards,
};

#[derive(Debug)]
struct Args {
    cards_path: String,
    db_path: String,
    tagger_url: String,
    tags: Vec<String>,
    start: usize,
    limit: Option<usize>,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut db_path = default_db_path().display().to_string();
    let mut tagger_url = TAGGER_BASE_URL.to_string();
    let mut tags = Vec::new();
    let mut start = 1usize;
    let mut limit = None;

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
            "--tagger-url" => {
                tagger_url = iter
                    .next()
                    .ok_or_else(|| "--tagger-url requires a URL".to_string())?;
            }
            "--tag" => {
                tags.push(
                    iter.next()
                        .ok_or_else(|| "--tag requires a tag slug".to_string())?,
                );
            }
            "--start" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--start requires a number".to_string())?;
                start = raw
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --start value '{raw}'"))?;
                if start == 0 {
                    return Err("--start must be 1 or greater".to_string());
                }
            }
            "--limit" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                limit = Some(
                    raw.parse::<usize>()
                        .map_err(|_| format!("invalid --limit value '{raw}'"))?,
                );
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --release -p ironsmith-tools --bin sync_card_tagging -- [--cards <path>] [--db-path <path>] [--tagger-url <url>] [--tag <slug> ...] [--start <n>] [--limit <n>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --cards/--db-path/--tagger-url/--tag/--start/--limit"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        db_path,
        tagger_url,
        tags,
        start,
        limit,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let local_cards = load_canonical_cards(&args.cards_path)?;
    let local_card_names = local_cards.keys().cloned().collect::<BTreeSet<_>>();
    let mut db = CardStatusDb::open(&args.db_path)?;
    let mut tags = if args.tags.is_empty() {
        db.oracle_tags()?
    } else {
        args.tags
    };
    if tags.is_empty() {
        return Err("no oracle tags available to sync".into());
    }
    let total_available_tags = tags.len();
    let start_index = args.start.saturating_sub(1);
    if start_index >= total_available_tags {
        return Err(format!(
            "--start {} is beyond the available tag count {}",
            args.start, total_available_tags
        )
        .into());
    }
    tags = tags.into_iter().skip(start_index).collect();
    if let Some(limit) = args.limit {
        tags.truncate(limit);
    }

    let client = TaggerClient::open(&args.tagger_url)?;

    let mut total_remote_matches = 0usize;
    let mut total_local_rows = 0usize;
    let mut total_skipped_nonlocal = 0usize;
    let mut failed_tags = Vec::new();

    for (index, tag) in tags.iter().enumerate() {
        let absolute_position = start_index + index + 1;
        match fetch_all_oracle_tag_card_names(&client, tag) {
            Ok(tagged_cards) => {
                let rows = build_local_tag_rows(tag, &tagged_cards, &local_card_names);
                total_remote_matches += tagged_cards.len();
                total_local_rows += rows.len();
                total_skipped_nonlocal += tagged_cards.len().saturating_sub(rows.len());
                db.replace_tag_rows_for_tags(std::slice::from_ref(tag), &rows)?;
            }
            Err(error) => {
                eprintln!("[WARN] skipping tag #{absolute_position} '{tag}': {error}");
                failed_tags.push(tag.clone());
            }
        }

        if (index + 1) % 25 == 0 || index + 1 == tags.len() {
            println!(
                "[INFO] processed {}/{} tags (local rows so far: {}, skipped non-local matches: {}, failed tags: {})",
                absolute_position,
                total_available_tags,
                total_local_rows,
                total_skipped_nonlocal,
                failed_tags.len()
            );
        }
    }

    println!("Card tagging sync complete");
    println!("- Oracle tags attempted this run: {}", tags.len());
    println!("- Start position: {}", args.start);
    println!("- Remote card/tag matches seen: {}", total_remote_matches);
    println!("- Local card/tag rows written: {}", total_local_rows);
    println!("- Non-local matches skipped: {}", total_skipped_nonlocal);
    println!("- Failed tags skipped: {}", failed_tags.len());
    println!("- Cards source: {}", args.cards_path);
    println!("- Tagger: {}", args.tagger_url);
    println!("- DB: {}", args.db_path);

    Ok(())
}
