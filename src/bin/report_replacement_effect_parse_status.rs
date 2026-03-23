use std::collections::HashMap;
use std::env;
use std::fs;
use std::panic::{self, AssertUnwindSafe};

use ironsmith::cards::{
    CardDefinition, CardDefinitionBuilder, generated_definition_has_unimplemented_content,
};
use ironsmith::compiled_text::compiled_lines;
use ironsmith::ids::CardId;
use ironsmith::semantic_compare::compare_semantics_scored;
use ironsmith_tools::{CardStatusDb, CompilationSnapshot, ParseStatus, default_db_path};
use serde_json::Value;

#[derive(Debug)]
struct Args {
    names_path: String,
    cards_path: String,
    out_csv: String,
    limit: Option<usize>,
    db_path: String,
    no_db: bool,
}

#[derive(Debug)]
struct CardPayload {
    parse_input: String,
    oracle_text: String,
}

#[derive(Debug)]
struct Row {
    name: String,
    parsed: bool,
    parse_strict: bool,
    parse_with_allow_unsupported: bool,
    status: String,
    parse_error_strict: String,
    parse_error_allow_unsupported: String,
    has_unimplemented: bool,
    semantic_mismatch: bool,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    compiled_lines_count: usize,
}

enum ParseOutcome {
    Success(CardDefinition),
    Error(String),
}

fn parse_args() -> Result<Args, String> {
    let mut names_path = "replacement_effect_cards_strict.txt".to_string();
    let mut cards_path = "replacement_effect_cards_strict_oracle_subset.json".to_string();
    let mut out_csv = "replacement_effect_cards_parse_report.csv".to_string();
    let mut limit = None;
    let mut db_path = default_db_path().display().to_string();
    let mut no_db = false;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--names" => {
                names_path = iter
                    .next()
                    .ok_or_else(|| "--names requires a path".to_string())?;
            }
            "--cards" => {
                cards_path = iter
                    .next()
                    .ok_or_else(|| "--cards requires a path".to_string())?;
            }
            "--out" => {
                out_csv = iter
                    .next()
                    .ok_or_else(|| "--out requires a path".to_string())?;
            }
            "--limit" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                limit = Some(
                    raw.parse::<usize>()
                        .map_err(|err| format!("invalid --limit '{raw}': {err}"))?,
                );
            }
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--no-db" => {
                no_db = true;
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --bin report_replacement_effect_parse_status -- [--names <path>] [--cards <path>] [--out <path>] [--limit <n>] [--db-path <path>] [--no-db]".to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --names/--cards/--out/--limit/--db-path/--no-db"
                ));
            }
        }
    }

    Ok(Args {
        names_path,
        cards_path,
        out_csv,
        limit,
        db_path,
        no_db,
    })
}

fn read_names(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let mut names = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    Ok(names)
}

fn get_first_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(|faces| faces.as_array())
        .and_then(|faces| faces.first())
}

fn value_to_string(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    Some(value.to_string())
}

fn pick_field(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    if let Some(v) = card.get(key).and_then(value_to_string) {
        return Some(v);
    }
    face.and_then(|f| f.get(key)).and_then(value_to_string)
}

