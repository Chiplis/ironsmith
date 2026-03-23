use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::panic::{self, AssertUnwindSafe};

use ironsmith::cards::{
    CardDefinition, CardDefinitionBuilder, generated_definition_has_unimplemented_content,
};
use ironsmith::compiled_text::oracle_like_lines;
use ironsmith::ids::CardId;
use ironsmith::semantic_compare::strip_reminder_text_for_comparison;
use ironsmith_tools::{CardStatusDb, CompilationSnapshot, ParseStatus, default_db_path};
use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde_json::Value;

#[derive(Debug)]
struct Args {
    cards_path: String,
    all_out: String,
    mismatch_out: String,
    strip_reminder_for_comparison: bool,
    db_path: String,
    no_db: bool,
}

#[derive(Debug)]
struct CardInput {
    name: String,
    oracle_text: String,
    comparison_oracle_text: String,
    metadata_lines: Vec<String>,
    parse_input: String,
    comparison_parse_input: String,
}

#[derive(Debug, Clone)]
struct CsvRow {
    name: String,
    oracle_text: String,
    compiled_text: String,
}

enum ParseOutcome {
    Success(CardDefinition),
    Error(String),
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut all_out = "cards_compiled_oracle_text.csv".to_string();
    let mut mismatch_out = "cards_compiled_oracle_text_semantic_mismatch.csv".to_string();
    let mut strip_reminder_for_comparison = true;
    let mut db_path = default_db_path().display().to_string();
    let mut no_db = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = args
                    .next()
                    .ok_or_else(|| "--cards requires a path".to_string())?;
            }
            "--all-out" => {
                all_out = args
                    .next()
                    .ok_or_else(|| "--all-out requires a path".to_string())?;
            }
            "--mismatch-out" => {
                mismatch_out = args
                    .next()
                    .ok_or_else(|| "--mismatch-out requires a path".to_string())?;
            }
            "--comparison-mode" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--comparison-mode requires a value".to_string())?;
                strip_reminder_for_comparison = match value.as_str() {
                    "strip-reminder" => true,
                    "full" => false,
                    _ => {
                        return Err(format!(
                            "invalid --comparison-mode '{value}'. expected 'strip-reminder' or 'full'"
                        ));
                    }
                };
            }
            "--db-path" => {
                db_path = args
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--no-db" => {
                no_db = true;
            }
            "-h" | "--help" => {
                return Err(
                    "usage: cargo run --bin export_compiled_oracle_csv -- [--cards <path>] [--all-out <path>] [--mismatch-out <path>] [--comparison-mode <strip-reminder|full>] [--db-path <path>] [--no-db]".to_string(),
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. expected --cards/--all-out/--mismatch-out/--comparison-mode/--db-path/--no-db"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        all_out,
        mismatch_out,
        strip_reminder_for_comparison,
        db_path,
        no_db,
    })
}

fn value_to_string(value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if let Some(value) = value.as_str() {
        return Some(value.to_string());
    }
    Some(value.to_string())
}

fn get_first_face(card: &Value) -> Option<&Value> {
    card.get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.first())
}

fn pick_field(card: &Value, face: Option<&Value>, key: &str) -> Option<String> {
    if let Some(value) = card.get(key).and_then(value_to_string) {
        return Some(value);
    }
    face.and_then(|value| value.get(key))
        .and_then(value_to_string)
}

fn build_card_input(card: &Value) -> Option<CardInput> {
    let face = get_first_face(card);
    let name = pick_field(card, face, "name")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let oracle_text = pick_field(card, face, "oracle_text")?.trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }

    let mana_cost = pick_field(card, face, "mana_cost");
    let type_line = pick_field(card, face, "type_line");
    let power = pick_field(card, face, "power");
    let toughness = pick_field(card, face, "toughness");
    let loyalty = pick_field(card, face, "loyalty");
    let defense = pick_field(card, face, "defense");

    let mut metadata_lines = Vec::new();
    if let Some(mana_cost) = mana_cost.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Mana cost: {}", mana_cost.trim()));
    }
    if let Some(type_line) = type_line.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Type: {}", type_line.trim()));
    }
    if let (Some(power), Some(toughness)) = (power, toughness) {
        if !power.trim().is_empty() && !toughness.trim().is_empty() {
            metadata_lines.push(format!(
                "Power/Toughness: {}/{}",
                power.trim(),
                toughness.trim()
            ));
        }
    }
    if let Some(loyalty) = loyalty.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Loyalty: {}", loyalty.trim()));
    }
    if let Some(defense) = defense.filter(|value| !value.trim().is_empty()) {
        metadata_lines.push(format!("Defense: {}", defense.trim()));
    }
    let parse_input = build_parse_input(&metadata_lines, &oracle_text);
    let comparison_oracle_text = strip_reminder_text_for_comparison(&oracle_text);
    let comparison_parse_input = build_parse_input(&metadata_lines, &comparison_oracle_text);

    Some(CardInput {
        name,
        oracle_text,
        comparison_oracle_text,
        metadata_lines,
        parse_input,
        comparison_parse_input,
    })
}

