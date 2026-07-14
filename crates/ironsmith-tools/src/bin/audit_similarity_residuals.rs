//! Recompiles every registry card and emits clause-level similarity residuals
//! for strict-compiled cards scoring below a threshold, as JSONL.
//!
//! Each row pairs an under-covered clause with its best counterpart on the
//! other side plus the token diff, which is what the similarity score is
//! actually built from — use it to cluster rendering/parsing gaps by family.
//!
//! Usage:
//!   cargo run --release -p ironsmith-tools --bin audit_similarity_residuals -- \
//!     [--db-path <path>] [--threshold 0.99] [--out /tmp/residuals.jsonl]

use ironsmith::semantic_compare::compare_card_semantics_clause_residuals;
use ironsmith_tools::{
    CardPayload, CardStatusDb, compile_strict_snapshot_from_payload, default_db_path,
};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::BTreeSet;
use std::io::Write;

const RAYON_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug)]
struct Args {
    db_path: String,
    threshold: f32,
    out: String,
    compare_file: Option<String>,
    names: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut db_path = default_db_path().display().to_string();
    let mut threshold = 0.99f32;
    let mut out = "/tmp/residuals.jsonl".to_string();
    let mut compare_file = None;
    let mut names = None;

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--threshold" => {
                threshold = iter
                    .next()
                    .ok_or_else(|| "--threshold requires a value".to_string())?
                    .parse()
                    .map_err(|err| format!("invalid --threshold: {err}"))?;
            }
            "--out" => {
                out = iter
                    .next()
                    .ok_or_else(|| "--out requires a path".to_string())?;
            }
            "--compare-file" => {
                compare_file = Some(
                    iter.next()
                        .ok_or_else(|| "--compare-file requires a path".to_string())?,
                );
            }
            "--names" => {
                names = Some(
                    iter.next()
                        .ok_or_else(|| "--names requires a path".to_string())?,
                );
            }
            "-h" | "--help" => {
                return Err(
                    "usage: audit_similarity_residuals [--db-path <path>] [--threshold 0.99] [--out <path>] [--compare-file <path>] [--names <path>]"
                        .to_string(),
                );
            }
            _ => return Err(format!("unknown argument '{arg}'")),
        }
    }

    Ok(Args {
        db_path,
        threshold,
        out,
        compare_file,
        names,
    })
}

#[derive(Serialize)]
struct ResidualRow<'a> {
    card: &'a str,
    score: f32,
    side: &'static str,
    clause: &'a str,
    best_match: Option<&'a str>,
    jaccard: f32,
    missing: &'a [String],
    extra: &'a [String],
}

fn audit_compare_file(args: &Args, compare_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string(compare_file)?;
    let input = input.strip_prefix("Name: ").unwrap_or(&input);
    let mut writer = std::io::BufWriter::new(std::fs::File::create(&args.out)?);
    let mut card_count = 0usize;

    for block in input.split("\n\nName: ") {
        let Some((card, rest)) = block.split_once('\n') else {
            continue;
        };
        let Some((score_line, rest)) = rest.split_once('\n') else {
            continue;
        };
        let Some(score) = score_line
            .strip_prefix("Similarity: ")
            .and_then(|value| value.parse::<f32>().ok())
        else {
            continue;
        };
        if score >= args.threshold {
            continue;
        }
        let Some((_, rest)) = rest.split_once("Original oracle text:\n") else {
            continue;
        };
        let Some((oracle, compiled)) = rest.split_once("\nCompiled oracle text:\n") else {
            continue;
        };
        let compiled_lines = compiled.lines().map(str::to_string).collect::<Vec<_>>();
        let (oracle_residuals, compiled_residuals) =
            compare_card_semantics_clause_residuals(card, oracle, &compiled_lines);
        card_count += 1;
        for (side, residuals) in [
            ("oracle", oracle_residuals),
            ("compiled", compiled_residuals),
        ] {
            for residual in &residuals {
                if residual.best_jaccard >= 1.0 {
                    continue;
                }
                let row = ResidualRow {
                    card,
                    score,
                    side,
                    clause: &residual.clause,
                    best_match: residual.best_match.as_deref(),
                    jaccard: residual.best_jaccard,
                    missing: &residual.missing_tokens,
                    extra: &residual.extra_tokens,
                };
                writeln!(writer, "{}", serde_json::to_string(&row)?)?;
            }
        }
    }
    writer.flush()?;
    eprintln!(
        "wrote residuals for {card_count} cards below {} from {compare_file} to {}",
        args.threshold, args.out
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;

    if let Some(compare_file) = args.compare_file.as_deref() {
        return audit_compare_file(&args, compare_file);
    }

    let db = CardStatusDb::open_read_only(&args.db_path)?;
    let mut cards: Vec<CardPayload> = db.registry_card_payloads()?;
    if let Some(names_path) = args.names.as_deref() {
        let requested = std::fs::read_to_string(names_path)?
            .lines()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        cards.retain(|payload| requested.contains(&payload.name));
        eprintln!(
            "selected {} registry cards from {} requested names in {names_path}",
            cards.len(),
            requested.len()
        );
    }
    if cards.is_empty() {
        return Err(format!("no registry_card rows in {}", args.db_path).into());
    }

    let rayon_pool = rayon::ThreadPoolBuilder::new()
        .stack_size(RAYON_WORKER_STACK_SIZE)
        .build()?;
    let reports = rayon_pool.install(|| {
        cards
            .par_iter()
            .filter_map(|payload| {
                let snapshot = compile_strict_snapshot_from_payload(payload);
                if !matches!(
                    snapshot.parse_status,
                    ironsmith_tools::ParseStatus::StrictCompiled
                ) || snapshot.similarity_score >= args.threshold
                {
                    return None;
                }
                let compiled_lines = snapshot
                    .compiled_text
                    .as_deref()
                    .unwrap_or("")
                    .lines()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>();
                let (oracle_residuals, compiled_residuals) =
                    compare_card_semantics_clause_residuals(
                        &snapshot.card_name,
                        &snapshot.normalized_oracle_text,
                        &compiled_lines,
                    );
                Some((
                    snapshot.card_name.clone(),
                    snapshot.similarity_score,
                    oracle_residuals,
                    compiled_residuals,
                ))
            })
            .collect::<Vec<_>>()
    });

    let mut writer = std::io::BufWriter::new(std::fs::File::create(&args.out)?);
    let mut card_count = 0usize;
    for (card, score, oracle_residuals, compiled_residuals) in &reports {
        card_count += 1;
        for (side, residuals) in [
            ("oracle", oracle_residuals),
            ("compiled", compiled_residuals),
        ] {
            for residual in residuals {
                if residual.best_jaccard >= 1.0 {
                    continue;
                }
                let row = ResidualRow {
                    card,
                    score: *score,
                    side,
                    clause: &residual.clause,
                    best_match: residual.best_match.as_deref(),
                    jaccard: residual.best_jaccard,
                    missing: &residual.missing_tokens,
                    extra: &residual.extra_tokens,
                };
                writeln!(writer, "{}", serde_json::to_string(&row)?)?;
            }
        }
    }
    writer.flush()?;
    eprintln!(
        "wrote residuals for {card_count} cards below {} to {}",
        args.threshold, args.out
    );
    Ok(())
}
