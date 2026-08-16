use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use ironsmith_compiler::{OracleGrammarDocument, OracleGrammarLine, parse_oracle_grammar_document};
use ironsmith_tools::{CardPayload, default_cards_path, default_db_path, load_canonical_cards};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug)]
struct Args {
    cards_path: String,
    db_path: Option<String>,
    limit: Option<usize>,
    json_out: Option<String>,
    md_out: Option<String>,
    examples_per_node: usize,
    top_md_children: usize,
    strict_only: bool,
}

#[derive(Debug, Clone)]
struct DbObservation {
    parse_status: String,
}

#[derive(Debug, Clone)]
struct Segment {
    kind: String,
    label: String,
}

#[derive(Debug, Clone, Serialize)]
struct JsonExample {
    card: String,
    text: String,
}

#[derive(Debug, Clone, Default)]
struct GrammarNode {
    kind: String,
    label: String,
    occurrences: usize,
    cards: BTreeSet<String>,
    grammar_status_counts: BTreeMap<String, usize>,
    db_parse_status_counts: BTreeMap<String, usize>,
    examples: Vec<JsonExample>,
    children: BTreeMap<String, GrammarNode>,
}

#[derive(Debug, Clone, Serialize)]
struct JsonGrammarNode {
    kind: String,
    label: String,
    occurrences: usize,
    card_count: usize,
    grammar_status_counts: BTreeMap<String, usize>,
    db_parse_status_counts: BTreeMap<String, usize>,
    examples: Vec<JsonExample>,
    children: Vec<JsonGrammarNode>,
}

