use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::ids::CardId;
use ironsmith::static_abilities::StaticAbilityId;

#[derive(Debug)]
struct Args {
    cards_path: String,
    partition_path: String,
    limit: Option<usize>,
    top: usize,
}

#[derive(Debug, Clone)]
struct CardInput {
    name: String,
    parse_input: String,
}

fn usage() {
    eprintln!(
        "Usage: cargo run --bin audit_unimplemented_partition -- \\\n+  --partition <reports/parse_ok_unimplemented/...> [--cards <cards.json>] [--limit <n>] [--top <n>]"
    );
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut partition_path: Option<String> = None;
    let mut limit = None;
    let mut top = 40usize;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = iter
                    .next()
                    .ok_or_else(|| "--cards requires a path".to_string())?;
            }
            "--partition" => {
                partition_path = Some(
                    iter.next()
                        .ok_or_else(|| "--partition requires a path".to_string())?,
                );
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
            "--top" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--top requires a number".to_string())?;
                top = raw
                    .parse::<usize>()
                    .map_err(|err| format!("invalid --top '{raw}': {err}"))?;
            }
            "-h" | "--help" => {
                usage();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument '{arg}'")),
        }
    }

    let partition_path = partition_path.ok_or_else(|| "missing --partition".to_string())?;

    Ok(Args {
        cards_path,
        partition_path,
        limit,
        top,
    })
}

fn read_partition_names(path: &str) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&raw)?;
    let entries = json
        .get("entries")
        .and_then(|value| value.as_array())
        .ok_or_else(|| std::io::Error::other("partition report missing entries[]"))?;
    let mut out = HashSet::new();
    for entry in entries {
        if let Some(name) = entry.get("name").and_then(|value| value.as_str()) {
            out.insert(name.to_string());
        }
    }
    Ok(out)
}

fn is_stream_metadata_line(line: &str) -> bool {
    line.starts_with("Mana cost:")
        || line.starts_with("Type:")
        || line.starts_with("Power/Toughness:")
        || line.starts_with("Loyalty:")
        || line.starts_with("Defense:")
}

fn parse_stream_block(lines: &[String]) -> Option<CardInput> {
    if lines.is_empty() {
        return None;
    }
    let name_line = lines[0].trim();
    let name = name_line.strip_prefix("Name: ")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let parse_lines = lines[1..].to_vec();
    let parse_input = parse_lines.join("\n").trim().to_string();
    if parse_input.is_empty() {
        return None;
    }

    // Validate oracle text exists in the stream (after metadata lines).
    let mut oracle_start = None;
    for (idx, line) in parse_lines.iter().enumerate() {
        if !is_stream_metadata_line(line.trim()) {
            oracle_start = Some(idx);
            break;
        }
    }
    let oracle_start = oracle_start?;
    let oracle_text = parse_lines[oracle_start..].join("\n").trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }

    Some(CardInput { name, parse_input })
}

fn load_card_inputs_from_stream(
    cards_path: &str,
) -> Result<Vec<CardInput>, Box<dyn std::error::Error>> {
    let scripts_dir = format!("{}/scripts", env!("CARGO_MANIFEST_DIR"));
    let python_code = r#"
import sys
from pathlib import Path

sys.path.insert(0, sys.argv[1])
from stream_scryfall_blocks import iter_json_array, build_block

cards_path = Path(sys.argv[2])
for card in iter_json_array(cards_path):
    block = build_block(card)
    if not block:
        continue
    print(block)
    print("---")
"#;
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(python_code)
        .arg(&scripts_dir)
        .arg(cards_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| {
        std::io::Error::other("failed to capture stream_scryfall_blocks.py stdout")
    })?;

    let mut cards = Vec::new();
    let mut block_lines = Vec::new();
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = line?;
        if line.trim() == "---" {
            if let Some(card) = parse_stream_block(&block_lines) {
                cards.push(card);
            }
            block_lines.clear();
            continue;
        }
        block_lines.push(line);
    }
    if let Some(card) = parse_stream_block(&block_lines) {
        cards.push(card);
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "scripts/stream_scryfall_blocks.py failed with status {status}"
        ))
        .into());
    }

    Ok(cards)
}

