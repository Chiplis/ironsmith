use ironsmith_tools::{
    CardPayload, CardStatusDb, compile_snapshot_from_payload, default_db_path, load_canonical_cards,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
struct Args {
    cards_path: Option<String>,
    db_path: String,
    tag: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = None;
    let mut db_path = default_db_path().display().to_string();
    let mut tag = None;

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = Some(
                    iter.next()
                        .ok_or_else(|| "--cards requires a path".to_string())?,
                );
            }
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--tag" => {
                tag = Some(
                    iter.next()
                        .ok_or_else(|| "--tag requires a tag slug".to_string())?,
                );
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --release -p ironsmith-tools --bin sync_card_status_db -- [--db-path <path>] [--tag <slug>] [--cards <path>]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --cards/--db-path/--tag"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        db_path,
        tag,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let mut db = CardStatusDb::open(&args.db_path)?;
    let strict_scores_before = db.latest_strict_compiled_scores()?;
    let cards = if let Some(cards_path) = args.cards_path.as_deref() {
        load_canonical_cards(cards_path)?
            .into_values()
            .collect::<Vec<CardPayload>>()
    } else {
        let rows = db.registry_card_payloads()?;
        if rows.is_empty() {
            return Err(format!(
                "no registry_card rows found in {}; run sync_registry_db first or pass --cards",
                args.db_path
            )
            .into());
        }
        rows
    };
    let canonical_card_names = cards
        .iter()
        .map(|payload| payload.name.clone())
        .collect::<Vec<_>>();
    let tag_filtered_names = if let Some(tag) = args.tag.as_deref() {
        let names = db.card_names_for_tag(tag)?;
        if names.is_empty() {
            return Err(format!(
                "no card_tagging rows found for tag '{tag}' in {}",
                args.db_path
            )
            .into());
        }
        Some(names.into_iter().collect::<BTreeSet<_>>())
    } else {
        None
    };

    let mut inserted = 0usize;
    let mut processed = 0usize;
    for payload in &cards {
        let name = &payload.name;
        if let Some(filtered_names) = &tag_filtered_names
            && !filtered_names.contains(name)
        {
            continue;
        }
        processed += 1;
        let snapshot = compile_snapshot_from_payload(payload);
        if db.insert_snapshot_if_changed(&snapshot)? {
            inserted += 1;
        }
    }
    let pruned = if tag_filtered_names.is_none() {
        Some(db.prune_cards_not_in_names(&canonical_card_names)?)
    } else {
        None
    };
    let strict_scores_after = db.latest_strict_compiled_scores()?;
    let score_delta_summary = summarize_score_deltas(&strict_scores_before, &strict_scores_after);

    println!("Card status DB sync complete");
    println!("- Canonical cards processed: {processed}");
    println!("- New compilation rows inserted: {inserted}");
    println!(
        "- Strict-compiled semantic score avg before: {} across {} cards",
        format_average(strict_scores_before.values().copied()),
        strict_scores_before.len()
    );
    println!(
        "- Strict-compiled semantic score avg after: {} across {} cards",
        format_average(strict_scores_after.values().copied()),
        strict_scores_after.len()
    );
    println!(
        "- Cards with increased strict-compiled score: {} (avg {})",
        score_delta_summary.increased_count,
        format_signed_average(&score_delta_summary.increased_deltas)
    );
    println!(
        "- Cards with decreased strict-compiled score: {} (avg {})",
        score_delta_summary.decreased_count,
        format_signed_average(&score_delta_summary.decreased_deltas)
    );
    if let Some(tag) = &args.tag {
        println!("- Tag filter: {tag}");
        println!("- DB pruning skipped: yes");
    } else if let Some(pruned) = pruned {
        println!("- Cards removed from DB: {}", pruned.distinct_cards_deleted);
        println!(
            "- Compilation rows deleted: {}",
            pruned.compilation_rows_deleted
        );
        println!("- Tag rows deleted: {}", pruned.tag_rows_deleted);
    }
    println!("- DB: {}", args.db_path);

    Ok(())
}

#[derive(Debug, Default)]
struct ScoreDeltaSummary {
    increased_count: usize,
    decreased_count: usize,
    increased_deltas: Vec<f32>,
    decreased_deltas: Vec<f32>,
}

fn summarize_score_deltas(
    before: &BTreeMap<String, f32>,
    after: &BTreeMap<String, f32>,
) -> ScoreDeltaSummary {
    let mut summary = ScoreDeltaSummary::default();
    for (card_name, after_score) in after {
        let Some(before_score) = before.get(card_name) else {
            continue;
        };
        let delta = after_score - before_score;
        if delta > 0.0 {
            summary.increased_count += 1;
            summary.increased_deltas.push(delta);
        } else if delta < 0.0 {
            summary.decreased_count += 1;
            summary.decreased_deltas.push(delta);
        }
    }
    summary
}

fn format_average(values: impl IntoIterator<Item = f32>) -> String {
    match average(values) {
        Some(value) => format!("{value:.4}"),
        None => "n/a".to_string(),
    }
}

fn format_signed_average(values: &[f32]) -> String {
    match average(values.iter().copied()) {
        Some(value) => format!("{value:+.4}"),
        None => "n/a".to_string(),
    }
}

fn average(values: impl IntoIterator<Item = f32>) -> Option<f32> {
    let mut total = 0.0f32;
    let mut count = 0usize;
    for value in values {
        total += value;
        count += 1;
    }
    (count > 0).then_some(total / count as f32)
}