fn build_card_payload(card: &Value) -> Option<CardPayload> {
    let face = get_first_face(card);
    let mana_cost = pick_field(card, face, "mana_cost");
    let type_line = pick_field(card, face, "type_line");
    let oracle_text = pick_field(card, face, "oracle_text").unwrap_or_default();
    let power = pick_field(card, face, "power");
    let toughness = pick_field(card, face, "toughness");
    let loyalty = pick_field(card, face, "loyalty");
    let defense = pick_field(card, face, "defense");

    let mut lines = Vec::new();
    if let Some(mana_cost) = mana_cost
        && !mana_cost.is_empty()
    {
        lines.push(format!("Mana cost: {mana_cost}"));
    }
    if let Some(type_line) = type_line
        && !type_line.is_empty()
    {
        lines.push(format!("Type: {type_line}"));
    }
    if let (Some(power), Some(toughness)) = (power, toughness)
        && !power.is_empty()
        && !toughness.is_empty()
    {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(loyalty) = loyalty
        && !loyalty.is_empty()
    {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    if let Some(defense) = defense
        && !defense.is_empty()
    {
        lines.push(format!("Defense: {defense}"));
    }
    if !oracle_text.is_empty() {
        lines.push(oracle_text.clone());
    }

    if lines.is_empty() {
        return None;
    }

    Some(CardPayload {
        parse_input: lines.join("\n"),
        oracle_text,
    })
}

fn load_cards(path: &str) -> Result<HashMap<String, CardPayload>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let cards: Vec<Value> = serde_json::from_str(&raw)?;
    let mut map = HashMap::new();
    for card in cards {
        let Some(name) = card
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
        else {
            continue;
        };
        if map.contains_key(&name) {
            continue;
        }
        if let Some(payload) = build_card_payload(&card) {
            map.insert(name, payload);
        }
    }
    Ok(map)
}

fn set_allow_unsupported(enabled: bool) {
    unsafe {
        if enabled {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", "1");
        } else {
            env::remove_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED");
        }
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        return (*msg).to_string();
    }
    if let Some(msg) = payload.downcast_ref::<String>() {
        return msg.clone();
    }
    "unknown panic payload".to_string()
}

fn parse_card(name: &str, parse_input: &str, allow_unsupported: bool) -> ParseOutcome {
    set_allow_unsupported(allow_unsupported);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        CardDefinitionBuilder::new(CardId::new(), name).parse_text(parse_input.to_string())
    }));
    match result {
        Ok(Ok(definition)) => ParseOutcome::Success(definition),
        Ok(Err(err)) => ParseOutcome::Error(format!("{err:?}")),
        Err(payload) => ParseOutcome::Error(format!("panic: {}", panic_payload_to_string(payload))),
    }
}