#[derive(Debug, Clone, Serialize)]
struct JsonParseFailure {
    card: String,
    strict_error: String,
    fallback_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct JsonReport {
    cards_source: String,
    db_source: Option<String>,
    cards_processed: usize,
    grammar_status_counts: BTreeMap<String, usize>,
    db_parse_status_counts: BTreeMap<String, usize>,
    strict_parse_failures: usize,
    fallback_tokenized_cards: usize,
    parse_failures_sample: Vec<JsonParseFailure>,
    root: JsonGrammarNode,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    let cards = load_canonical_cards(&args.cards_path)?;
    let db_observations = if let Some(db_path) = args.db_path.as_deref() {
        load_db_observations(db_path)?
    } else {
        BTreeMap::new()
    };

    let mut root = GrammarNode {
        kind: "root".to_string(),
        label: "Oracle grammar corpus".to_string(),
        ..GrammarNode::default()
    };
    let mut grammar_status_counts = BTreeMap::new();
    let mut db_parse_status_counts = BTreeMap::new();
    let mut parse_failures_sample = Vec::new();
    let mut strict_parse_failures = 0usize;
    let mut fallback_tokenized_cards = 0usize;
    let mut processed = 0usize;

    for payload in cards.values().take(args.limit.unwrap_or(usize::MAX)) {
        processed += 1;
        if processed.is_multiple_of(1000) {
            eprintln!("[INFO] processed {processed} cards");
        }

        let db_observation = db_observations.get(&payload.name);
        if let Some(observation) = db_observation {
            *db_parse_status_counts
                .entry(observation.parse_status.clone())
                .or_default() += 1;
        }

        let strict = parse_oracle_grammar_document(&payload.name, &payload.parse_input, false);
        match strict {
            Ok(document) => {
                *grammar_status_counts
                    .entry("strict_cst".to_string())
                    .or_default() += 1;
                record_document(
                    &mut root,
                    payload,
                    &document,
                    "strict_cst",
                    db_observation,
                    args.examples_per_node,
                );
            }
            Err(strict_error) => {
                strict_parse_failures += 1;
                if !args.strict_only {
                    match parse_oracle_grammar_document(&payload.name, &payload.parse_input, true) {
                        Ok(document) => {
                            *grammar_status_counts
                                .entry("allow_unsupported_cst".to_string())
                                .or_default() += 1;
                            record_document(
                                &mut root,
                                payload,
                                &document,
                                "allow_unsupported_cst",
                                db_observation,
                                args.examples_per_node,
                            );
                            continue;
                        }
                        Err(fallback_error) => {
                            fallback_tokenized_cards += 1;
                            *grammar_status_counts
                                .entry("fallback_tokenized".to_string())
                                .or_default() += 1;
                            if parse_failures_sample.len() < 100 {
                                parse_failures_sample.push(JsonParseFailure {
                                    card: payload.name.clone(),
                                    strict_error: strict_error.to_string(),
                                    fallback_error: Some(fallback_error.to_string()),
                                });
                            }
                        }
                    }
                } else {
                    fallback_tokenized_cards += 1;
                    *grammar_status_counts
                        .entry("fallback_tokenized".to_string())
                        .or_default() += 1;
                    if parse_failures_sample.len() < 100 {
                        parse_failures_sample.push(JsonParseFailure {
                            card: payload.name.clone(),
                            strict_error: strict_error.to_string(),
                            fallback_error: None,
                        });
                    }
                }
                record_fallback_document(
                    &mut root,
                    payload,
                    db_observation,
                    args.examples_per_node,
                );
            }
        }
    }

    let report = JsonReport {
        cards_source: args.cards_path.clone(),
        db_source: args.db_path.clone(),
        cards_processed: processed,
        grammar_status_counts,
        db_parse_status_counts,
        strict_parse_failures,
        fallback_tokenized_cards,
        parse_failures_sample,
        root: root.to_json(),
    };

    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = args.json_out.as_deref() {
        fs::write(path, json.as_bytes())?;
        eprintln!("[INFO] wrote JSON report to {path}");
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(json.as_bytes())?;
        stdout.write_all(b"\n")?;
    }

    if let Some(path) = args.md_out.as_deref() {
        let markdown = render_markdown(&report, args.top_md_children);
        fs::write(path, markdown)?;
        eprintln!("[INFO] wrote Markdown report to {path}");
    }

    Ok(())
}

fn parse_args() -> Result<Args, Box<dyn Error>> {
    let mut cards_path = default_cards_path().display().to_string();
    let mut db_path = Some(default_db_path().display().to_string());
    let mut limit = None;
    let mut json_out = None;
    let mut md_out = None;
    let mut examples_per_node = 5usize;
    let mut top_md_children = 20usize;
    let mut strict_only = false;

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = iter.next().ok_or("--cards requires a path")?;
            }
            "--db" => {
                db_path = Some(iter.next().ok_or("--db requires a path")?);
            }
            "--no-db" => {
                db_path = None;
            }
            "--limit" => {
                let raw = iter.next().ok_or("--limit requires a number")?;
                limit = Some(raw.parse::<usize>()?);
            }
            "--json-out" => {
                json_out = Some(iter.next().ok_or("--json-out requires a path")?);
            }
            "--md-out" => {
                md_out = Some(iter.next().ok_or("--md-out requires a path")?);
            }
            "--examples" => {
                let raw = iter.next().ok_or("--examples requires a number")?;
                examples_per_node = raw.parse::<usize>()?;
            }
            "--top-md-children" => {
                let raw = iter.next().ok_or("--top-md-children requires a number")?;
                top_md_children = raw.parse::<usize>()?;
            }
            "--strict-only" => {
                strict_only = true;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }

    Ok(Args {
        cards_path,
        db_path,
        limit,
        json_out,
        md_out,
        examples_per_node,
        top_md_children,
        strict_only,
    })
}

fn print_help() {
    eprintln!(
        "Usage: cargo run -p ironsmith-tools --bin export_oracle_grammar_tree -- \\
  [--cards cards.json] [--db reports/engine-status.sqlite3|--no-db] \\
  [--limit N] [--json-out path] [--md-out path] [--examples N] \\
  [--top-md-children N] [--strict-only]"
    );
}

fn load_db_observations(path: &str) -> Result<BTreeMap<String, DbObservation>, Box<dyn Error>> {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        eprintln!("[WARN] DB path does not exist, continuing without DB observations: {path}");
        return Ok(BTreeMap::new());
    }
    let conn = Connection::open(path)?;
    let mut stmt = conn.prepare(
        "SELECT card_name, parse_status
         FROM latest_card_compilation
         ORDER BY card_name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            DbObservation {
                parse_status: row.get::<_, String>(1)?,
            },
        ))
    })?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (name, observation) = row?;
        out.insert(name, observation);
    }
    Ok(out)
}