fn add_tally(map: &mut HashMap<String, HashSet<String>>, key: String, card: &str) {
    map.entry(key).or_default().insert(card.to_string());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let names = read_partition_names(&args.partition_path)?;
    println!(
        "Loaded {} partition names from {}",
        names.len(),
        args.partition_path
    );

    let cards = load_card_inputs_from_stream(&args.cards_path)?;

    let mut processed = 0usize;
    let mut parsed_ok = 0usize;
    let mut parse_failed = 0usize;
    let mut parse_failure_examples: Vec<(String, String)> = Vec::new();
    let mut cards_with_placeholder_static = 0usize;
    let mut cards_with_unimplemented_trigger = 0usize;
    let mut cards_with_both = 0usize;

    let mut placeholder_static_displays: HashMap<String, HashSet<String>> = HashMap::new();
    let mut unimplemented_trigger_descriptions: HashMap<String, HashSet<String>> = HashMap::new();

    for card_input in cards {
        if !names.contains(&card_input.name) {
            continue;
        }
        if let Some(limit) = args.limit
            && processed >= limit
        {
            break;
        }

        processed += 1;
        let parse_result = CardDefinitionBuilder::new(CardId::new(), &card_input.name)
            .parse_text(card_input.parse_input);
        let Ok(def) = parse_result else {
            parse_failed += 1;
            if parse_failure_examples.len() < 5 {
                parse_failure_examples
                    .push((card_input.name.clone(), format!("{:?}", parse_result.err())));
            }
            continue;
        };
        parsed_ok += 1;

        let mut has_placeholder_static = false;
        let mut has_unimplemented_trigger = false;
        for ability in &def.abilities {
            match &ability.kind {
                AbilityKind::Static(static_ability) => {
                    if matches!(
                        static_ability.id(),
                        StaticAbilityId::KeywordMarker
                            | StaticAbilityId::RuleTextPlaceholder
                            | StaticAbilityId::UnsupportedParserLine
                    ) {
                        has_placeholder_static = true;
                        add_tally(
                            &mut placeholder_static_displays,
                            static_ability.display().to_ascii_lowercase(),
                            &card_input.name,
                        );
                    }
                }
                AbilityKind::Triggered(triggered) => {
                    let trigger_dbg = format!("{:?}", triggered.trigger).to_ascii_lowercase();
                    if trigger_dbg.contains("unimplemented_trigger") {
                        has_unimplemented_trigger = true;
                        add_tally(
                            &mut unimplemented_trigger_descriptions,
                            triggered.trigger.display().to_ascii_lowercase(),
                            &card_input.name,
                        );
                    }
                }
                _ => {}
            }
        }

        if has_placeholder_static {
            cards_with_placeholder_static += 1;
        }
        if has_unimplemented_trigger {
            cards_with_unimplemented_trigger += 1;
        }
        if has_placeholder_static && has_unimplemented_trigger {
            cards_with_both += 1;
        }
    }

    println!("Audit complete");
    println!("- Cards processed (filtered): {processed}");
    println!("- Parsed successfully: {parsed_ok}");
    println!("- Parse failures: {parse_failed}");
    if !parse_failure_examples.is_empty() {
        println!("Parse failure examples:");
        for (name, err) in &parse_failure_examples {
            println!("  - {name}: {err}");
        }
    }
    println!("- Cards with placeholder static ability: {cards_with_placeholder_static}");
    println!("- Cards with unimplemented triggers: {cards_with_unimplemented_trigger}");
    println!("- Cards with both: {cards_with_both}");

    if !placeholder_static_displays.is_empty() {
        let mut rows: Vec<(String, usize)> = placeholder_static_displays
            .iter()
            .map(|(display, cards)| (display.clone(), cards.len()))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        println!("Top placeholder static ability displays:");
        for (display, count) in rows.into_iter().take(args.top) {
            println!("  - {count} cards: {display}");
        }
    }

    if !unimplemented_trigger_descriptions.is_empty() {
        let mut rows: Vec<(String, usize)> = unimplemented_trigger_descriptions
            .iter()
            .map(|(desc, cards)| (desc.clone(), cards.len()))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        println!("Top unimplemented trigger descriptions:");
        for (desc, count) in rows.into_iter().take(args.top) {
            println!("  - {count} cards: {desc}");
        }
    }

    Ok(())
}