fn build_parse_input(metadata_lines: &[String], oracle_text: &str) -> String {
    let mut lines = metadata_lines.to_vec();
    if !oracle_text.trim().is_empty() {
        lines.push(oracle_text.to_string());
    }
    lines.join("\n")
}

fn normalize_line_for_comparison(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn reminder_equivalent_lines_from_oracle(oracle_text: &str) -> HashSet<String> {
    oracle_text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
                return None;
            }
            let inner = trimmed.trim_start_matches('(').trim_end_matches(')').trim();
            if inner.is_empty() {
                return None;
            }
            Some(normalize_line_for_comparison(inner))
        })
        .collect()
}

fn strip_compiled_reminder_equivalents(compiled_text: &str, oracle_text: &str) -> String {
    let reminder_equivalents = reminder_equivalent_lines_from_oracle(oracle_text);
    strip_reminder_text_for_comparison(compiled_text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !reminder_equivalents.contains(&normalize_line_for_comparison(line)))
        .collect::<Vec<_>>()
        .join("\n")
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

fn parse_card(name: &str, parse_input: &str, allow_unsupported: bool) -> ParseOutcome {
    set_allow_unsupported(allow_unsupported);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        CardDefinitionBuilder::new(CardId::from_raw(1), name).parse_text(parse_input.to_string())
    }));
    match result {
        Ok(Ok(definition)) => ParseOutcome::Success(definition),
        Ok(Err(error)) => ParseOutcome::Error(format!("{error:?}")),
        Err(_payload) => ParseOutcome::Error("panic".to_string()),
    }
}