fn record_document(
    root: &mut GrammarNode,
    payload: &CardPayload,
    document: &OracleGrammarDocument,
    grammar_status: &str,
    db_observation: Option<&DbObservation>,
    examples_per_node: usize,
) {
    let mut sequence = Vec::new();
    for line in &document.lines {
        sequence.push(line_family(line));
        for (path, text) in line_paths(payload, line) {
            root.record_path(
                &path,
                &payload.name,
                grammar_status,
                db_observation,
                &text,
                examples_per_node,
            );
        }
    }
    root.record_path(
        &[
            segment("document", "CST line sequence"),
            segment("sequence", sequence.join(" > ")),
        ],
        &payload.name,
        grammar_status,
        db_observation,
        &payload.oracle_text,
        examples_per_node,
    );
}

fn record_fallback_document(
    root: &mut GrammarNode,
    payload: &CardPayload,
    db_observation: Option<&DbObservation>,
    examples_per_node: usize,
) {
    let mut sequence = Vec::new();
    for line in payload
        .parse_input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let family = fallback_line_family(line);
        sequence.push(family.clone());
        let shape = abstract_text(&payload.name, line);
        let head = first_words(&shape, 5);
        root.record_path(
            &[
                segment("fallback_line_family", family),
                segment("head", head),
                segment("shape", shape),
            ],
            &payload.name,
            "fallback_tokenized",
            db_observation,
            line,
            examples_per_node,
        );
    }
    root.record_path(
        &[
            segment("document", "Fallback line sequence"),
            segment("sequence", sequence.join(" > ")),
        ],
        &payload.name,
        "fallback_tokenized",
        db_observation,
        &payload.oracle_text,
        examples_per_node,
    );
}

