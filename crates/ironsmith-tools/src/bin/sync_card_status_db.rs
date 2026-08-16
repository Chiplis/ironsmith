use ironsmith_tools::{
    CardPayload, CardStatusDb, CompilationSnapshot, compile_authoritative_snapshot_from_payload,
    compile_strict_snapshot_from_payload, default_db_path, load_canonical_cards,
};
use rayon::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::Mutex,
    time::{Duration, Instant},
};

const RAYON_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug)]
struct Args {
    cards_path: Option<String>,
    db_path: String,
    tag: Option<String>,
    names: BTreeSet<String>,
    strict_only: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = None;
    let mut db_path = default_db_path().display().to_string();
    let mut tag = None;
    let mut names = BTreeSet::new();
    let mut strict_only = false;

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
            "--name" => {
                let name = iter
                    .next()
                    .ok_or_else(|| "--name requires a card name".to_string())?;
                let normalized = ironsmith_tools::normalize_lookup_name(&name);
                if !normalized.is_empty() {
                    names.insert(normalized);
                }
            }
            "--names-file" => {
                let path = iter
                    .next()
                    .ok_or_else(|| "--names-file requires a path".to_string())?;
                let raw = fs::read_to_string(&path)
                    .map_err(|err| format!("failed to read --names-file {path}: {err}"))?;
                for line in raw.lines() {
                    let normalized = ironsmith_tools::normalize_lookup_name(line);
                    if !normalized.is_empty() {
                        names.insert(normalized);
                    }
                }
            }
            "--strict-only" => {
                strict_only = true;
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --release -p ironsmith-tools --bin sync_card_status_db -- [--db-path <path>] [--tag <slug>] [--name <card>] [--names-file <path>] [--cards <path>] [--strict-only]"
                        .to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --cards/--db-path/--tag/--name/--names-file/--strict-only"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        db_path,
        tag,
        names,
        strict_only,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let total_start = Instant::now();
    let args = parse_args().map_err(std::io::Error::other)?;

    let db_open_start = Instant::now();
    let mut db = CardStatusDb::open(&args.db_path)?;
    let db_open_elapsed = db_open_start.elapsed();

    let score_before_start = Instant::now();
    let strict_scores_before = db.latest_strict_compiled_scores()?;
    let score_before_elapsed = score_before_start.elapsed();

    let load_cards_start = Instant::now();
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
    let load_cards_elapsed = load_cards_start.elapsed();

    let filter_start = Instant::now();
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
    let name_filtered_names = if args.names.is_empty() {
        None
    } else {
        Some(args.names.clone())
    };

    let filtered_cards = cards
        .iter()
        .filter(|payload| {
            tag_filtered_names
                .as_ref()
                .is_none_or(|filtered_names| filtered_names.contains(&payload.name))
                && name_filtered_names
                    .as_ref()
                    .is_none_or(|filtered_names| filtered_names.contains(&payload.name))
        })
        .collect::<Vec<_>>();
    if let Some(filtered_names) = name_filtered_names.as_ref() {
        let found_names = filtered_cards
            .iter()
            .map(|payload| payload.name.clone())
            .collect::<BTreeSet<_>>();
        let missing_names = filtered_names
            .difference(&found_names)
            .cloned()
            .collect::<Vec<_>>();
        if !missing_names.is_empty() {
            return Err(format!(
                "card(s) requested with --name were not found in the selected card source: {}",
                missing_names.join(", ")
            )
            .into());
        }
    }
    let filter_elapsed = filter_start.elapsed();

    let processed = filtered_cards.len();
    let rayon_pool = rayon::ThreadPoolBuilder::new()
        .stack_size(RAYON_WORKER_STACK_SIZE)
        .build()?;
    let compile_start = Instant::now();
    let completed_cards = Mutex::new(0usize);
    let (compiled_snapshots, rayon_threads) = rayon_pool.install(|| {
        let snapshots = filtered_cards
            .par_iter()
            .map(|payload| {
                let snapshot = if args.strict_only {
                    compile_strict_snapshot_from_payload(payload)
                } else {
                    compile_authoritative_snapshot_from_payload(payload)
                };
                let mut completed = completed_cards
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                *completed += 1;
                eprintln!("{}", format_card_progress(*completed, processed, &snapshot));
                snapshot
            })
            .collect::<Vec<_>>();
        (snapshots, rayon::current_num_threads())
    });
    let compile_elapsed = compile_start.elapsed();

    let insert_start = Instant::now();
    let inserted = db.insert_snapshots_if_changed(&compiled_snapshots)?;
    let insert_elapsed = insert_start.elapsed();

    let prune_start = Instant::now();
    let pruned = if tag_filtered_names.is_none() && name_filtered_names.is_none() {
        Some(db.prune_cards_not_in_names(&canonical_card_names)?)
    } else {
        None
    };
    let prune_elapsed = prune_start.elapsed();

    let score_after_start = Instant::now();
    let strict_scores_after = db.latest_strict_compiled_scores()?;
    let score_delta_summary = summarize_score_deltas(&strict_scores_before, &strict_scores_after);
    let score_after_elapsed = score_after_start.elapsed();
    let total_elapsed = total_start.elapsed();

    println!("Card status DB sync complete");
    println!("- Canonical cards processed: {processed}");
    println!(
        "- Compile mode: {}",
        if args.strict_only {
            "strict-only"
        } else {
            "strict with allow-unsupported fallback"
        }
    );
    println!("- Rayon threads: {rayon_threads}");
    println!("- Rayon worker stack: {RAYON_WORKER_STACK_SIZE} bytes");
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
    if score_delta_summary.regressions.is_empty() {
        println!("- Regressed strict-compiled cards: none");
    } else {
        println!("- Regressed strict-compiled cards:");
        for regression in &score_delta_summary.regressions {
            println!(
                "  - {}: {:.4} -> {:.4} ({:+.4})",
                regression.card_name,
                regression.before_score,
                regression.after_score,
                regression.delta()
            );
        }
    }
    if let Some(tag) = &args.tag {
        println!("- Tag filter: {tag}");
    }
    if let Some(names) = &name_filtered_names {
        println!("- Name filter: {} card(s)", names.len());
    }
    if tag_filtered_names.is_some() || name_filtered_names.is_some() {
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
    println!("- Timing:");
    println!("  - total: {}", format_duration(total_elapsed));
    println!("  - open DB: {}", format_duration(db_open_elapsed));
    println!(
        "  - load previous strict scores: {}",
        format_duration(score_before_elapsed)
    );
    println!("  - load cards: {}", format_duration(load_cards_elapsed));
    println!("  - filter cards: {}", format_duration(filter_elapsed));
    println!(
        "  - compile snapshots: {}",
        format_duration(compile_elapsed)
    );
    println!("  - insert snapshots: {}", format_duration(insert_elapsed));
    println!("  - prune stale rows: {}", format_duration(prune_elapsed));
    println!(
        "  - load updated scores: {}",
        format_duration(score_after_elapsed)
    );

    Ok(())
}

fn format_card_progress(completed: usize, total: usize, snapshot: &CompilationSnapshot) -> String {
    format!(
        "[{completed}/{total}] {} — {} (similarity {:.4})",
        snapshot.card_name,
        snapshot.parse_status.as_str(),
        snapshot.similarity_score
    )
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.3}s", duration.as_secs_f64())
    } else if duration.as_millis() > 0 {
        format!("{}ms", duration.as_millis())
    } else if duration.as_micros() > 0 {
        format!("{}us", duration.as_micros())
    } else {
        format!("{}ns", duration.as_nanos())
    }
}