fn csv_escape(value: &str) -> String {
    if !value.contains([',', '"', '\n', '\r']) {
        return value.to_string();
    }
    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn write_csv(path: &str, rows: &[CsvRow]) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = String::new();
    out.push_str("name,oracle_text,compiled_text\n");
    for row in rows {
        let fields = [
            csv_escape(&row.name),
            csv_escape(&row.oracle_text),
            csv_escape(&row.compiled_text),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

fn definition_has_semantics(definition: &CardDefinition) -> bool {
    !definition.abilities.is_empty()
        || definition
            .spell_effect
            .as_ref()
            .is_some_and(|effects| !effects.is_empty())
        || definition.aura_attach_filter.is_some()
        || !definition.alternative_casts.is_empty()
        || !definition.optional_costs.is_empty()
        || definition.max_saga_chapter.is_some()
        || !definition.additional_cost.costs().is_empty()
}

fn sanitize_definition_for_semantic_compare(definition: &CardDefinition) -> CardDefinition {
    let mut sanitized = definition.clone();
    sanitized.card.id = CardId::from_raw(1);
    sanitized.card.oracle_text.clear();
    sanitized.card.other_face = sanitized.card.other_face.map(|_| CardId::from_raw(2));
    for ability in &mut sanitized.abilities {
        ability.text = None;
    }
    sanitized
}

fn roundtrip_semantic_mismatch(
    comparison_definition: &CardDefinition,
    card_input: &CardInput,
    comparison_compiled_text: &str,
) -> bool {
    if generated_definition_has_unimplemented_content(comparison_definition) {
        return true;
    }
    if comparison_compiled_text.trim().is_empty() {
        return definition_has_semantics(comparison_definition);
    }

    let roundtrip_input = build_parse_input(&card_input.metadata_lines, comparison_compiled_text);
    let roundtrip = match parse_card(&card_input.name, &roundtrip_input, false) {
        ParseOutcome::Success(definition) => definition,
        ParseOutcome::Error(_) => return true,
    };

    let original_snapshot = format!(
        "{:#?}",
        sanitize_definition_for_semantic_compare(comparison_definition)
    );
    let roundtrip_snapshot = format!(
        "{:#?}",
        sanitize_definition_for_semantic_compare(&roundtrip)
    );
    original_snapshot != roundtrip_snapshot
}

fn for_each_card_in_json_array<R, F>(
    reader: R,
    mut processor: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    R: Read,
    F: FnMut(Value) -> Result<(), String>,
{
    struct ArrayVisitor<'a, F> {
        processor: &'a mut F,
    }

    impl<'de, F> Visitor<'de> for ArrayVisitor<'_, F>
    where
        F: FnMut(Value) -> Result<(), String>,
    {
        type Value = ();

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON array of card objects")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
        where
            A: SeqAccess<'de>,
        {
            while let Some(card) = seq.next_element::<Value>()? {
                (self.processor)(card).map_err(de::Error::custom)?;
            }
            Ok(())
        }
    }

    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    deserializer.deserialize_any(ArrayVisitor {
        processor: &mut processor,
    })?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let file = File::open(&args.cards_path)?;
    let reader = BufReader::new(file);

    let original_allow_unsupported = env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED").ok();
    let db = if args.no_db {
        None
    } else {
        Some(CardStatusDb::open(&args.db_path)?)
    };

    let mut total_rows = 0usize;
    let mut rows_with_oracle = 0usize;
    let mut compiled_rows = 0usize;
    let mut strict_successes = 0usize;
    let mut fallback_successes = 0usize;
    let mut parse_failures = 0usize;
    let mut comparison_parse_failures = 0usize;
    let mut mismatch_rows_count = 0usize;

    let mut all_rows = Vec::<CsvRow>::new();
    let mut mismatch_rows = Vec::<CsvRow>::new();

    for_each_card_in_json_array(reader, |card| {
        total_rows += 1;

        let Some(card_input) = build_card_input(&card) else {
            return Ok(());
        };
        rows_with_oracle += 1;

        let (definition, used_fallback, parse_error, parse_status) =
            match parse_card(&card_input.name, &card_input.parse_input, false) {
                ParseOutcome::Success(definition) => {
                    (definition, false, None, ParseStatus::StrictCompiled)
                }
                ParseOutcome::Error(strict_error) => {
                    match parse_card(&card_input.name, &card_input.parse_input, true) {
                        ParseOutcome::Success(definition) => (
                            definition,
                            true,
                            None,
                            ParseStatus::CompiledWithAllowUnsupported,
                        ),
                        ParseOutcome::Error(_) => {
                            parse_failures += 1;
                            if let Some(db) = db.as_ref() {
                                let snapshot = CompilationSnapshot::from_definition_result(
                                    &card_input.name,
                                    &card_input.oracle_text,
                                    ParseStatus::ParseFailed,
                                    Some(strict_error),
                                    None,
                                );
                                db.insert_snapshot_if_changed(&snapshot)
                                    .map_err(|err| err.to_string())?;
                            }
                            return Ok(());
                        }
                    }
                }
            };

        if let Some(db) = db.as_ref() {
            let snapshot = CompilationSnapshot::from_definition_result(
                &card_input.name,
                &card_input.oracle_text,
                parse_status,
                parse_error,
                Some(&definition),
            );
            db.insert_snapshot_if_changed(&snapshot)
                .map_err(|err| err.to_string())?;
        }

        let compiled = oracle_like_lines(&definition);
        let compiled_text = compiled.join("\n");
        let comparison_compiled_text = if args.strip_reminder_for_comparison {
            strip_compiled_reminder_equivalents(&compiled_text, &card_input.oracle_text)
        } else {
            compiled_text.clone()
        };
        let row = CsvRow {
            name: card_input.name.clone(),
            oracle_text: if args.strip_reminder_for_comparison {
                card_input.comparison_oracle_text.clone()
            } else {
                card_input.oracle_text.clone()
            },
            compiled_text: comparison_compiled_text.clone(),
        };

        let selected_comparison_parse_input = if args.strip_reminder_for_comparison {
            &card_input.comparison_parse_input
        } else {
            &card_input.parse_input
        };

        let comparison_definition = if selected_comparison_parse_input == &card_input.parse_input {
            definition.clone()
        } else {
            match parse_card(&card_input.name, selected_comparison_parse_input, false) {
                ParseOutcome::Success(definition) => definition,
                ParseOutcome::Error(_) => {
                    comparison_parse_failures += 1;
                    all_rows.push(row.clone());
                    mismatch_rows_count += 1;
                    mismatch_rows.push(row);
                    compiled_rows += 1;
                    if used_fallback {
                        fallback_successes += 1;
                    } else {
                        strict_successes += 1;
                    }
                    return Ok(());
                }
            }
        };

        let mismatch = roundtrip_semantic_mismatch(
            &comparison_definition,
            &card_input,
            &comparison_compiled_text,
        );

        compiled_rows += 1;
        if used_fallback {
            fallback_successes += 1;
        } else {
            strict_successes += 1;
        }

        all_rows.push(row.clone());
        if mismatch {
            mismatch_rows_count += 1;
            mismatch_rows.push(row);
        }

        Ok(())
    })?;

    match original_allow_unsupported {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", value);
        },
        None => set_allow_unsupported(false),
    }

    write_csv(&args.all_out, &all_rows)?;
    write_csv(&args.mismatch_out, &mismatch_rows)?;

    println!("Compiled oracle export complete");
    println!("- JSON rows scanned: {total_rows}");
    println!("- Rows with oracle text: {rows_with_oracle}");
    println!("- Compiled rows written: {compiled_rows}");
    println!("- Strict parse successes: {strict_successes}");
    println!("- Fallback parse successes: {fallback_successes}");
    println!("- Parse failures after fallback: {parse_failures}");
    if args.strip_reminder_for_comparison {
        println!(
            "- Comparison parse failures after stripping reminder text: {comparison_parse_failures}"
        );
    } else {
        println!("- Comparison parse failures in full mode: {comparison_parse_failures}");
    }
    println!("- Semantic mismatch rows: {mismatch_rows_count}");
    println!("- All rows CSV: {}", args.all_out);
    println!("- Mismatch rows CSV: {}", args.mismatch_out);
    if !args.no_db {
        println!("- DB: {}", args.db_path);
    }

    Ok(())
}