fn csv_escape(value: &str) -> String {
    let needs_quotes =
        value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r');
    if !needs_quotes {
        return value.to_string();
    }
    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn write_csv(path: &str, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = String::new();
    out.push_str("name,parsed,parse_strict,parse_with_allow_unsupported,status,parse_error_strict,parse_error_allow_unsupported,has_unimplemented,semantic_mismatch,oracle_coverage,compiled_coverage,similarity_score,line_delta,compiled_lines_count\n");

    for row in rows {
        let fields = [
            csv_escape(&row.name),
            row.parsed.to_string(),
            row.parse_strict.to_string(),
            row.parse_with_allow_unsupported.to_string(),
            csv_escape(&row.status),
            csv_escape(&row.parse_error_strict),
            csv_escape(&row.parse_error_allow_unsupported),
            row.has_unimplemented.to_string(),
            row.semantic_mismatch.to_string(),
            format!("{:.6}", row.oracle_coverage),
            format!("{:.6}", row.compiled_coverage),
            format!("{:.6}", row.similarity_score),
            row.line_delta.to_string(),
            row.compiled_lines_count.to_string(),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }

    fs::write(path, out)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let names = read_names(&args.names_path)?;
    let cards_by_name = load_cards(&args.cards_path)?;
    let db = if args.no_db {
        None
    } else {
        Some(CardStatusDb::open(&args.db_path)?)
    };

    let original_allow_unsupported = env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED").ok();

    let mut rows = Vec::new();
    let mut parse_ok_count = 0usize;
    let mut parse_fail_count = 0usize;
    let mut parse_unsupported_count = 0usize;
    let mut parse_correct_count = 0usize;
    let mut semantic_mismatch_count = 0usize;
    let mut missing_card_data_count = 0usize;

    for (index, name) in names.iter().enumerate() {
        if let Some(limit) = args.limit
            && index >= limit
        {
            break;
        }

        let Some(payload) = cards_by_name.get(name) else {
            missing_card_data_count += 1;
            rows.push(Row {
                name: name.clone(),
                parsed: false,
                parse_strict: false,
                parse_with_allow_unsupported: false,
                status: "missing_card_data".to_string(),
                parse_error_strict: String::new(),
                parse_error_allow_unsupported: String::new(),
                has_unimplemented: false,
                semantic_mismatch: false,
                oracle_coverage: 0.0,
                compiled_coverage: 0.0,
                similarity_score: 0.0,
                line_delta: 0,
                compiled_lines_count: 0,
            });
            continue;
        };

        let strict_result = parse_card(name, &payload.parse_input, false);

        let mut row = Row {
            name: name.clone(),
            parsed: false,
            parse_strict: false,
            parse_with_allow_unsupported: false,
            status: String::new(),
            parse_error_strict: String::new(),
            parse_error_allow_unsupported: String::new(),
            has_unimplemented: false,
            semantic_mismatch: false,
            oracle_coverage: 0.0,
            compiled_coverage: 0.0,
            similarity_score: 0.0,
            line_delta: 0,
            compiled_lines_count: 0,
        };

        let snapshot = match strict_result {
            ParseOutcome::Success(definition) => {
                row.parsed = true;
                row.parse_strict = true;
                row.parse_with_allow_unsupported = true;
                let compiled = compiled_lines(&definition);
                row.compiled_lines_count = compiled.len();
                row.has_unimplemented = generated_definition_has_unimplemented_content(&definition);
                let (oracle_cov, compiled_cov, similarity, line_delta, semantic_mismatch) =
                    compare_semantics_scored(&payload.oracle_text, &compiled, None);
                row.oracle_coverage = oracle_cov;
                row.compiled_coverage = compiled_cov;
                row.similarity_score = similarity;
                row.line_delta = line_delta;
                row.semantic_mismatch = semantic_mismatch;

                if row.has_unimplemented {
                    row.status = "parses_with_unsupported_semantics".to_string();
                    parse_unsupported_count += 1;
                } else if row.semantic_mismatch {
                    row.status = "parses_but_semantic_mismatch".to_string();
                    semantic_mismatch_count += 1;
                } else {
                    row.status = "parses_correctly".to_string();
                    parse_correct_count += 1;
                }
                parse_ok_count += 1;
                CompilationSnapshot::from_definition_result(
                    name,
                    &payload.oracle_text,
                    ParseStatus::StrictCompiled,
                    None,
                    Some(&definition),
                )
            }
            ParseOutcome::Error(err) => {
                row.parse_error_strict = err;
                let allow_result = parse_card(name, &payload.parse_input, true);
                match allow_result {
                    ParseOutcome::Success(definition) => {
                        row.parsed = true;
                        row.parse_with_allow_unsupported = true;
                        row.status = "parses_with_unsupported_semantics".to_string();
                        row.has_unimplemented =
                            generated_definition_has_unimplemented_content(&definition);

                        let compiled = compiled_lines(&definition);
                        row.compiled_lines_count = compiled.len();
                        let (oracle_cov, compiled_cov, similarity, line_delta, semantic_mismatch) =
                            compare_semantics_scored(&payload.oracle_text, &compiled, None);
                        row.oracle_coverage = oracle_cov;
                        row.compiled_coverage = compiled_cov;
                        row.similarity_score = similarity;
                        row.line_delta = line_delta;
                        row.semantic_mismatch = semantic_mismatch;

                        parse_ok_count += 1;
                        parse_unsupported_count += 1;
                        CompilationSnapshot::from_definition_result(
                            name,
                            &payload.oracle_text,
                            ParseStatus::CompiledWithAllowUnsupported,
                            None,
                            Some(&definition),
                        )
                    }
                    ParseOutcome::Error(allow_err) => {
                        row.status = "does_not_parse".to_string();
                        row.parse_error_allow_unsupported = allow_err;
                        parse_fail_count += 1;
                        CompilationSnapshot::from_definition_result(
                            name,
                            &payload.oracle_text,
                            ParseStatus::ParseFailed,
                            Some(row.parse_error_strict.clone()),
                            None,
                        )
                    }
                }
            }
        };

        if let Some(db) = db.as_ref() {
            db.insert_snapshot_if_changed(&snapshot)?;
        }
        rows.push(row);
    }

    match original_allow_unsupported {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", value);
        },
        None => set_allow_unsupported(false),
    }

    write_csv(&args.out_csv, &rows)?;

    println!("Replacement parse report complete");
    println!("- Names requested: {}", names.len());
    println!("- Rows written: {}", rows.len());
    println!("- Parse OK (any mode): {parse_ok_count}");
    println!("- Does not parse (even with allow unsupported): {parse_fail_count}");
    println!("- Parses with unsupported semantics: {parse_unsupported_count}");
    println!("- Parses correctly: {parse_correct_count}");
    println!("- Parses but semantic mismatch: {semantic_mismatch_count}");
    println!("- Missing card data: {missing_card_data_count}");
    println!("- CSV: {}", args.out_csv);
    if !args.no_db {
        println!("- DB: {}", args.db_path);
    }

    Ok(())
}