fn line_paths(payload: &CardPayload, line: &OracleGrammarLine) -> Vec<(Vec<Segment>, String)> {
    match line {
        OracleGrammarLine::Metadata { kind, value } => vec![(
            vec![
                segment("line_family", "Metadata"),
                segment("metadata_kind", kind),
                segment("shape", abstract_text(&payload.name, value)),
            ],
            value.clone(),
        )],
        OracleGrammarLine::Keyword {
            kind,
            text,
            parse_text,
            ..
        } => vec![(
            vec![
                segment("line_family", "Keyword"),
                segment("keyword_kind", kind),
                segment("shape", abstract_text(&payload.name, parse_text)),
            ],
            text.clone(),
        )],
        OracleGrammarLine::Activated {
            cost_text,
            cost_debug,
            effect_text,
            effect_parse_text,
            chosen_option_label,
            ..
        } => vec![(
            vec![
                segment("line_family", "Activated"),
                segment("cost_shape", abstract_text(&payload.name, cost_text)),
                segment("cost_ast", debug_head(cost_debug)),
                segment(
                    "effect_shape",
                    abstract_text(&payload.name, effect_parse_text),
                ),
                segment(
                    "labeled_option",
                    option_label(chosen_option_label.as_deref()),
                ),
            ],
            effect_text.clone(),
        )],
        OracleGrammarLine::Triggered {
            trigger_text,
            trigger_parse_text,
            effect_text,
            effect_parse_text,
            intervening_if_debug,
            max_triggers_per_turn,
            chosen_option_label,
            ..
        } => vec![(
            vec![
                segment("line_family", "Triggered"),
                segment(
                    "trigger_shape",
                    abstract_text(&payload.name, trigger_parse_text),
                ),
                segment(
                    "intervening_if",
                    if intervening_if_debug.is_some() {
                        "present"
                    } else {
                        "absent"
                    },
                ),
                segment(
                    "max_triggers_per_turn",
                    max_triggers_per_turn
                        .map(|count| count.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                ),
                segment(
                    "effect_shape",
                    abstract_text(&payload.name, effect_parse_text),
                ),
                segment(
                    "labeled_option",
                    option_label(chosen_option_label.as_deref()),
                ),
            ],
            format!("{trigger_text}, {effect_text}"),
        )],
        OracleGrammarLine::Static {
            text,
            parse_text,
            chosen_option_label,
            ..
        } => vec![(
            vec![
                segment("line_family", "Static"),
                segment("shape", abstract_text(&payload.name, parse_text)),
                segment(
                    "labeled_option",
                    option_label(chosen_option_label.as_deref()),
                ),
            ],
            text.clone(),
        )],
        OracleGrammarLine::Statement {
            text,
            parse_text,
            parse_group_texts,
            ..
        } => {
            let mut paths = vec![(
                vec![
                    segment("line_family", "Statement"),
                    segment("shape", abstract_text(&payload.name, parse_text)),
                ],
                text.clone(),
            )];
            for group in parse_group_texts {
                paths.push((
                    vec![
                        segment("line_family", "Statement group"),
                        segment("shape", abstract_text(&payload.name, group)),
                    ],
                    group.clone(),
                ));
            }
            paths
        }
        OracleGrammarLine::Modal { modes, .. } => {
            let mut paths = vec![(
                vec![
                    segment("line_family", "Modal"),
                    segment("mode_count", modes.len().to_string()),
                ],
                format!("{} modes", modes.len()),
            )];
            for mode in modes {
                paths.push((
                    vec![
                        segment("line_family", "Modal mode"),
                        segment("shape", abstract_text(&payload.name, &mode.text)),
                        segment("effect_ast", debug_heads(&mode.effects_debug)),
                    ],
                    mode.text.clone(),
                ));
            }
            paths
        }
        OracleGrammarLine::LevelHeader {
            min_level,
            max_level,
            pt,
            items,
        } => {
            let range = match max_level {
                Some(max) => format!("{min_level}-{max}"),
                None => format!("{min_level}+"),
            };
            let mut paths = vec![(
                vec![
                    segment("line_family", "LevelHeader"),
                    segment("level_range", range),
                    segment(
                        "pt",
                        pt.map(|(power, toughness)| format!("{power}/{toughness}"))
                            .unwrap_or_else(|| "none".to_string()),
                    ),
                ],
                "level header".to_string(),
            )];
            for item in items {
                paths.push((
                    vec![
                        segment("line_family", "Level item"),
                        segment("item_kind", &item.kind),
                        segment("shape", abstract_text(&payload.name, &item.text)),
                        segment("parsed_ast", debug_head(&item.parsed_debug)),
                    ],
                    item.text.clone(),
                ));
            }
            paths
        }
        OracleGrammarLine::SagaChapter {
            chapters,
            text,
            effects_debug,
            ..
        } => vec![(
            vec![
                segment("line_family", "SagaChapter"),
                segment(
                    "chapters",
                    chapters
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
                segment("shape", abstract_text(&payload.name, text)),
                segment("effect_ast", debug_heads(effects_debug)),
            ],
            text.clone(),
        )],
        OracleGrammarLine::Unsupported { info, reason_code } => vec![(
            vec![
                segment("line_family", "Unsupported"),
                segment("reason_code", reason_code),
                segment("shape", abstract_text(&payload.name, &info.normalized_line)),
            ],
            info.raw_line.clone(),
        )],
    }
}

fn line_family(line: &OracleGrammarLine) -> String {
    match line {
        OracleGrammarLine::Metadata { .. } => "Metadata",
        OracleGrammarLine::Keyword { .. } => "Keyword",
        OracleGrammarLine::Activated { .. } => "Activated",
        OracleGrammarLine::Triggered { .. } => "Triggered",
        OracleGrammarLine::Static { .. } => "Static",
        OracleGrammarLine::Statement { .. } => "Statement",
        OracleGrammarLine::Modal { .. } => "Modal",
        OracleGrammarLine::LevelHeader { .. } => "LevelHeader",
        OracleGrammarLine::SagaChapter { .. } => "SagaChapter",
        OracleGrammarLine::Unsupported { .. } => "Unsupported",
    }
    .to_string()
}

fn fallback_line_family(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("mana cost:")
        || lower.starts_with("type:")
        || lower.starts_with("type line:")
        || lower.starts_with("power/toughness:")
        || lower.starts_with("loyalty:")
        || lower.starts_with("defense:")
    {
        "Metadata".to_string()
    } else if lower.starts_with("when ")
        || lower.starts_with("whenever ")
        || lower.starts_with("at ")
    {
        "Triggered-like".to_string()
    } else if line.trim_start().starts_with(['•', '*', '-']) {
        "Modal-bullet-like".to_string()
    } else if line.contains(':') {
        "Colon-line-like".to_string()
    } else if lower.starts_with(['i', 'v', 'x']) && lower.contains('—') {
        "Saga-like".to_string()
    } else {
        "Sentence-like".to_string()
    }
}

fn segment(kind: impl Into<String>, label: impl Into<String>) -> Segment {
    Segment {
        kind: kind.into(),
        label: truncate_label(label.into(), 240),
    }
}

fn option_label(label: Option<&str>) -> String {
    label.unwrap_or("none").to_string()
}

fn debug_heads(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    values
        .iter()
        .map(|value| debug_head(value))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn debug_head(value: &str) -> String {
    let first = value
        .split([' ', '{', '('])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(value);
    truncate_label(first.to_string(), 120)
}

fn first_words(text: &str, count: usize) -> String {
    text.split_whitespace()
        .take(count)
        .collect::<Vec<_>>()
        .join(" ")
}

fn abstract_text(card_name: &str, text: &str) -> String {
    let mut source = text.to_ascii_lowercase();
    source = source
        .replace(['’', '‘'], "'")
        .replace(['“', '”'], "\"")
        .replace('—', " — ");
    for alias in card_name_aliases(card_name) {
        source = source.replace(alias.as_str(), "~");
    }

    let chars = source.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut idx = 0usize;
    while idx < chars.len() {
        let ch = chars[idx];
        if ch.is_whitespace() {
            idx += 1;
            continue;
        }
        if ch == '{' {
            idx += 1;
            while idx < chars.len() && chars[idx] != '}' {
                idx += 1;
            }
            if idx < chars.len() {
                idx += 1;
            }
            tokens.push("{MANA}".to_string());
            continue;
        }
        if ch == '"' {
            idx += 1;
            while idx < chars.len() && chars[idx] != '"' {
                idx += 1;
            }
            if idx < chars.len() {
                idx += 1;
            }
            tokens.push("<QUOTE>".to_string());
            continue;
        }
        if ch.is_ascii_digit()
            || ((ch == '+' || ch == '-')
                && chars.get(idx + 1).is_some_and(|next| next.is_ascii_digit()))
        {
            idx += 1;
            while idx < chars.len()
                && (chars[idx].is_ascii_digit() || matches!(chars[idx], '/' | '+' | '-'))
            {
                idx += 1;
            }
            tokens.push("<N>".to_string());
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '\'' || ch == '~' {
            let start = idx;
            idx += 1;
            while idx < chars.len()
                && (chars[idx].is_ascii_alphabetic() || chars[idx] == '\'' || chars[idx] == '~')
            {
                idx += 1;
            }
            tokens.push(chars[start..idx].iter().collect());
            continue;
        }
        if matches!(ch, ',' | '.' | ':' | ';' | '—' | '(' | ')' | '/' | '+') {
            tokens.push(ch.to_string());
        }
        idx += 1;
    }

    truncate_label(tokens.join(" "), 240)
}

fn card_name_aliases(name: &str) -> Vec<String> {
    let lower = name.to_ascii_lowercase();
    let mut aliases = vec![lower.clone()];
    if let Some((first, _)) = lower.split_once(',') {
        let first = first.trim();
        if first.len() > 1 {
            aliases.push(first.to_string());
        }
    }
    if let Some((first, _)) = lower.split_once(" // ") {
        let first = first.trim();
        if first.len() > 1 {
            aliases.push(first.to_string());
        }
    }
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.len()));
    aliases.dedup();
    aliases
}

fn truncate_label(mut label: String, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label;
    }
    label = label.chars().take(max_chars.saturating_sub(3)).collect();
    label.push_str("...");
    label
}

impl GrammarNode {
    fn record_path(
        &mut self,
        path: &[Segment],
        card_name: &str,
        grammar_status: &str,
        db_observation: Option<&DbObservation>,
        example_text: &str,
        examples_per_node: usize,
    ) {
        self.touch(
            card_name,
            grammar_status,
            db_observation,
            example_text,
            examples_per_node,
        );
        let mut node = self;
        for segment in path {
            let key = format!("{}\u{1f}{}", segment.kind, segment.label);
            node = node.children.entry(key).or_insert_with(|| GrammarNode {
                kind: segment.kind.clone(),
                label: segment.label.clone(),
                ..GrammarNode::default()
            });
            node.touch(
                card_name,
                grammar_status,
                db_observation,
                example_text,
                examples_per_node,
            );
        }
    }

    fn touch(
        &mut self,
        card_name: &str,
        grammar_status: &str,
        db_observation: Option<&DbObservation>,
        example_text: &str,
        examples_per_node: usize,
    ) {
        self.occurrences += 1;
        self.cards.insert(card_name.to_string());
        *self
            .grammar_status_counts
            .entry(grammar_status.to_string())
            .or_default() += 1;
        if let Some(observation) = db_observation {
            *self
                .db_parse_status_counts
                .entry(observation.parse_status.clone())
                .or_default() += 1;
        }
        if self.examples.len() < examples_per_node
            && !self
                .examples
                .iter()
                .any(|example| example.card == card_name)
        {
            self.examples.push(JsonExample {
                card: card_name.to_string(),
                text: truncate_label(example_text.trim().to_string(), 300),
            });
        }
    }

    fn to_json(&self) -> JsonGrammarNode {
        let mut children = self
            .children
            .values()
            .map(GrammarNode::to_json)
            .collect::<Vec<_>>();
        children.sort_by(|left, right| {
            right
                .occurrences
                .cmp(&left.occurrences)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.label.cmp(&right.label))
        });
        JsonGrammarNode {
            kind: self.kind.clone(),
            label: self.label.clone(),
            occurrences: self.occurrences,
            card_count: self.cards.len(),
            grammar_status_counts: self.grammar_status_counts.clone(),
            db_parse_status_counts: self.db_parse_status_counts.clone(),
            examples: self.examples.clone(),
            children,
        }
    }
}

