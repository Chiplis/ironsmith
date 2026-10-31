//! Read-only, single-parse-per-face feedback loop for card coverage fixes.
//! Build only this target, then run --names names.txt --out results.json.
//! Optional --checks checks.json verifies typed-definition/audit contracts.
use ironsmith_tools::{CardStatusDb, compile_authoritative_snapshot_with_audit};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fs, time::Instant};

mod card_score_probe_runtime;
#[path = "../../tests/support/semantic_audit_probes.rs"]
mod semantic_audit_probes;

#[derive(Default, Deserialize)]
struct Check {
    card: String,
    #[serde(default)]
    audit_contains: Vec<String>,
    #[serde(default)]
    audit_excludes: Vec<String>,
    #[serde(default)]
    definition_contains: Vec<String>,
    #[serde(default)]
    min_score: Option<f32>,
    #[serde(default)]
    presentation_independent: bool,
    #[serde(default)]
    mutations_visible: bool,
    #[serde(default)]
    runtime: bool,
}

#[derive(Serialize)]
struct Row {
    card_name: String,
    parse_status: String,
    similarity_score: f32,
    oracle: String,
    audit: String,
    parse_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    definition: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    contract_failures: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let mut db_path = "reports/engine-status.sqlite3".to_string();
    let mut names_path = None;
    let mut out = None;
    let mut checks_path = None;
    let mut definitions = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db_path = args.next().ok_or("--db needs a path")?,
            "--names" => names_path = Some(args.next().ok_or("--names needs a path")?),
            "--out" => out = Some(args.next().ok_or("--out needs a path")?),
            "--checks" => checks_path = Some(args.next().ok_or("--checks needs a path")?),
            "--definitions" => definitions = true,
            _ => return Err(format!("unknown argument {arg}; use --names FILE --out FILE [--db FILE] [--checks FILE] [--definitions]").into()),
        }
    }
    let names: BTreeSet<String> = fs::read_to_string(names_path.ok_or("--names is required")?)?
        .lines()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .collect();
    let out = out.ok_or("--out is required")?;
    let checks: Vec<Check> = checks_path
        .map(fs::read_to_string)
        .transpose()?
        .map(|text| serde_json::from_str(&text))
        .transpose()?
        .unwrap_or_default();
    let db = CardStatusDb::open_read_only(db_path)?;
    let cards = db.registry_corpus_cards()?;
    let cards: Vec<_> = cards
        .into_iter()
        .filter(|c| names.contains(&c.name))
        .collect();
    let found: BTreeSet<_> = cards.iter().map(|c| c.name.clone()).collect();
    let missing: Vec<_> = names.difference(&found).collect();
    if !missing.is_empty() {
        return Err(format!("unknown canonical names: {missing:?}").into());
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .stack_size(64 * 1024 * 1024)
        .build()?;
    let completed = std::sync::atomic::AtomicUsize::new(0);
    let mut results = pool.install(|| {
        cards
            .par_iter()
            .map(|card| {
                let observations: Vec<_> = card
                    .faces
                    .iter()
                    .map(compile_authoritative_snapshot_with_audit)
                    .collect();
                let faces: Vec<_> = observations
                    .iter()
                    .map(|(snapshot, _, _)| snapshot.clone())
                    .collect();
                let combined =
                    ironsmith_tools::corpus::aggregate_face_snapshots(&card.name, &faces);
                let audit = observations
                    .iter()
                    .map(|(s, lines, _)| format!("{}:\n{}", s.card_name, lines.join("\n")))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let mut contract_failures = Vec::new();
                if let Some(check) = checks.iter().find(|check| check.card == card.name) {
                    for (snapshot, before, definition) in &observations {
                        let Some(definition) = definition else {
                            continue;
                        };
                        if check.runtime {
                            contract_failures.extend(
                                card_score_probe_runtime::check(&card.name, definition)
                                    .into_iter()
                                    .map(|failure| {
                                        format!("{}: runtime: {failure}", snapshot.card_name)
                                    }),
                            );
                        }
                        if check.presentation_independent {
                            let poisoned = semantic_audit_probes::poison_definition(definition);
                            if *before != ironsmith::compiled_text::semantic_audit_lines(&poisoned)
                            {
                                contract_failures.push(format!(
                                    "{}: audit depends on retained presentation",
                                    snapshot.card_name
                                ));
                            }
                        }
                        if check.mutations_visible {
                            let mutations = semantic_audit_probes::executable_mutations(definition);
                            if mutations.is_empty() {
                                contract_failures.push(format!(
                                    "{}: no supported executable mutation",
                                    snapshot.card_name
                                ));
                            }
                            for (index, kind, changed) in mutations {
                                if *before
                                    == ironsmith::compiled_text::semantic_audit_lines(&changed)
                                {
                                    contract_failures.push(format!(
                                        "{}: hidden mutation {kind} at ability {index}",
                                        snapshot.card_name
                                    ));
                                }
                            }
                        }
                    }
                }
                let definition = faces
                    .iter()
                    .filter_map(|s| s.compiled_card_definition.as_deref())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let row = Row {
                    card_name: card.name.clone(),
                    parse_status: combined.parse_status.as_str().to_string(),
                    similarity_score: combined.similarity_score,
                    oracle: combined.normalized_oracle_text,
                    audit,
                    parse_error: combined.parse_error,
                    definition: Some(definition),
                    contract_failures,
                };
                let done = completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if cards.len() > 100 && (done % 100 == 0 || done == cards.len()) {
                    eprintln!(
                        "Checked {done}/{} cards in {:.1}s",
                        cards.len(),
                        start.elapsed().as_secs_f64()
                    );
                }
                row
            })
            .collect::<Vec<_>>()
    });
    results.sort_by(|a, b| a.card_name.cmp(&b.card_name));
    let mut failures: Vec<_> = results
        .iter()
        .flat_map(|row| row.contract_failures.iter().cloned())
        .collect();
    for check in &checks {
        let Some(row) = results.iter().find(|r| r.card_name == check.card) else {
            failures.push(format!("{}: check was not selected", check.card));
            continue;
        };
        if row.parse_status != "strict_compiled" {
            failures.push(format!("{}: {}", check.card, row.parse_status));
        }
        for expected in &check.audit_contains {
            if !row.audit.contains(expected) {
                failures.push(format!("{}: audit missing {expected:?}", check.card));
            }
        }
        for excluded in &check.audit_excludes {
            if row.audit.contains(excluded) {
                failures.push(format!("{}: audit contains {excluded:?}", check.card));
            }
        }
        for expected in &check.definition_contains {
            if !row.definition.as_deref().unwrap_or("").contains(expected) {
                failures.push(format!("{}: definition missing {expected:?}", check.card));
            }
        }
        if let Some(min) = check.min_score {
            if row.similarity_score < min {
                failures.push(format!(
                    "{}: score {} < {min}",
                    check.card, row.similarity_score
                ));
            }
        }
    }
    if !definitions {
        for row in &mut results {
            row.definition = None;
        }
    }
    let report = serde_json::json!({"elapsed_seconds": start.elapsed().as_secs_f64(), "cards": results.len(), "checks": checks.len(), "failures": failures, "results": results});
    fs::write(out, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "{} cards, {} checks, {} failures, {:.1}s",
        report["cards"],
        checks.len(),
        failures.len(),
        start.elapsed().as_secs_f64()
    );
    if !failures.is_empty() {
        return Err(format!("contract failures: {}", failures.join("; ")).into());
    }
    Ok(())
}
