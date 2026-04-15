use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};

use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::compiled_text::compiled_lines;
use ironsmith::ids::CardId;
use serde::Serialize;

#[derive(Debug)]
struct Args {
    out: String,
    limit: Option<usize>,
    examples: usize,
    raw: bool,
}

#[derive(Debug, Serialize)]
struct AuditRow {
    index: usize,
    name: String,
    oracle_text: String,
    compiled_text: String,
    has_object_filter: bool,
    object_filter_mentions: usize,
    parse_error: Option<String>,
}

fn raw_contains_unimplemented(definition: &ironsmith::cards::CardDefinition) -> bool {
    format!("{definition:#?}")
        .to_ascii_lowercase()
        .contains("unimplemented")
}

fn parse_args() -> Result<Args, String> {
    let mut out = "/tmp/compiled_audit.jsonl".to_string();
    let mut limit = None;
    let mut examples = 20usize;
    let mut raw = false;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--out" => {
                out = iter
                    .next()
                    .ok_or_else(|| "--out requires a path".to_string())?;
            }
            "--limit" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                limit = Some(
                    raw.parse::<usize>()
                        .map_err(|e| format!("invalid --limit value '{raw}': {e}"))?,
                );
            }
            "--examples" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--examples requires a number".to_string())?;
                examples = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --examples value '{raw}': {e}"))?;
            }
            "--raw" => {
                raw = true;
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. supported: --out <path> --limit <n> --examples <n> --raw"
                ));
            }
        }
    }

    Ok(Args {
        out,
        limit,
        examples,
        raw,
    })
}

fn extract_effect_type(line: &str) -> Option<String> {
    let marker = "Effect(";
    let start = line.find(marker)? + marker.len();
    let rest = &line[start..];
    let end = rest
        .find(|ch: char| ch == ' ' || ch == '{' || ch == ')' || ch == ':')
        .unwrap_or(rest.len());
    let kind = &rest[..end];
    if kind.is_empty() {
        None
    } else {
        Some(kind.to_string())
    }
}

fn split_block(block: &str) -> Option<(String, String, String)> {
    let mut lines = block.lines();
    let first = lines.next()?.trim();
    if first.is_empty() {
        return None;
    }
    let name = first.strip_prefix("Name: ")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let parse_input = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    if parse_input.is_empty() {
        return None;
    }

    let oracle_text = parse_input
        .lines()
        .filter(|line| {
            let lower = line.trim().to_ascii_lowercase();
            !(lower.starts_with("mana cost:")
                || lower.starts_with("type:")
                || lower.starts_with("type line:")
                || lower.starts_with("power/toughness:")
                || lower.starts_with("loyalty:")
                || lower.starts_with("defense:"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    Some((name, parse_input, oracle_text))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(io::Error::other)?;

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let out_file = File::create(&args.out)?;
    let mut writer = BufWriter::new(out_file);

    let mut total = 0usize;
    let mut parsed_ok = 0usize;
    let mut parse_failed = 0usize;
    let mut filtered_unimplemented = 0usize;
    let mut cards_with_object_filter = 0usize;
    let mut object_filter_mentions_total = 0usize;
    let mut effect_kind_counts: HashMap<String, usize> = HashMap::new();
    let mut object_filter_examples: Vec<(String, usize)> = Vec::new();

    for block in input.split("\n---\n") {
        if let Some(limit) = args.limit
            && total >= limit
        {
            break;
        }

        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let Some((name, parse_input, oracle_text)) = split_block(block) else {
            continue;
        };

        total += 1;

        let mut compiled_text = String::new();
        let mut parse_error = None;

        match CardDefinitionBuilder::new(CardId::new(), &name).parse_text(parse_input) {
            Ok(def) => {
                if raw_contains_unimplemented(&def) {
                    parse_failed += 1;
                    filtered_unimplemented += 1;
                    parse_error = Some(
                        "Filtered as unparseable: raw compiled output contains 'unimplemented'"
                            .to_string(),
                    );
                } else {
                    parsed_ok += 1;
                    compiled_text = if args.raw {
                        format!("{def:#?}")
                    } else {
                        let lines = compiled_lines(&def);
                        if lines.is_empty() {
                            "<none>".to_string()
                        } else {
                            lines.join("\n")
                        }
                    };
                };
            }
            Err(err) => {
                parse_failed += 1;
                parse_error = Some(format!("{err:?}"));
            }
        }

        let object_filter_mentions = compiled_text.matches("ObjectFilter").count();
        let has_object_filter = object_filter_mentions > 0;
        if has_object_filter {
            cards_with_object_filter += 1;
            object_filter_mentions_total += object_filter_mentions;
            if object_filter_examples.len() < args.examples {
                object_filter_examples.push((name.clone(), object_filter_mentions));
            }
            for line in compiled_text.lines() {
                if !line.contains("ObjectFilter") {
                    continue;
                }
                if let Some(kind) = extract_effect_type(line) {
                    *effect_kind_counts.entry(kind).or_insert(0) += 1;
                }
            }
        }

        let row = AuditRow {
            index: total,
            name,
            oracle_text,
            compiled_text,
            has_object_filter,
            object_filter_mentions,
            parse_error,
        };
        serde_json::to_writer(&mut writer, &row)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;

    println!("Audit complete");
    println!("- Total cards processed: {total}");
    println!("- Parsed successfully: {parsed_ok}");
    println!("- Parse failures: {parse_failed}");
    println!("- Filtered as unparseable for 'unimplemented': {filtered_unimplemented}");
    println!("- Cards with 'ObjectFilter' in compiled text: {cards_with_object_filter}");
    println!("- Total 'ObjectFilter' mentions: {object_filter_mentions_total}");
    println!("- JSONL report: {}", args.out);

    if !effect_kind_counts.is_empty() {
        let mut top: Vec<(String, usize)> = effect_kind_counts.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        println!("Top fallback effect kinds with ObjectFilter:");
        for (kind, count) in top.into_iter().take(args.examples) {
            println!("  - {kind}: {count}");
        }
    }

    if !object_filter_examples.is_empty() {
        println!("Example cards still containing ObjectFilter:");
        for (name, mentions) in object_filter_examples {
            println!("  - {name} ({mentions})");
        }
    }

    Ok(())
}