#[derive(Debug, Default)]
struct ScoreDeltaSummary {
    increased_count: usize,
    decreased_count: usize,
    increased_deltas: Vec<f32>,
    decreased_deltas: Vec<f32>,
    regressions: Vec<ScoreRegression>,
}

#[derive(Debug)]
struct ScoreRegression {
    card_name: String,
    before_score: f32,
    after_score: f32,
}

impl ScoreRegression {
    fn delta(&self) -> f32 {
        self.after_score - self.before_score
    }
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
            summary.regressions.push(ScoreRegression {
                card_name: card_name.clone(),
                before_score: *before_score,
                after_score: *after_score,
            });
        }
    }
    summary.regressions.sort_by(|left, right| {
        left.delta()
            .total_cmp(&right.delta())
            .then_with(|| left.card_name.cmp(&right.card_name))
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith_tools::ParseStatus;

    #[test]
    fn card_progress_includes_count_name_status_and_score() {
        let snapshot = CompilationSnapshot {
            card_name: "Progress Test".to_string(),
            oracle_text: String::new(),
            raw_oracle_text: String::new(),
            parse_status: ParseStatus::StrictCompiled,
            parse_error: None,
            normalized_oracle_text: String::new(),
            compiled_text: None,
            compiled_card_definition: None,
            oracle_coverage: 1.0,
            compiled_coverage: 1.0,
            similarity_score: 0.99725,
            line_delta: 0,
            semantic_mismatch: false,
            has_unimplemented: false,
            parse_lossy: false,
            parse_loss_reasons: String::new(),
            parse_loss_count: 0,
            content_hash: String::new(),
        };

        assert_eq!(
            format_card_progress(17, 42, &snapshot),
            "[17/42] Progress Test — strict_compiled (similarity 0.9973)"
        );
    }
}