fn render_markdown(report: &JsonReport, top_children: usize) -> String {
    let mut out = String::new();
    out.push_str("# Oracle Grammar Tree\n\n");
    out.push_str(&format!("- Cards processed: {}\n", report.cards_processed));
    out.push_str(&format!(
        "- Strict parse failures: {}\n",
        report.strict_parse_failures
    ));
    out.push_str(&format!(
        "- Fallback-tokenized cards: {}\n",
        report.fallback_tokenized_cards
    ));
    out.push_str("- Grammar statuses:\n");
    for (status, count) in &report.grammar_status_counts {
        out.push_str(&format!("  - {status}: {count}\n"));
    }
    if !report.db_parse_status_counts.is_empty() {
        out.push_str("- DB parse statuses:\n");
        for (status, count) in &report.db_parse_status_counts {
            out.push_str(&format!("  - {status}: {count}\n"));
        }
    }

    out.push_str("\n## Top Branches\n\n");
    for child in report.root.children.iter().take(top_children) {
        render_markdown_node(&mut out, child, 3, top_children);
    }

    if !report.parse_failures_sample.is_empty() {
        out.push_str("\n## Parse Failure Sample\n\n");
        for failure in report.parse_failures_sample.iter().take(20) {
            out.push_str(&format!(
                "- {}: {}\n",
                failure.card,
                truncate_label(failure.strict_error.clone(), 220)
            ));
        }
    }
    out
}

fn render_markdown_node(
    out: &mut String,
    node: &JsonGrammarNode,
    depth: usize,
    top_children: usize,
) {
    let heading = "#".repeat(depth.min(6));
    out.push_str(&format!(
        "{heading} {}: {}\n\n",
        node.kind,
        markdown_escape(&node.label)
    ));
    out.push_str(&format!(
        "- Occurrences: {}\n- Cards: {}\n",
        node.occurrences, node.card_count
    ));
    if !node.examples.is_empty() {
        out.push_str("- Examples: ");
        out.push_str(
            &node
                .examples
                .iter()
                .take(3)
                .map(|example| example.card.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push('\n');
    }
    out.push('\n');

    if depth >= 4 {
        return;
    }
    for child in node.children.iter().take(top_children) {
        render_markdown_node(out, child, depth + 1, top_children);
    }
}

fn markdown_escape(text: &str) -> String {
    text.replace('\n', " ").replace('|', "\\|")
}
