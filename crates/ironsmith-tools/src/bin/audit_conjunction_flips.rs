//! Scans the latest strict-compiled cards for suspected and↔or conjunction
//! flips between oracle text and compiled text, as JSONL.
//!
//! "and"/"or" are similarity-comparison stopwords, so a compiled filter that
//! says "artifact and creature" where the oracle says "artifact or creature"
//! scores identically — this targeted invariant surfaces exactly that class.
//! Mass-quantified clauses (all/each/every) are skipped because oracle idiom
//! legitimately uses "and" there for what a filter expresses with "or".
//!
//! Usage:
//!   cargo run --release -p ironsmith-tools --bin audit_conjunction_flips -- \
//!     [--db-path <path>] [--out /tmp/conjunction_flips.jsonl]

use ironsmith::semantic_compare::conjunction_flips_between;
use rusqlite::Connection;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct FlipRow<'a> {
    card_name: &'a str,
    similarity_score: f64,
    left: String,
    right: String,
    oracle_conjunction: String,
    compiled_conjunction: String,
    oracle_clause: String,
    compiled_clause: String,
}

fn main() {
    let mut db_path = ironsmith_tools::default_db_path().display().to_string();
    let mut out = "/tmp/conjunction_flips.jsonl".to_string();
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db-path" => db_path = iter.next().expect("--db-path requires a path"),
            "--out" => out = iter.next().expect("--out requires a path"),
            other => {
                eprintln!(
                    "unknown argument '{other}'; usage: audit_conjunction_flips [--db-path <path>] [--out <path>]"
                );
                std::process::exit(2);
            }
        }
    }

    let conn = Connection::open(&db_path).expect("open status db");
    let mut stmt = conn
        .prepare(
            "SELECT card_name, oracle_text, compiled_text, similarity_score
             FROM latest_card_compilation
             WHERE parse_status = 'strict_compiled' AND compiled_text IS NOT NULL",
        )
        .expect("prepare latest compilation query");
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })
        .expect("query latest compilations");

    let mut writer = std::io::BufWriter::new(std::fs::File::create(&out).expect("create out file"));
    let mut flagged_cards = 0usize;
    let mut flagged_flips = 0usize;
    for row in rows {
        let (card_name, oracle_text, compiled_text, similarity_score) = row.expect("read row");
        let flips = conjunction_flips_between(&oracle_text, &compiled_text);
        if flips.is_empty() {
            continue;
        }
        flagged_cards += 1;
        for flip in flips {
            flagged_flips += 1;
            let record = FlipRow {
                card_name: &card_name,
                similarity_score,
                left: flip.left,
                right: flip.right,
                oracle_conjunction: flip.oracle_conjunction,
                compiled_conjunction: flip.compiled_conjunction,
                oracle_clause: flip.oracle_clause,
                compiled_clause: flip.compiled_clause,
            };
            writeln!(writer, "{}", serde_json::to_string(&record).unwrap()).expect("write row");
        }
    }
    writer.flush().expect("flush out file");
    eprintln!("{flagged_cards} cards with {flagged_flips} suspected and/or flips -> {out}");
}
