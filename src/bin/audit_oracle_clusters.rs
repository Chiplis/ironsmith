use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use ironsmith::cards::{CardDefinitionBuilder, generated_definition_has_unimplemented_content};
use ironsmith::compiled_text::compiled_lines;
use ironsmith::ids::CardId;
use ironsmith::semantic_compare::{
    clause_comparison_tokens as shared_clause_comparison_tokens,
    compare_semantics_scored as shared_compare_semantics_scored,
    semantic_clauses_for_compare as shared_semantic_clauses,
};
use ironsmith_tools::{CardStatusDb, CompilationSnapshot, ParseStatus, default_db_path};

mod tooling_paths;

#[derive(Debug)]
struct Args {
    cards_path: Option<String>,
    limit: Option<usize>,
    min_cluster_size: usize,
    top_clusters: usize,
    examples_per_cluster: usize,
    json_out: Option<String>,
    parser_trace: bool,
    trace_name: Option<String>,
    allow_unsupported: bool,
    use_embeddings: bool,
    embedding_dims: usize,
    embedding_threshold: f32,
    mismatch_names_out: Option<String>,
    false_positive_names: Option<String>,
    failures_out: Option<String>,
    audits_out: Option<String>,
    cluster_csv_out: Option<String>,
    parse_errors_csv_out: Option<String>,
    parse_error_summary_csv_out: Option<String>,
    db_path: String,
    no_db: bool,
}

#[derive(Debug, Clone)]
struct CardInput {
    name: String,
    oracle_text: String,
    parse_input: String,
}

#[derive(Debug, Clone)]
struct CardAudit {
    name: String,
    oracle_text: String,
    cluster_key: String,
    parse_error: Option<String>,
    compiled_lines: Vec<String>,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    semantic_mismatch: bool,
    semantic_false_positive: bool,
    has_unimplemented: bool,
}

#[derive(Debug)]
struct JsonReport {
    cards_processed: usize,
    parse_failures: usize,
    semantic_mismatches: usize,
    semantic_false_positives: usize,
    clusters_total: usize,
    clusters_reported: usize,
    clusters: Vec<JsonCluster>,
}

#[derive(Debug)]
struct JsonCluster {
    signature: String,
    size: usize,
    parse_failures: usize,
    semantic_mismatches: usize,
    semantic_false_positives: usize,
    parse_failure_rate: f32,
    semantic_mismatch_rate: f32,
    top_errors: Vec<JsonErrorCount>,
    examples: Vec<JsonExample>,
}

#[derive(Debug)]
struct JsonErrorCount {
    error: String,
    count: usize,
}

#[derive(Debug)]
struct JsonExample {
    name: String,
    parse_error: Option<String>,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    oracle_excerpt: String,
    compiled_excerpt: String,
    oracle_text: String,
    compiled_lines: Vec<String>,
}

#[derive(Debug)]
struct JsonFailureReport {
    threshold: f32,
    cards_processed: usize,
    failures: usize,
    entries: Vec<JsonFailureEntry>,
}

#[derive(Debug)]
struct JsonAuditsReport {
    threshold: f32,
    embedding_dims: usize,
    cards_processed: usize,
    parse_failures: usize,
    semantic_mismatches: usize,
    semantic_false_positives: usize,
    parse_success_with_unimplemented: usize,
    entries: Vec<JsonAuditEntry>,
}

#[derive(Debug)]
struct JsonAuditEntry {
    name: String,
    parse_error: Option<String>,
    semantic_mismatch: bool,
    semantic_false_positive: bool,
    has_unimplemented: bool,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
}

#[derive(Debug)]
struct JsonFailureEntry {
    name: String,
    parse_error: Option<String>,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    oracle_text: String,
    compiled_text: String,
    compiled_lines: Vec<String>,
}

#[derive(Clone, Copy)]
struct ParseFailureClusterRef<'a> {
    signature: &'a str,
    entries: &'a [CardAudit],
    size: usize,
    parse_failures: usize,
    semantic_mismatches: usize,
    semantic_false_positives: usize,
    parse_failure_rate: f32,
    semantic_mismatch_rate: f32,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = None;
    let mut limit = None;
    let mut min_cluster_size = 8usize;
    let mut top_clusters = 30usize;
    let mut examples_per_cluster = 3usize;
    let mut json_out = None;
    let mut parser_trace = false;
    let mut trace_name = None;
    let mut allow_unsupported = false;
    let mut use_embeddings = false;
    let mut embedding_dims = 384usize;
    let mut embedding_threshold = 0.17f32;
    let mut mismatch_names_out = None;
    let mut false_positive_names = None;
    let mut failures_out = None;
    let mut audits_out = None;
    let mut cluster_csv_out = None;
    let mut parse_errors_csv_out = None;
    let mut parse_error_summary_csv_out = None;
    let mut db_path = default_db_path().display().to_string();
    let mut no_db = false;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = Some(
                    iter.next()
                        .ok_or_else(|| "--cards requires a path".to_string())?,
                );
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
            "--min-cluster-size" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--min-cluster-size requires a number".to_string())?;
                min_cluster_size = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --min-cluster-size value '{raw}': {e}"))?;
            }
            "--top-clusters" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--top-clusters requires a number".to_string())?;
                top_clusters = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --top-clusters value '{raw}': {e}"))?;
            }
            "--examples" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--examples requires a number".to_string())?;
                examples_per_cluster = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --examples value '{raw}': {e}"))?;
            }
            "--json-out" => {
                json_out = Some(
                    iter.next()
                        .ok_or_else(|| "--json-out requires a path".to_string())?,
                );
            }
            "--parser-trace" => {
                parser_trace = true;
            }
            "--trace-name" => {
                trace_name = Some(
                    iter.next()
                        .ok_or_else(|| "--trace-name requires a card-name substring".to_string())?
                        .to_ascii_lowercase(),
                );
            }
            "--allow-unsupported" => {
                allow_unsupported = true;
            }
            "--use-embeddings" => {
                use_embeddings = true;
            }
            "--embedding-dims" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--embedding-dims requires a number".to_string())?;
                embedding_dims = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --embedding-dims value '{raw}': {e}"))?;
            }
            "--embedding-threshold" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--embedding-threshold requires a float".to_string())?;
                embedding_threshold = raw
                    .parse::<f32>()
                    .map_err(|e| format!("invalid --embedding-threshold value '{raw}': {e}"))?;
            }
            "--mismatch-names-out" => {
                mismatch_names_out = Some(
                    iter.next()
                        .ok_or_else(|| "--mismatch-names-out requires a path".to_string())?,
                );
            }
            "--false-positive-names" => {
                false_positive_names = Some(
                    iter.next()
                        .ok_or_else(|| "--false-positive-names requires a path".to_string())?,
                );
            }
            "--failures-out" => {
                failures_out = Some(
                    iter.next()
                        .ok_or_else(|| "--failures-out requires a path".to_string())?,
                );
            }
            "--audits-out" => {
                audits_out = Some(
                    iter.next()
                        .ok_or_else(|| "--audits-out requires a path".to_string())?,
                );
            }
            "--cluster-csv-out" => {
                cluster_csv_out = Some(
                    iter.next()
                        .ok_or_else(|| "--cluster-csv-out requires a path".to_string())?,
                );
            }
            "--parse-errors-csv-out" => {
                parse_errors_csv_out = Some(
                    iter.next()
                        .ok_or_else(|| "--parse-errors-csv-out requires a path".to_string())?,
                );
            }
            "--parse-error-summary-csv-out" => {
                parse_error_summary_csv_out =
                    Some(iter.next().ok_or_else(|| {
                        "--parse-error-summary-csv-out requires a path".to_string()
                    })?);
            }
            "--db-path" => {
                db_path = iter
                    .next()
                    .ok_or_else(|| "--db-path requires a path".to_string())?;
            }
            "--no-db" => {
                no_db = true;
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. supported: --cards <path> --limit <n> --min-cluster-size <n> --top-clusters <n> --examples <n> --json-out <path> --parser-trace --trace-name <substring> --allow-unsupported --use-embeddings --embedding-dims <n> --embedding-threshold <f32> --mismatch-names-out <path> --false-positive-names <path> --failures-out <path> --audits-out <path> --cluster-csv-out <path> --parse-errors-csv-out <path> --parse-error-summary-csv-out <path> --db-path <path> --no-db"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        limit,
        min_cluster_size,
        top_clusters,
        examples_per_cluster,
        json_out,
        parser_trace,
        trace_name,
        allow_unsupported,
        use_embeddings,
        embedding_dims,
        embedding_threshold,
        mismatch_names_out,
        false_positive_names,
        failures_out,
        audits_out,
        cluster_csv_out,
        parse_errors_csv_out,
        parse_error_summary_csv_out,
        db_path,
        no_db,
    })
}

fn semantic_clauses(text: &str) -> Vec<String> {
    shared_semantic_clauses(text)
}

fn replace_case_insensitive(text: &str, needle: &str, replacement: &str) -> String {
    let replacement_text = text.to_string();
    let haystack = replacement_text.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    if needle.is_empty() {
        return replacement_text;
    }
    if !haystack.contains(&needle) {
        return replacement_text;
    }

    let mut result = String::with_capacity(replacement_text.len());
    let mut src_idx = 0usize;
    let mut search_idx = 0usize;

    while let Some(found) = haystack[search_idx..].find(&needle) {
        let abs_idx = search_idx + found;
        result.push_str(&replacement_text[src_idx..abs_idx]);
        result.push_str(replacement);
        src_idx = abs_idx + needle.len();
        search_idx = src_idx;
    }

    result.push_str(&replacement_text[src_idx..]);
    result
}

fn is_cluster_keyword(token: &str) -> bool {
    matches!(
        token,
        "<mana>"
            | "<num>"
            | "<pt>"
            | "target"
            | "each"
            | "any"
            | "all"
            | "you"
            | "opponent"
            | "player"
            | "creature"
            | "artifact"
            | "enchantment"
            | "planeswalker"
            | "battle"
            | "permanent"
            | "spell"
            | "card"
            | "token"
            | "counter"
            | "draw"
            | "discard"
            | "gain"
            | "lose"
            | "destroy"
            | "exile"
            | "return"
            | "search"
            | "shuffle"
            | "reveal"
            | "mill"
            | "surveil"
            | "scry"
            | "look"
            | "choose"
            | "sacrifice"
            | "add"
            | "pay"
            | "cast"
            | "countered"
            | "put"
            | "remove"
            | "move"
            | "deal"
            | "damage"
            | "tap"
            | "untap"
            | "attack"
            | "block"
            | "regenerate"
            | "copy"
            | "transform"
            | "investigate"
            | "proliferate"
            | "vote"
            | "if"
            | "unless"
            | "when"
            | "whenever"
            | "at"
            | "beginning"
            | "end"
            | "step"
            | "turn"
            | "until"
            | "while"
            | "as"
            | "instead"
            | "where"
            | "for"
            | "this"
            | "that"
            | "it"
            | "those"
            | "their"
            | "your"
            | "can"
            | "cant"
            | "not"
            | "becomes"
            | "become"
            | "has"
            | "have"
            | "get"
            | "gets"
            | "enters"
            | "dies"
            | "graveyard"
            | "library"
            | "hand"
            | "battlefield"
            | "zone"
            | "owner"
            | "choice"
            | "controller"
            | "life"
            | "power"
            | "toughness"
    )
}

fn clause_signature(clause: &str) -> String {
    let tokens = shared_clause_comparison_tokens(clause);
    if tokens.len() <= 3 {
        return if tokens.is_empty() {
            "<empty>".to_string()
        } else {
            tokens.join(" ")
        };
    }

    let mut out = Vec::new();
    for token in tokens {
        let mapped = if is_cluster_keyword(&token) {
            token
        } else if token.len() > 14 {
            "<arg>".to_string()
        } else {
            token
        };
        if out.last().is_some_and(|last| last == &mapped) {
            continue;
        }
        out.push(mapped);
    }
    if out.is_empty() {
        "<empty>".to_string()
    } else {
        out.join(" ")
    }
}

#[derive(Debug, Clone, Copy)]
struct EmbeddingConfig {
    dims: usize,
    mismatch_threshold: f32,
}

fn to_shared_embedding_config(
    embedding: Option<EmbeddingConfig>,
) -> Option<ironsmith::semantic_compare::EmbeddingConfig> {
    embedding.map(|cfg| ironsmith::semantic_compare::EmbeddingConfig {
        dims: cfg.dims,
        mismatch_threshold: cfg.mismatch_threshold,
    })
}

fn cluster_key(oracle_text: &str) -> String {
    let mut parts: Vec<String> = semantic_clauses(oracle_text)
        .into_iter()
        .map(|clause| clause_signature(&clause))
        .filter(|part| part != "<empty>")
        .collect();
    parts.sort();
    if parts.is_empty() {
        "0|<empty>".to_string()
    } else {
        format!("{}|{}", parts.len(), parts.join(" || "))
    }
}

fn is_compiled_heading_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim().to_ascii_lowercase();
    prefix == "spell effects"
        || prefix.starts_with("activated ability ")
        || prefix.starts_with("triggered ability ")
        || prefix.starts_with("static ability ")
        || prefix.starts_with("keyword ability ")
        || prefix.starts_with("mana ability ")
        || prefix.starts_with("ability ")
        || prefix.starts_with("alternative cast ")
}

fn strip_compiled_prefix(line: &str) -> &str {
    let Some((prefix, rest)) = line.split_once(':') else {
        return line;
    };
    if is_compiled_heading_prefix(prefix) {
        rest.trim()
    } else {
        line
    }
}

fn normalize_card_self_references(text: &str, card_name: &str) -> String {
    let full_name = card_name.trim();
    if full_name.is_empty() {
        return text.to_string();
    }

    let left_half = full_name
        .split("//")
        .next()
        .map(str::trim)
        .unwrap_or(full_name);
    let short_name = left_half
        .split(',')
        .next()
        .map(str::trim)
        .unwrap_or(left_half);

    let mut names = vec![full_name, left_half, short_name];
    if let Some(stripped) = full_name
        .strip_prefix("A-")
        .or_else(|| full_name.strip_prefix("a-"))
    {
        names.push(stripped);
    }
    if let Some(stripped) = left_half
        .strip_prefix("A-")
        .or_else(|| left_half.strip_prefix("a-"))
    {
        names.push(stripped);
    }
    if let Some(stripped) = short_name
        .strip_prefix("A-")
        .or_else(|| short_name.strip_prefix("a-"))
    {
        names.push(stripped);
    }
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    names.dedup();

    let mut normalized = text.to_string();
    for name in names {
        if name.len() < 3 {
            continue;
        }
        let possessive = format!("{name}'s");
        normalized = replace_case_insensitive(&normalized, &possessive, "this");
        normalized = replace_case_insensitive(&normalized, &possessive.replace('\'', "’"), "this");
        normalized = replace_case_insensitive(&normalized, name, "this");
    }
    if let Some(lead) = short_name.split_whitespace().next() {
        let lead = lead.trim();
        if lead.len() >= 3 {
            let lead_or = format!("{lead} or Whenever");
            normalized = replace_case_insensitive(&normalized, &lead_or, "this or Whenever");
            normalized = replace_case_insensitive(
                &normalized,
                &lead_or.to_ascii_lowercase(),
                "this or whenever",
            );
        }
    }
    normalized = normalized
        .replace("That object's controller", "its controller")
        .replace("that object's controller", "its controller");
    normalized
}

fn split_lose_all_abilities_subject(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_end_matches('.');
    trimmed
        .strip_suffix(" loses all abilities")
        .or_else(|| trimmed.strip_suffix(" lose all abilities"))
        .map(str::trim)
}

fn extract_base_pt_tail_for_subject(line: &str, subject: &str) -> Option<String> {
    if let Some(pt) = line.strip_prefix("Affected permanents have base power and toughness ") {
        return Some(pt.trim().to_string());
    }
    for verb in ["has", "have"] {
        let prefix = format!("{subject} {verb} base power and toughness ");
        if let Some(pt) = line.strip_prefix(&prefix) {
            return Some(pt.trim().to_string());
        }
    }
    None
}

fn split_mana_add_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let (cost, effect) = trimmed.split_once(':')?;
    let add_tail = effect.trim().strip_prefix("Add ")?;
    let add_tail = add_tail.trim();
    if add_tail.is_empty() || add_tail.contains('.') || add_tail.contains(';') {
        return None;
    }
    Some((cost.trim().to_string(), add_tail.to_string()))
}

fn merge_simple_mana_add_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some((base_cost, first_add)) = split_mana_add_line(&lines[idx]) {
            let mut adds = vec![first_add];
            let mut consumed = 1usize;
            while idx + consumed < lines.len() {
                let Some((next_cost, next_add)) = split_mana_add_line(&lines[idx + consumed])
                else {
                    break;
                };
                if !next_cost.eq_ignore_ascii_case(&base_cost) {
                    break;
                }
                if !adds
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&next_add))
                {
                    adds.push(next_add);
                }
                consumed += 1;
            }
            if adds.len() >= 2 {
                merged.push(format!("{base_cost}: Add {}", adds.join(" or ")));
                idx += consumed;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn merge_blockability_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let is_pair = (left.eq_ignore_ascii_case("This creature can't block")
                && right.eq_ignore_ascii_case("This creature can't be blocked"))
                || (left.eq_ignore_ascii_case("Can't block")
                    && right.eq_ignore_ascii_case("Can't be blocked"));
            if is_pair {
                merged.push("This creature can't block and can't be blocked".to_string());
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn is_destroy_all_clause(line: &str) -> Option<&'static str> {
    let normalized = strip_cost_prefix(line)
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "destroy all artifacts" => Some("artifacts"),
        "destroy all creatures" => Some("creatures"),
        "destroy all enchantments" => Some("enchantments"),
        _ => None,
    }
}

fn strip_cost_prefix(line: &str) -> &str {
    let trimmed = line.trim();
    if let Some((prefix, tail)) = trimmed.split_once(':') {
        let prefix = prefix.trim();
        let looks_like_cost = prefix.starts_with('{')
            || prefix.eq_ignore_ascii_case("t")
            || matches!(
                prefix.to_ascii_lowercase().as_str(),
                "t" | "tap"
                    | "tap this source"
                    | "tap this creature"
                    | "tap this land"
                    | "tap this permanent"
            )
            || prefix.chars().all(|ch| {
                ch.is_ascii_whitespace()
                    || ch == ','
                    || ch == '{'
                    || ch == '}'
                    || ch == '('
                    || ch == ')'
                    || ch.is_ascii_digit()
                    || ch == '/'
                    || ch == '|'
                    || matches!(
                        ch,
                        'W' | 'U' | 'B' | 'R' | 'G' | 'w' | 'u' | 'b' | 'r' | 'g' | 'x' | 'X'
                    )
            });
        if looks_like_cost {
            return strip_cost_prefix(tail.trim());
        }
    }
    trimmed
}

fn is_damage_to_any_target_clause(line: &str) -> bool {
    let normalized = strip_cost_prefix(line).to_ascii_lowercase();
    let lower = normalized.as_str();
    (lower.starts_with("this creature deals") || lower.starts_with("deals"))
        && lower.contains("damage")
        && lower.contains(" to any target")
}

fn normalize_damage_to_self_clause(line: &str) -> Option<String> {
    let trimmed = strip_cost_prefix(line).trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("damage") {
        return None;
    }
    if !(lower.contains("to this creature") || lower.contains("to itself")) {
        return None;
    }
    if lower.starts_with("deal ") {
        let tail = trimmed
            .trim_start_matches("Deal")
            .trim()
            .replace("this creature ", "")
            .replace("to this creature", "to itself");
        Some(format!("deals {tail}"))
    } else if lower.starts_with("this creature deals ") {
        Some(
            trimmed
                .replace("this creature ", "")
                .replace("to this creature", "to itself"),
        )
    } else if lower.starts_with("deals ") {
        Some(trimmed.replace("to this creature", "to itself"))
    } else {
        None
    }
}

fn merge_damage_to_self_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    for line in lines {
        let parts = line
            .split('.')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() < 2 {
            merged.push(line.to_string());
            continue;
        }

        let mut idx = 0usize;
        let mut rebuilt = Vec::new();
        while idx < parts.len() {
            if idx + 1 < parts.len()
                && is_damage_to_any_target_clause(parts[idx])
                && let Some(trailing) = normalize_damage_to_self_clause(parts[idx + 1])
            {
                rebuilt.push(format!("{} and {}", parts[idx], trailing));
                idx += 2;
                continue;
            }
            rebuilt.push(parts[idx].to_string());
            idx += 1;
        }
        merged.push(rebuilt.join(". "));
    }
    merged
}

fn merge_destroy_all_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut split_lines = Vec::with_capacity(lines.len() * 2);
    for line in lines {
        let mut has_destroy_all = false;
        for part in line.split('.') {
            if is_destroy_all_clause(part).is_some() {
                has_destroy_all = true;
                break;
            }
        }
        if !has_destroy_all {
            split_lines.push(line.to_string());
            continue;
        }

        for part in line
            .split('.')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
        {
            split_lines.push(part.to_string());
        }
    }
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    const DESTROY_ALL_TYPES: [&str; 3] = ["artifacts", "creatures", "enchantments"];

    while idx < split_lines.len() {
        let Some(first) = is_destroy_all_clause(&split_lines[idx]) else {
            merged.push(split_lines[idx].clone());
            idx += 1;
            continue;
        };
        let mut types = vec![first];
        idx += 1;

        while idx < split_lines.len() {
            let Some(next) = is_destroy_all_clause(&split_lines[idx]) else {
                break;
            };
            types.push(next);
            idx += 1;
        }

        if types.len() < 2 {
            merged.push(format!("Destroy all {first}."));
            continue;
        }

        if types.len() == 2 && types[0] == types[1] {
            merged.push(format!("Destroy all {}.", types[0]));
            continue;
        }

        types.dedup();
        let mut ordered = Vec::new();
        for typ in DESTROY_ALL_TYPES {
            if types.contains(&typ) {
                ordered.push(typ);
            }
        }
        let joined = match ordered.as_slice() {
            [single] => format!("Destroy all {single}."),
            [first, second] => format!("Destroy all {first} and {second}."),
            [first, second, third] => format!("Destroy all {first}, {second}, and {third}."),
            _ => format!("Destroy all {}.", ordered.join(", ")),
        };
        merged.push(joined);
    }

    merged
}

fn merge_transform_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let left = lines[idx].trim().trim_end_matches('.');
        let Some(subject) = split_lose_all_abilities_subject(left) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut consumed = 1usize;
        let mut colors: Vec<String> = Vec::new();
        let mut card_types: Vec<String> = Vec::new();
        let mut subtypes: Vec<String> = Vec::new();
        let mut named: Option<String> = None;
        let mut base_pt: Option<String> = None;

        while idx + consumed < lines.len() {
            let line = lines[idx + consumed].trim().trim_end_matches('.');
            if let Some(pt) = extract_base_pt_tail_for_subject(line, subject) {
                base_pt = Some(pt);
                consumed += 1;
                continue;
            }

            let subject_prefix = format!("{subject} is ");
            let Some(rest) = line.strip_prefix(&subject_prefix) else {
                break;
            };
            let rest = rest.trim();
            if let Some(name) = rest.strip_prefix("named ") {
                named = Some(name.trim().to_string());
                consumed += 1;
                continue;
            }
            for part in rest
                .split(" and ")
                .map(str::trim)
                .filter(|part| !part.is_empty())
            {
                let lower = part.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "white" | "blue" | "black" | "red" | "green" | "colorless"
                ) {
                    if !colors.contains(&lower) {
                        colors.push(lower);
                    }
                    continue;
                }
                if matches!(
                    lower.as_str(),
                    "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
                ) {
                    if !card_types.contains(&lower) {
                        card_types.push(lower);
                    }
                    continue;
                }
                if !subtypes.contains(&lower) {
                    subtypes.push(lower);
                }
            }
            consumed += 1;
        }

        if consumed == 1 {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let mut combined = format!("{subject} loses all abilities");
        let mut descriptor = String::new();
        if !colors.is_empty() {
            descriptor.push_str(&colors.join(" and "));
        }
        if !subtypes.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&subtypes.join(" and "));
        }
        if !card_types.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&card_types.join(" and "));
        }
        if !descriptor.is_empty() {
            combined.push_str(" and is ");
            combined.push_str(&descriptor);
        }
        if let Some(pt) = base_pt {
            combined.push_str(" with base power and toughness ");
            combined.push_str(&pt);
        }
        if let Some(name) = named {
            combined.push_str(" named ");
            combined.push_str(&name);
        }
        merged.push(combined);
        idx += consumed;
    }

    merged
}

fn compare_semantics(
    card_name: &str,
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let normalized_oracle = normalize_card_self_references(oracle_text, card_name);
    let normalized_compiled_lines = compiled_lines
        .iter()
        .map(|line| normalize_card_self_references(line, card_name))
        .collect::<Vec<_>>();
    let stripped_compiled_lines = normalized_compiled_lines
        .iter()
        .map(|line| strip_compiled_prefix(line).to_string())
        .collect::<Vec<_>>();
    let merged_mana_lines = merge_simple_mana_add_compiled_lines(&stripped_compiled_lines);
    let merged_blockability_lines = merge_blockability_compiled_lines(&merged_mana_lines);
    let merged_damage_lines = merge_damage_to_self_compiled_lines(&merged_blockability_lines);
    let merged_destroy_all_lines = merge_destroy_all_compiled_lines(&merged_damage_lines);
    let normalized_compiled_lines = merge_transform_compiled_lines(&merged_destroy_all_lines);
    shared_compare_semantics_scored(
        &normalized_oracle,
        &normalized_compiled_lines,
        to_shared_embedding_config(embedding),
    )
}

fn normalize_parse_error(error: &str) -> String {
    let mut normalized = error
        .trim()
        .trim_start_matches("ParseError(\"")
        .trim_start_matches("UnsupportedLine(\"")
        .trim_end_matches("\")")
        .to_string();
    for marker in [" (clause:", " (line:", " (source:"] {
        if let Some(idx) = normalized.find(marker) {
            normalized = normalized[..idx].trim().to_string();
        }
    }
    normalized
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let mut out = String::new();
    for ch in text.chars().take(keep) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

fn first_oracle_excerpt(text: &str) -> String {
    semantic_clauses(text)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn first_compiled_excerpt(lines: &[String]) -> String {
    lines
        .first()
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

fn read_name_set(path: &str) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            if let Some(rest) = line.strip_prefix("Name:") {
                let name = rest.trim();
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            } else if line.contains(':') {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect())
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

    let oracle_text = parse_lines
        .iter()
        .position(|line| !is_stream_metadata_line(line.trim()))
        .map(|oracle_start| parse_lines[oracle_start..].join("\n").trim().to_string())
        .unwrap_or_default();

    Some(CardInput {
        name,
        oracle_text,
        parse_input,
    })
}

fn load_card_inputs_from_stream(
    cards_path: &str,
) -> Result<Vec<CardInput>, Box<dyn std::error::Error>> {
    let scripts_dir = tooling_paths::repo_root()?.join("scripts");
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

fn load_card_inputs_from_registry(
    db: &CardStatusDb,
) -> Result<Vec<CardInput>, Box<dyn std::error::Error>> {
    let payloads = db.registry_card_payloads()?;
    if payloads.is_empty() {
        return Err("no registry_card rows found in DB; run sync_registry_db first".into());
    }
    Ok(payloads
        .into_iter()
        .map(|payload| CardInput {
            name: payload.name,
            oracle_text: payload.oracle_text,
            parse_input: payload.parse_input,
        })
        .collect())
}

fn json_push_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if c <= '\u{1F}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn json_push_opt_string(out: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        json_push_string(out, value);
    } else {
        out.push_str("null");
    }
}

fn json_push_f32(out: &mut String, value: f32) {
    if value.is_finite() {
        out.push_str(&value.to_string());
    } else {
        out.push_str("0.0");
    }
}

fn json_encode_failure_report(report: &JsonFailureReport) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str("\"threshold\":");
    json_push_f32(&mut out, report.threshold);
    out.push(',');
    out.push_str("\"cards_processed\":");
    out.push_str(&report.cards_processed.to_string());
    out.push(',');
    out.push_str("\"failures\":");
    out.push_str(&report.failures.to_string());
    out.push(',');
    out.push_str("\"entries\":[");
    for (idx, entry) in report.entries.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"name\":");
        json_push_string(&mut out, &entry.name);
        out.push(',');
        out.push_str("\"parse_error\":");
        json_push_opt_string(&mut out, entry.parse_error.as_deref());
        out.push(',');
        out.push_str("\"oracle_coverage\":");
        json_push_f32(&mut out, entry.oracle_coverage);
        out.push(',');
        out.push_str("\"compiled_coverage\":");
        json_push_f32(&mut out, entry.compiled_coverage);
        out.push(',');
        out.push_str("\"similarity_score\":");
        json_push_f32(&mut out, entry.similarity_score);
        out.push(',');
        out.push_str("\"line_delta\":");
        out.push_str(&entry.line_delta.to_string());
        out.push(',');
        out.push_str("\"oracle_text\":");
        json_push_string(&mut out, &entry.oracle_text);
        out.push(',');
        out.push_str("\"compiled_text\":");
        json_push_string(&mut out, &entry.compiled_text);
        out.push(',');
        out.push_str("\"compiled_lines\":[");
        for (line_idx, line) in entry.compiled_lines.iter().enumerate() {
            if line_idx > 0 {
                out.push(',');
            }
            json_push_string(&mut out, line);
        }
        out.push(']');
        out.push('}');
    }
    out.push(']');
    out.push('}');
    out
}

fn ensure_parent_dir(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Some(parent) = Path::new(path).parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent)?;
    Ok(())
}

fn csv_push_field(out: &mut String, value: &str) {
    let needs_quotes = value.contains([',', '"', '\n', '\r']);
    if needs_quotes {
        out.push('"');
        for ch in value.chars() {
            if ch == '"' {
                out.push('"');
            }
            out.push(ch);
        }
        out.push('"');
    } else {
        out.push_str(value);
    }
}

fn csv_push_row<I>(out: &mut String, values: I)
where
    I: IntoIterator<Item = String>,
{
    for (idx, value) in values.into_iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        csv_push_field(out, &value);
    }
    out.push('\n');
}

fn compare_cluster_entries(a: &CardAudit, b: &CardAudit) -> Ordering {
    let a_key = (
        a.parse_error.is_none(),
        a.oracle_coverage.min(a.compiled_coverage),
        -(a.line_delta.abs()),
    );
    let b_key = (
        b.parse_error.is_none(),
        b.oracle_coverage.min(b.compiled_coverage),
        -(b.line_delta.abs()),
    );
    a_key
        .0
        .cmp(&b_key.0)
        .then_with(|| a_key.1.partial_cmp(&b_key.1).unwrap_or(Ordering::Equal))
        .then_with(|| a_key.2.cmp(&b_key.2))
        .then_with(|| a.name.cmp(&b.name))
}

fn cluster_error_counts(entries: &[CardAudit]) -> Vec<(String, usize)> {
    let mut error_counts: HashMap<String, usize> = HashMap::new();
    for entry in entries {
        if let Some(error) = entry.parse_error.as_ref() {
            *error_counts
                .entry(normalize_parse_error(error))
                .or_insert(0usize) += 1;
        }
    }
    let mut error_counts_vec: Vec<(String, usize)> = error_counts.into_iter().collect();
    error_counts_vec.sort_by(|(a_error, a_count), (b_error, b_count)| {
        b_count.cmp(a_count).then_with(|| a_error.cmp(b_error))
    });
    error_counts_vec
}

fn top_errors_summary(error_counts: &[(String, usize)], limit: usize) -> String {
    error_counts
        .iter()
        .take(limit)
        .map(|(error, count)| format!("{count}x {error}"))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn example_names_summary(entries: &[CardAudit], limit: usize) -> String {
    entries
        .iter()
        .take(limit)
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn example_oracle_summary(entries: &[CardAudit], limit: usize) -> String {
    entries
        .iter()
        .take(limit)
        .map(|entry| first_oracle_excerpt(&entry.oracle_text))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn parse_failure_cluster_refs<'a>(
    ranked: &'a [(String, Vec<CardAudit>)],
) -> Vec<ParseFailureClusterRef<'a>> {
    let mut clusters = ranked
        .iter()
        .filter_map(|(signature, entries)| {
            let size = entries.len();
            let parse_failures = entries
                .iter()
                .filter(|entry| entry.parse_error.is_some())
                .count();
            if parse_failures == 0 {
                return None;
            }
            let semantic_mismatches = entries
                .iter()
                .filter(|entry| entry.parse_error.is_none() && entry.semantic_mismatch)
                .count();
            let semantic_false_positives = entries
                .iter()
                .filter(|entry| entry.parse_error.is_none() && entry.semantic_false_positive)
                .count();
            Some(ParseFailureClusterRef {
                signature,
                entries,
                size,
                parse_failures,
                semantic_mismatches,
                semantic_false_positives,
                parse_failure_rate: parse_failures as f32 / size.max(1) as f32,
                semantic_mismatch_rate: semantic_mismatches as f32 / size.max(1) as f32,
            })
        })
        .collect::<Vec<_>>();

    clusters.sort_by(|a, b| {
        b.parse_failures
            .cmp(&a.parse_failures)
            .then_with(|| b.size.cmp(&a.size))
            .then_with(|| a.signature.cmp(b.signature))
    });
    clusters
}

fn write_cluster_csv(
    path: &str,
    ranked: &[(String, Vec<CardAudit>)],
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_parent_dir(path)?;
    let clusters = parse_failure_cluster_refs(ranked);

    let mut out = String::new();
    csv_push_row(
        &mut out,
        vec![
            "cluster_rank".to_string(),
            "cluster_signature".to_string(),
            "cluster_size".to_string(),
            "parse_failures".to_string(),
            "parse_failure_rate".to_string(),
            "semantic_mismatches".to_string(),
            "semantic_mismatch_rate".to_string(),
            "semantic_false_positives".to_string(),
            "top_parse_errors".to_string(),
            "example_names".to_string(),
            "example_oracle_excerpts".to_string(),
        ],
    );

    for (index, cluster) in clusters.iter().enumerate() {
        let mut sorted_entries = cluster.entries.to_vec();
        sorted_entries.sort_by(compare_cluster_entries);
        let error_counts_vec = cluster_error_counts(&sorted_entries);

        csv_push_row(
            &mut out,
            vec![
                (index + 1).to_string(),
                cluster.signature.to_string(),
                cluster.size.to_string(),
                cluster.parse_failures.to_string(),
                cluster.parse_failure_rate.to_string(),
                cluster.semantic_mismatches.to_string(),
                cluster.semantic_mismatch_rate.to_string(),
                cluster.semantic_false_positives.to_string(),
                top_errors_summary(&error_counts_vec, 5),
                example_names_summary(&sorted_entries, 5),
                example_oracle_summary(&sorted_entries, 3),
            ],
        );
    }

    fs::write(path, out)?;
    println!(
        "Wrote cluster CSV to {path} ({} parse-failure clusters)",
        clusters.len()
    );
    Ok(())
}

fn write_parse_errors_csv(
    path: &str,
    ranked: &[(String, Vec<CardAudit>)],
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_parent_dir(path)?;
    let clusters = parse_failure_cluster_refs(ranked);

    let mut out = String::new();
    csv_push_row(
        &mut out,
        vec![
            "cluster_rank".to_string(),
            "cluster_signature".to_string(),
            "cluster_size".to_string(),
            "cluster_parse_failures".to_string(),
            "cluster_parse_failure_rate".to_string(),
            "cluster_semantic_mismatches".to_string(),
            "cluster_semantic_mismatch_rate".to_string(),
            "cluster_error_count".to_string(),
            "card_name".to_string(),
            "normalized_parse_error".to_string(),
            "raw_parse_error".to_string(),
            "oracle_excerpt".to_string(),
            "oracle_text".to_string(),
        ],
    );

    for (index, cluster) in clusters.iter().enumerate() {
        let mut sorted_entries = cluster.entries.to_vec();
        sorted_entries.sort_by(compare_cluster_entries);

        let mut per_error_counts: HashMap<String, usize> = HashMap::new();
        for entry in &sorted_entries {
            if let Some(error) = entry.parse_error.as_ref() {
                *per_error_counts
                    .entry(normalize_parse_error(error))
                    .or_insert(0usize) += 1;
            }
        }

        for entry in sorted_entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
        {
            let raw_error = entry.parse_error.clone().unwrap_or_default();
            let normalized_error = normalize_parse_error(&raw_error);
            let cluster_error_count = per_error_counts
                .get(&normalized_error)
                .copied()
                .unwrap_or(0usize);

            csv_push_row(
                &mut out,
                vec![
                    (index + 1).to_string(),
                    cluster.signature.to_string(),
                    cluster.size.to_string(),
                    cluster.parse_failures.to_string(),
                    cluster.parse_failure_rate.to_string(),
                    cluster.semantic_mismatches.to_string(),
                    cluster.semantic_mismatch_rate.to_string(),
                    cluster_error_count.to_string(),
                    entry.name.clone(),
                    normalized_error,
                    raw_error,
                    first_oracle_excerpt(&entry.oracle_text),
                    entry.oracle_text.clone(),
                ],
            );
        }
    }

    fs::write(path, out)?;
    println!("Wrote parse-errors CSV to {path}");
    Ok(())
}

fn write_parse_error_summary_csv(
    path: &str,
    ranked: &[(String, Vec<CardAudit>)],
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Default)]
    struct ErrorSummary {
        count: usize,
        unique_card_names: HashSet<String>,
        unique_cluster_signatures: HashSet<String>,
        example_card_names: Vec<String>,
        example_cluster_signatures: Vec<String>,
        example_raw_errors: Vec<String>,
    }

    fn push_unique_sample(samples: &mut Vec<String>, value: String, limit: usize) {
        if samples.len() >= limit || samples.iter().any(|existing| existing == &value) {
            return;
        }
        samples.push(value);
    }

    ensure_parent_dir(path)?;

    let mut summaries: HashMap<String, ErrorSummary> = HashMap::new();
    for cluster in parse_failure_cluster_refs(ranked) {
        for entry in cluster
            .entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
        {
            let raw_error = entry.parse_error.clone().unwrap_or_default();
            let normalized_error = normalize_parse_error(&raw_error);
            let summary = summaries.entry(normalized_error).or_default();
            summary.count += 1;
            summary.unique_card_names.insert(entry.name.clone());
            summary
                .unique_cluster_signatures
                .insert(cluster.signature.to_string());
            push_unique_sample(&mut summary.example_card_names, entry.name.clone(), 5);
            push_unique_sample(
                &mut summary.example_cluster_signatures,
                cluster.signature.to_string(),
                3,
            );
            push_unique_sample(&mut summary.example_raw_errors, raw_error, 3);
        }
    }

    let mut rows = summaries.into_iter().collect::<Vec<_>>();
    rows.sort_by(|(a_error, a_summary), (b_error, b_summary)| {
        b_summary
            .count
            .cmp(&a_summary.count)
            .then_with(|| {
                b_summary
                    .unique_card_names
                    .len()
                    .cmp(&a_summary.unique_card_names.len())
            })
            .then_with(|| a_error.cmp(b_error))
    });

    let mut out = String::new();
    csv_push_row(
        &mut out,
        vec![
            "normalized_parse_error".to_string(),
            "occurrences".to_string(),
            "unique_card_names".to_string(),
            "unique_cluster_signatures".to_string(),
            "example_card_names".to_string(),
            "example_cluster_signatures".to_string(),
            "example_raw_errors".to_string(),
        ],
    );

    for (error, summary) in rows {
        csv_push_row(
            &mut out,
            vec![
                error,
                summary.count.to_string(),
                summary.unique_card_names.len().to_string(),
                summary.unique_cluster_signatures.len().to_string(),
                summary.example_card_names.join(" | "),
                summary.example_cluster_signatures.join(" | "),
                summary.example_raw_errors.join(" | "),
            ],
        );
    }

    fs::write(path, out)?;
    println!("Wrote parse-error summary CSV to {path}");
    Ok(())
}

fn json_encode_audits_report(report: &JsonAuditsReport) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str("\"threshold\":");
    json_push_f32(&mut out, report.threshold);
    out.push(',');
    out.push_str("\"embedding_dims\":");
    out.push_str(&report.embedding_dims.to_string());
    out.push(',');
    out.push_str("\"cards_processed\":");
    out.push_str(&report.cards_processed.to_string());
    out.push(',');
    out.push_str("\"parse_failures\":");
    out.push_str(&report.parse_failures.to_string());
    out.push(',');
    out.push_str("\"semantic_mismatches\":");
    out.push_str(&report.semantic_mismatches.to_string());
    out.push(',');
    out.push_str("\"semantic_false_positives\":");
    out.push_str(&report.semantic_false_positives.to_string());
    out.push(',');
    out.push_str("\"parse_success_with_unimplemented\":");
    out.push_str(&report.parse_success_with_unimplemented.to_string());
    out.push(',');
    out.push_str("\"entries\":[");
    for (idx, entry) in report.entries.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"name\":");
        json_push_string(&mut out, &entry.name);
        out.push(',');
        out.push_str("\"parse_error\":");
        json_push_opt_string(&mut out, entry.parse_error.as_deref());
        out.push(',');
        out.push_str("\"semantic_mismatch\":");
        out.push_str(if entry.semantic_mismatch {
            "true"
        } else {
            "false"
        });
        out.push(',');
        out.push_str("\"semantic_false_positive\":");
        out.push_str(if entry.semantic_false_positive {
            "true"
        } else {
            "false"
        });
        out.push(',');
        out.push_str("\"has_unimplemented\":");
        out.push_str(if entry.has_unimplemented {
            "true"
        } else {
            "false"
        });
        out.push(',');
        out.push_str("\"oracle_coverage\":");
        json_push_f32(&mut out, entry.oracle_coverage);
        out.push(',');
        out.push_str("\"compiled_coverage\":");
        json_push_f32(&mut out, entry.compiled_coverage);
        out.push(',');
        out.push_str("\"similarity_score\":");
        json_push_f32(&mut out, entry.similarity_score);
        out.push(',');
        out.push_str("\"line_delta\":");
        out.push_str(&entry.line_delta.to_string());
        out.push('}');
    }
    out.push(']');
    out.push('}');
    out
}

fn json_encode_cluster_report(report: &JsonReport) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str("\"cards_processed\":");
    out.push_str(&report.cards_processed.to_string());
    out.push(',');
    out.push_str("\"parse_failures\":");
    out.push_str(&report.parse_failures.to_string());
    out.push(',');
    out.push_str("\"semantic_mismatches\":");
    out.push_str(&report.semantic_mismatches.to_string());
    out.push(',');
    out.push_str("\"semantic_false_positives\":");
    out.push_str(&report.semantic_false_positives.to_string());
    out.push(',');
    out.push_str("\"clusters_total\":");
    out.push_str(&report.clusters_total.to_string());
    out.push(',');
    out.push_str("\"clusters_reported\":");
    out.push_str(&report.clusters_reported.to_string());
    out.push(',');
    out.push_str("\"clusters\":[");
    for (idx, cluster) in report.clusters.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"signature\":");
        json_push_string(&mut out, &cluster.signature);
        out.push(',');
        out.push_str("\"size\":");
        out.push_str(&cluster.size.to_string());
        out.push(',');
        out.push_str("\"parse_failures\":");
        out.push_str(&cluster.parse_failures.to_string());
        out.push(',');
        out.push_str("\"semantic_mismatches\":");
        out.push_str(&cluster.semantic_mismatches.to_string());
        out.push(',');
        out.push_str("\"semantic_false_positives\":");
        out.push_str(&cluster.semantic_false_positives.to_string());
        out.push(',');
        out.push_str("\"parse_failure_rate\":");
        json_push_f32(&mut out, cluster.parse_failure_rate);
        out.push(',');
        out.push_str("\"semantic_mismatch_rate\":");
        json_push_f32(&mut out, cluster.semantic_mismatch_rate);
        out.push(',');
        out.push_str("\"top_errors\":[");
        for (error_idx, error) in cluster.top_errors.iter().enumerate() {
            if error_idx > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"error\":");
            json_push_string(&mut out, &error.error);
            out.push(',');
            out.push_str("\"count\":");
            out.push_str(&error.count.to_string());
            out.push('}');
        }
        out.push(']');
        out.push(',');
        out.push_str("\"examples\":[");
        for (example_idx, example) in cluster.examples.iter().enumerate() {
            if example_idx > 0 {
                out.push(',');
            }
            out.push('{');
            out.push_str("\"name\":");
            json_push_string(&mut out, &example.name);
            out.push(',');
            out.push_str("\"parse_error\":");
            json_push_opt_string(&mut out, example.parse_error.as_deref());
            out.push(',');
            out.push_str("\"oracle_coverage\":");
            json_push_f32(&mut out, example.oracle_coverage);
            out.push(',');
            out.push_str("\"compiled_coverage\":");
            json_push_f32(&mut out, example.compiled_coverage);
            out.push(',');
            out.push_str("\"similarity_score\":");
            json_push_f32(&mut out, example.similarity_score);
            out.push(',');
            out.push_str("\"line_delta\":");
            out.push_str(&example.line_delta.to_string());
            out.push(',');
            out.push_str("\"oracle_excerpt\":");
            json_push_string(&mut out, &example.oracle_excerpt);
            out.push(',');
            out.push_str("\"compiled_excerpt\":");
            json_push_string(&mut out, &example.compiled_excerpt);
            out.push(',');
            out.push_str("\"oracle_text\":");
            json_push_string(&mut out, &example.oracle_text);
            out.push(',');
            out.push_str("\"compiled_lines\":[");
            for (line_idx, line) in example.compiled_lines.iter().enumerate() {
                if line_idx > 0 {
                    out.push(',');
                }
                json_push_string(&mut out, line);
            }
            out.push(']');
            out.push('}');
        }
        out.push(']');
        out.push('}');
    }
    out.push(']');
    out.push('}');
    out
}

fn set_parser_trace(enabled: bool) {
    unsafe {
        if enabled {
            env::set_var("IRONSMITH_PARSER_TRACE", "1");
        } else {
            env::remove_var("IRONSMITH_PARSER_TRACE");
        }
    }
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let db = if args.no_db {
        None
    } else {
        Some(CardStatusDb::open(&args.db_path)?)
    };
    let cards = if let Some(cards_path) = args.cards_path.as_deref() {
        load_card_inputs_from_stream(cards_path)?
    } else if let Some(db) = db.as_ref() {
        load_card_inputs_from_registry(db)?
    } else {
        return Err("no card input source available: pass --cards or omit --no-db".into());
    };

    let original_trace = env::var("IRONSMITH_PARSER_TRACE").ok();
    let original_allow_unsupported = env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED").ok();
    let original_semantic_guard = env::var("IRONSMITH_PARSER_SEMANTIC_GUARD").ok();
    let original_semantic_dims = env::var("IRONSMITH_PARSER_SEMANTIC_DIMS").ok();
    let original_semantic_threshold = env::var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD").ok();
    if args.parser_trace {
        set_parser_trace(true);
    }
    if args.allow_unsupported {
        set_allow_unsupported(true);
    } else {
        set_allow_unsupported(false);
    }

    let embedding_cfg = if args.use_embeddings {
        Some(EmbeddingConfig {
            dims: args.embedding_dims,
            mismatch_threshold: args.embedding_threshold,
        })
    } else {
        None
    };
    let false_positive_names = match args.false_positive_names.as_deref() {
        Some(path) => read_name_set(path)?,
        None => HashSet::new(),
    };
    let mut audits = Vec::new();
    for card_input in cards {
        if let Some(limit) = args.limit
            && audits.len() >= limit
        {
            break;
        }

        let trace_for_card = if args.parser_trace {
            true
        } else if let Some(filter) = args.trace_name.as_ref() {
            card_input.name.to_ascii_lowercase().contains(filter)
        } else {
            false
        };
        if !args.parser_trace {
            set_parser_trace(trace_for_card);
        }

        let cluster_key = cluster_key(&card_input.oracle_text);
        let parse_result = CardDefinitionBuilder::new(CardId::new(), &card_input.name)
            .parse_text(card_input.parse_input.clone());

        let (audit, snapshot) = match parse_result {
            Ok(definition) => {
                let has_unimplemented = generated_definition_has_unimplemented_content(&definition);
                let compiled = compiled_lines(&definition);
                let (
                    oracle_coverage,
                    compiled_coverage,
                    similarity_score,
                    line_delta,
                    semantic_mismatch,
                ) = compare_semantics(
                    &card_input.name,
                    &card_input.oracle_text,
                    &compiled,
                    embedding_cfg,
                );
                (
                    CardAudit {
                        name: card_input.name.clone(),
                        oracle_text: card_input.oracle_text.clone(),
                        cluster_key,
                        parse_error: None,
                        compiled_lines: compiled,
                        oracle_coverage,
                        compiled_coverage,
                        similarity_score,
                        line_delta,
                        semantic_mismatch,
                        semantic_false_positive: false,
                        has_unimplemented,
                    },
                    CompilationSnapshot::from_definition_result(
                        &card_input.name,
                        &card_input.oracle_text,
                        if args.allow_unsupported {
                            ParseStatus::CompiledWithAllowUnsupported
                        } else {
                            ParseStatus::StrictCompiled
                        },
                        None,
                        Some(&definition),
                    ),
                )
            }
            Err(err) => {
                let parse_error = format!("{err:?}");
                (
                    CardAudit {
                        name: card_input.name.clone(),
                        oracle_text: card_input.oracle_text.clone(),
                        cluster_key,
                        parse_error: Some(parse_error.clone()),
                        compiled_lines: Vec::new(),
                        oracle_coverage: 0.0,
                        compiled_coverage: 0.0,
                        similarity_score: 0.0,
                        line_delta: 0,
                        semantic_mismatch: false,
                        semantic_false_positive: false,
                        has_unimplemented: false,
                    },
                    CompilationSnapshot::from_definition_result(
                        &card_input.name,
                        &card_input.oracle_text,
                        ParseStatus::ParseFailed,
                        Some(parse_error),
                        None,
                    ),
                )
            }
        };
        if let Some(db) = db.as_ref() {
            db.insert_snapshot_if_changed(&snapshot)?;
        }
        audits.push(audit);
    }

    for audit in &mut audits {
        if audit.parse_error.is_none()
            && audit.semantic_mismatch
            && false_positive_names.contains(&audit.name)
        {
            audit.semantic_mismatch = false;
            audit.semantic_false_positive = true;
        }
    }

    match original_trace {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_TRACE", value);
        },
        None => set_parser_trace(false),
    }
    match original_allow_unsupported {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", value);
        },
        None => set_allow_unsupported(false),
    }
    match original_semantic_guard {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_SEMANTIC_GUARD", value);
        },
        None => unsafe {
            env::remove_var("IRONSMITH_PARSER_SEMANTIC_GUARD");
        },
    }
    match original_semantic_dims {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_SEMANTIC_DIMS", value);
        },
        None => unsafe {
            env::remove_var("IRONSMITH_PARSER_SEMANTIC_DIMS");
        },
    }
    match original_semantic_threshold {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD", value);
        },
        None => unsafe {
            env::remove_var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD");
        },
    }

    let mut clusters: HashMap<String, Vec<CardAudit>> = HashMap::new();
    for audit in audits {
        clusters
            .entry(audit.cluster_key.clone())
            .or_default()
            .push(audit);
    }

    let cards_processed = clusters.values().map(Vec::len).sum::<usize>();
    let parse_failures = clusters
        .values()
        .flatten()
        .filter(|audit| audit.parse_error.is_some())
        .count();
    let semantic_mismatches = clusters
        .values()
        .flatten()
        .filter(|audit| audit.parse_error.is_none() && audit.semantic_mismatch)
        .count();
    let semantic_false_positives = clusters
        .values()
        .flatten()
        .filter(|audit| audit.parse_error.is_none() && audit.semantic_false_positive)
        .count();
    let parse_success_with_unimplemented = clusters
        .values()
        .flatten()
        .filter(|audit| audit.parse_error.is_none() && audit.has_unimplemented)
        .count();

    if let Some(path) = args.mismatch_names_out.as_ref() {
        let mut names = clusters
            .values()
            .flatten()
            .filter(|audit| audit.parse_error.is_none() && audit.semantic_mismatch)
            .map(|audit| audit.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        fs::write(path, names.join("\n"))?;
        println!("Wrote mismatch names to {path} ({} names)", names.len());
    }

    if let Some(path) = args.failures_out.as_ref() {
        let mut entries = clusters
            .values()
            .flatten()
            .filter(|audit| audit.parse_error.is_none() && audit.semantic_mismatch)
            .map(|audit| JsonFailureEntry {
                name: audit.name.clone(),
                parse_error: audit.parse_error.clone(),
                oracle_coverage: audit.oracle_coverage,
                compiled_coverage: audit.compiled_coverage,
                similarity_score: audit.similarity_score,
                line_delta: audit.line_delta,
                oracle_text: audit.oracle_text.clone(),
                compiled_text: audit.compiled_lines.join("\n"),
                compiled_lines: audit.compiled_lines.clone(),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| {
            a.similarity_score
                .partial_cmp(&b.similarity_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });
        let report = JsonFailureReport {
            threshold: args.embedding_threshold,
            cards_processed,
            failures: entries.len(),
            entries,
        };
        let payload = json_encode_failure_report(&report);
        fs::write(path, payload)?;
        println!(
            "Wrote threshold failure report to {path} ({} cards)",
            report.failures
        );
    }

    if let Some(path) = args.audits_out.as_ref() {
        let mut entries = clusters
            .values()
            .flatten()
            .map(|audit| JsonAuditEntry {
                name: audit.name.clone(),
                parse_error: audit.parse_error.clone(),
                semantic_mismatch: audit.semantic_mismatch,
                semantic_false_positive: audit.semantic_false_positive,
                has_unimplemented: audit.has_unimplemented,
                oracle_coverage: audit.oracle_coverage,
                compiled_coverage: audit.compiled_coverage,
                similarity_score: audit.similarity_score,
                line_delta: audit.line_delta,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let report = JsonAuditsReport {
            threshold: args.embedding_threshold,
            embedding_dims: args.embedding_dims,
            cards_processed,
            parse_failures,
            semantic_mismatches,
            semantic_false_positives,
            parse_success_with_unimplemented,
            entries,
        };
        let payload = json_encode_audits_report(&report);
        fs::write(path, payload)?;
        println!(
            "Wrote per-card audit report to {path} ({} cards)",
            report.cards_processed
        );
    }

    let mut ranked: Vec<(String, Vec<CardAudit>)> = clusters
        .into_iter()
        .filter(|(_, entries)| entries.len() >= args.min_cluster_size)
        .collect();

    ranked.sort_by(|(a_key, a_entries), (b_key, b_entries)| {
        let a_fail = a_entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
            .count();
        let b_fail = b_entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
            .count();
        let a_mismatch = a_entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_mismatch)
            .count();
        let b_mismatch = b_entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_mismatch)
            .count();
        let a_total = a_entries.len().max(1) as f32;
        let b_total = b_entries.len().max(1) as f32;
        let a_problem = (a_fail + a_mismatch) as f32 / a_total;
        let b_problem = (b_fail + b_mismatch) as f32 / b_total;
        b_problem
            .partial_cmp(&a_problem)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b_entries.len().cmp(&a_entries.len()))
            .then_with(|| a_key.cmp(b_key))
    });

    println!("Oracle cluster audit complete");
    println!("- Cards processed: {cards_processed}");
    println!("- Parse failures: {parse_failures}");
    println!("- Semantic mismatches: {semantic_mismatches}");
    println!(
        "- Parse-success cards with unimplemented content: {parse_success_with_unimplemented}"
    );
    if !false_positive_names.is_empty() {
        println!("- Marked semantic false positives: {semantic_false_positives}");
    }
    if let Some(cfg) = embedding_cfg {
        println!(
            "- Embedding audit: enabled (dims={}, threshold={:.2})",
            cfg.dims, cfg.mismatch_threshold
        );
    } else {
        println!("- Embedding audit: disabled");
    }
    if !args.no_db {
        println!("- DB: {}", args.db_path);
    }
    println!("- Total clusters: {}", ranked.len());
    println!("- Reporting up to {} clusters", args.top_clusters);
    println!();

    let mut parse_failure_hotspots = ranked
        .iter()
        .map(|(signature, entries)| {
            let parse_failures_count = entries
                .iter()
                .filter(|entry| entry.parse_error.is_some())
                .count();
            (signature, entries.len(), parse_failures_count)
        })
        .filter(|(_, _, parse_failures_count)| *parse_failures_count > 0)
        .collect::<Vec<_>>();
    parse_failure_hotspots.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.1.cmp(&a.1)));
    if !parse_failure_hotspots.is_empty() {
        println!("Top parse-failure clusters (absolute count):");
        for (signature, size, parse_failures_count) in parse_failure_hotspots.iter().take(8) {
            println!(
                "  - {} failures in cluster size {} :: {}",
                parse_failures_count,
                size,
                truncate_text(signature, 110)
            );
        }
        println!();
    }

    if let Some(path) = args.cluster_csv_out.as_ref() {
        write_cluster_csv(path, &ranked)?;
    }

    if let Some(path) = args.parse_errors_csv_out.as_ref() {
        write_parse_errors_csv(path, &ranked)?;
    }

    if let Some(path) = args.parse_error_summary_csv_out.as_ref() {
        write_parse_error_summary_csv(path, &ranked)?;
    }

    let clusters_total = ranked.len();
    let mut json_clusters = Vec::new();

    for (index, (signature, mut entries)) in ranked.into_iter().take(args.top_clusters).enumerate()
    {
        entries.sort_by(compare_cluster_entries);

        let size = entries.len();
        let parse_failures_count = entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
            .count();
        let semantic_mismatch_count = entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_mismatch)
            .count();
        let semantic_false_positive_count = entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_false_positive)
            .count();
        let parse_failure_rate = parse_failures_count as f32 / size.max(1) as f32;
        let semantic_mismatch_rate = semantic_mismatch_count as f32 / size.max(1) as f32;

        let error_counts_vec = cluster_error_counts(&entries);

        println!(
            "[{}] size={} parse_failures={} ({:.1}%) semantic_mismatches={} ({:.1}%)",
            index + 1,
            size,
            parse_failures_count,
            parse_failure_rate * 100.0,
            semantic_mismatch_count,
            semantic_mismatch_rate * 100.0
        );
        if semantic_false_positive_count > 0 {
            println!("marked false positives: {semantic_false_positive_count}");
        }
        println!("signature: {signature}");

        if !error_counts_vec.is_empty() {
            println!("top parse errors:");
            for (error, count) in error_counts_vec.iter().take(3) {
                println!("  - {count}x {error}");
            }
        }

        println!("examples:");
        for entry in entries.iter().take(args.examples_per_cluster) {
            if let Some(error) = &entry.parse_error {
                println!(
                    "  - {} [parse-failed] {}",
                    entry.name,
                    normalize_parse_error(error)
                );
            } else {
                println!(
                    "  - {} [score={:.2}, coverage o->{:.2}, c->{:.2}, delta={}]",
                    entry.name,
                    entry.similarity_score,
                    entry.oracle_coverage,
                    entry.compiled_coverage,
                    entry.line_delta
                );
            }
            println!("    oracle: {}", first_oracle_excerpt(&entry.oracle_text));
            println!(
                "    compiled: {}",
                first_compiled_excerpt(&entry.compiled_lines)
            );
        }
        println!();

        let top_errors = error_counts_vec
            .into_iter()
            .take(5)
            .map(|(error, count)| JsonErrorCount { error, count })
            .collect::<Vec<_>>();
        let examples = entries
            .iter()
            .take(args.examples_per_cluster)
            .map(|entry| JsonExample {
                name: entry.name.clone(),
                parse_error: entry.parse_error.clone(),
                oracle_coverage: entry.oracle_coverage,
                compiled_coverage: entry.compiled_coverage,
                similarity_score: entry.similarity_score,
                line_delta: entry.line_delta,
                oracle_excerpt: first_oracle_excerpt(&entry.oracle_text),
                compiled_excerpt: first_compiled_excerpt(&entry.compiled_lines),
                oracle_text: entry.oracle_text.clone(),
                compiled_lines: entry.compiled_lines.clone(),
            })
            .collect::<Vec<_>>();

        json_clusters.push(JsonCluster {
            signature,
            size,
            parse_failures: parse_failures_count,
            semantic_mismatches: semantic_mismatch_count,
            semantic_false_positives: semantic_false_positive_count,
            parse_failure_rate,
            semantic_mismatch_rate,
            top_errors,
            examples,
        });
    }

    if let Some(path) = args.json_out {
        let report = JsonReport {
            cards_processed,
            parse_failures,
            semantic_mismatches,
            semantic_false_positives,
            clusters_total,
            clusters_reported: json_clusters.len(),
            clusters: json_clusters,
        };
        let payload = json_encode_cluster_report(&report);
        fs::write(&path, payload)?;
        println!("Wrote JSON report to {path}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_signature_generalizes_numbers_and_mana() {
        let first = "Target creature gets +3/+3 until end of turn.";
        let second = "Target creature gets +5/+5 until end of turn.";
        assert_eq!(clause_signature(first), clause_signature(second));
    }

    #[test]
    fn test_compare_semantics_detects_drop() {
        let oracle = "Draw a card. Target player discards a card.";
        let compiled = vec!["you gain 2 life".to_string()];
        let (oracle_coverage, compiled_coverage, _similarity_score, line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(mismatch);
        assert!(oracle_coverage < 0.75);
        assert!(compiled_coverage <= 0.25);
        assert!(line_delta < 0);
    }

    #[test]
    fn test_compare_semantics_ignores_parenthetical_only_oracle() {
        let oracle = "({T}: Add {B} or {R}.)";
        let compiled = vec!["Mana ability 1: Tap this source, Add {B} or {R}".to_string()];
        let (_oracle_coverage, _compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(
            !mismatch,
            "parenthetical-only oracle reminder text should not count as semantic mismatch"
        );
    }

    #[test]
    fn test_compare_semantics_keeps_boast_costed_prefix() {
        let oracle = "Boast — {1}{R}: This creature deals 1 damage to any target. (Activate only if this creature attacked this turn and only once each turn.)";
        let compiled = vec![
            "Activated ability 1: Boast {1}{R}: This creature deals 1 damage to any target."
                .to_string(),
        ];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
    }

    #[test]
    fn test_compare_semantics_ignores_only_once_reminder_only() {
        let oracle = "Sacrifice a Food: Draw a card. (Activate only once each turn.)";
        let compiled = vec!["Activated ability 1: Sacrifice a Food: Draw a card.".to_string()];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
    }

    #[test]
    fn test_strip_ability_word_prefix_keeps_boast() {
        let clauses =
            semantic_clauses("Boast — {1}{R}: This creature deals 1 damage to any target.");
        assert_eq!(
            clauses,
            vec!["Boast {1}{R}: This creature deals 1 damage to any target".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_ignore_parenthetical_lines_with_colon() {
        let oracle = "Create a Treasure token.\n(It's an artifact with \"{T}, Sacrifice this token: Add one mana of any color.\")";
        let clauses = semantic_clauses(oracle);
        assert_eq!(clauses, vec!["Create a Treasure token".to_string()]);
    }

    #[test]
    fn test_compare_semantics_excludes_compiled_clause_matched_only_by_reminder() {
        let oracle = "({T}: Add {R} or {W}.) This land enters tapped.";
        let compiled = vec![
            "Mana ability 1: {T}: Add {R}.".to_string(),
            "Mana ability 2: {T}: Add {W}.".to_string(),
            "Static ability 3: This land enters tapped.".to_string(),
        ];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
    }

    #[test]
    fn test_merge_simple_mana_add_compiled_lines_merges_three_modes() {
        let merged = merge_simple_mana_add_compiled_lines(&[
            "{T}: Add {W}.".to_string(),
            "{T}: Add {B}.".to_string(),
            "{T}: Add {G}.".to_string(),
        ]);
        assert_eq!(merged, vec!["{T}: Add {W} or {B} or {G}".to_string()]);
    }

    #[test]
    fn test_compare_semantics_normalizes_triome_three_color_mana_list() {
        let oracle = "({T}: Add {W}, {B}, or {G}.)\nThis land enters tapped.\nCycling {3} ({3}, Discard this card: Draw a card.)";
        let compiled = vec![
            "Mana ability 1: {T}: Add {W}.".to_string(),
            "Mana ability 2: {T}: Add {B}.".to_string(),
            "Mana ability 3: {T}: Add {G}.".to_string(),
            "Static ability 4: This land enters tapped.".to_string(),
            "Keyword ability 5: Cycling {3}".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity, _line_delta, mismatch) =
            compare_semantics(
                "",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            similarity >= 0.99,
            "expected triome mana list normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for triome mana list");
    }

    #[test]
    fn test_compare_semantics_normalizes_additional_mana_tap_trigger() {
        let oracle = "Whenever you tap a creature for mana, add an additional {G}.";
        let compiled =
            vec!["Triggered ability 2: Whenever you tap a creature for mana: Add {G}.".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity, _line_delta, mismatch) =
            compare_semantics(
                "",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            similarity >= 0.99,
            "expected additional-mana trigger normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for additional-mana trigger wording"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_sacrifice_cost_choice_scaffolding() {
        let oracle = "{1}{R}, Sacrifice a Goblin: This creature deals 4 damage to target creature.";
        let compiled = vec![
            "Activated ability 1: {1}{R}, Sacrifice a Goblin you control: This creature deals 4 damage to target creature."
                .to_string(),
        ];
        let (oracle_coverage, compiled_coverage, similarity, _line_delta, mismatch) =
            compare_semantics(
                "Arms Dealer",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
        assert!(
            similarity >= 0.99,
            "expected high similarity, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for normalized sacrifice cost"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_multi_sacrifice_cost_choice_scaffolding() {
        let oracle = "{2}{R}, Sacrifice two Goblins: Create three 1/1 red Goblin creature tokens.";
        let compiled = vec![
            "Activated ability 1: {2}{R}, Sacrifice two Goblins you control: Create three 1/1 red Goblin creature tokens."
                .to_string(),
        ];
        let (oracle_coverage, compiled_coverage, similarity, _line_delta, mismatch) =
            compare_semantics(
                "Goblin Warrens",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
        assert!(
            similarity >= 0.99,
            "expected high similarity, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for normalized multi-sacrifice cost"
        );
    }

    #[test]
    fn test_split_common_semantic_conjunctions_strips_multiple_ability_labels() {
        let clauses = semantic_clauses(
            "Static ability 4: Each player draws a card.\nTriggered ability 7: This creature deals 2 damage.\nKeyword ability 1: Flying",
        );
        assert_eq!(
            clauses,
            vec![
                "Each player draws a card".to_string(),
                "This creature deals 2 damage".to_string(),
                "Flying".to_string()
            ]
        );
    }

    #[test]
    fn test_embedding_mode_catches_dropped_where_plus_semantics() {
        let oracle = "Hobbit's Sting deals X damage to target creature, where X is the number of creatures you control plus the number of Foods you control.";
        let compiled = vec!["Deal X damage to target creature Food".to_string()];
        let (_oracle_coverage, _compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.90,
                }),
            );
        assert!(
            mismatch,
            "embedding mode should flag lost where/plus value semantics"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_named_trigger_subject() {
        let clauses = semantic_clauses("Whenever Tui and La become tapped, draw a card.");
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("whenever this creature"));
        assert!(clause.contains("tapp"));
        assert!(clause.contains("draw a card"));
    }

    #[test]
    fn test_semantic_clauses_split_and_you_gain_chain() {
        let clauses = semantic_clauses("Destroy target creature and you lose 2 life.");
        assert_eq!(
            clauses,
            vec![
                "Destroy target creature".to_string(),
                "You lose 2 life".to_string()
            ]
        );
    }

    #[test]
    fn test_semantic_clauses_split_you_draw_and_lose_chain() {
        let clauses =
            semantic_clauses("Whenever this creature attacks, you draw a card and lose 1 life.");
        assert_eq!(
            clauses,
            vec![
                "Whenever this creature attacks, draw a card".to_string(),
                "You lose 1 life".to_string()
            ]
        );
    }

    #[test]
    fn test_semantic_clauses_split_then_chain() {
        let clauses = semantic_clauses("Scry 1, then draw a card.");
        assert_eq!(
            clauses,
            vec!["Scry 1".to_string(), "draw a card".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_split_and_create_chain() {
        let clauses = semantic_clauses(
            "At the beginning of your upkeep, you lose 1 life and create a 1/1 black Faerie Rogue creature token with flying.",
        );
        assert_eq!(clauses.len(), 2);
        assert!(clauses[0].to_ascii_lowercase().contains("you lose 1 life"));
        assert!(clauses[1].to_ascii_lowercase().starts_with("create "));
    }

    #[test]
    fn test_semantic_clauses_normalize_explicit_damage_source_prefix() {
        let clauses = semantic_clauses(
            "When this creature leaves the battlefield, this creature deals 1 damage to you.",
        );
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("when this creature leaves the battlefield"));
        assert!(
            clause.contains("deal 1 damage to you") || clause.contains("deals 1 damage to you")
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_player_conditional_phrase() {
        let clauses = semantic_clauses(
            "For each player, if that player controls a multicolored creature, that player draws a card.",
        );
        assert_eq!(
            clauses,
            vec!["Each player who control a multicolored creature draws a card".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_player_that_player_phrase() {
        let clauses = semantic_clauses("For each player, that player draws a card.");
        assert_eq!(clauses, vec!["Each player draws a card".to_string()]);
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_player_prefix() {
        let clauses = semantic_clauses("For each player, Exile card in that player's library.");
        assert_eq!(
            clauses,
            vec!["Each player exile card in that player's library".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_opponent_prefix() {
        let clauses = semantic_clauses("for each opponent, deals 1 damage to that player.");
        assert_eq!(
            clauses,
            vec!["each opponent deals 1 damage to that player".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_opponent_that_player_phrase() {
        let clauses = semantic_clauses("For each opponent, that player draws a card.");
        assert_eq!(clauses, vec!["Each opponent draws a card".to_string()]);
    }

    #[test]
    fn test_semantic_clauses_normalize_unsupported_parser_line_fallback() {
        let clauses = semantic_clauses(
            "Unsupported parser line fallback: • Fire — {0} — Fire Magic deals 1 damage to each creature. (ParseError(\"unsupported target phrase).",
        );
        assert_eq!(
            clauses,
            vec!["Fire magic deals 1 damage to each creature".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_unsupported_parser_tiered_mode_line() {
        let clauses = semantic_clauses(
            "Unsupported parser line fallback: Tiered (Choose one additional cost.) (ParseError(\"could not find verb in effect clause).",
        );
        assert_eq!(
            clauses,
            vec!["Tiered (Choose one additional cost.)".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_preserve_that_objects_reference() {
        let clauses = semantic_clauses(
            "Choose target creature. Destroy all Auras or Equipment attached to that objects.",
        );
        assert_eq!(
            clauses,
            vec![
                "Choose target creature".to_string(),
                "Destroy all Auras or Equipment attached to that objects".to_string()
            ]
        );
    }

    #[test]
    fn test_semantic_clauses_preserve_that_object_reference() {
        let clauses = semantic_clauses("Whenever this creature enters, return that object.");
        assert_eq!(
            clauses,
            vec!["Whenever this creature enters, return that object".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_capitalized_player_reference() {
        let clauses = semantic_clauses(
            "That player controls an island. Each opponent controls one as well. That player draws two cards.",
        );
        let joined = clauses.join(" ");
        assert!(
            joined.contains("They control an island")
                && joined.contains("Each opponent controls one as well"),
            "capitalized that player references should normalize to they"
        );
    }

    #[test]
    fn test_semantic_clauses_repair_until_tail_split() {
        let clauses = semantic_clauses(
            "When this enchantment enters, exile target nonland permanent an opponent controls. until this enchantment leaves the battlefield.",
        );
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("exile target nonland permanent"));
        assert!(clause.contains("until this enchantment leaves the battlefield"));
    }

    #[test]
    fn test_semantic_clauses_add_missing_attack_subject() {
        let clauses = semantic_clauses("Can't attack unless defending player controls island.");
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.starts_with("this creature can't attack unless"));
    }

    #[test]
    fn test_semantic_clauses_normalize_you_control_no_colon_clause() {
        let clauses = semantic_clauses("You control no swamps: Sacrifice this creature.");
        assert_eq!(
            clauses,
            vec!["When you control no swamps, sacrifice this creature".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_merge_split_damage_followup() {
        let clauses =
            semantic_clauses("This creature deals 1 damage to any target. Deal 1 damage to you.");
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("deal 1 damage to any target"));
        assert!(clause.contains("and 1 damage to you"));
    }

    #[test]
    fn test_semantic_clauses_expand_create_list_clause() {
        let clauses = semantic_clauses(
            "Create a 2/1 white and black Inkling creature token with flying, a 3/2 red and white Spirit creature token, and a 4/4 blue and red Elemental creature token.",
        );
        assert_eq!(clauses.len(), 3);
        assert!(clauses[0].contains("Create a 2/1 white and black Inkling creature token"));
        assert!(clauses[1].contains("Create a 3/2 red and white Spirit creature token"));
        assert!(clauses[2].contains("Create a 4/4 blue and red Elemental creature token"));
    }

    #[test]
    fn test_semantic_clauses_normalize_untap_reference() {
        let clauses = semantic_clauses(
            "Target creature gains flying until end of turn. Untap that creature.",
        );
        assert_eq!(clauses.len(), 2);
        assert_eq!(clauses[1], "Untap it");
    }

    #[test]
    fn test_semantic_clauses_merge_counter_then_damage() {
        let clauses = semantic_clauses("Counter target spell. Deal 3 damage to target creature.");
        assert_eq!(clauses.len(), 1);
        assert_eq!(
            clauses[0],
            "Counter target spell and Deal 3 damage to target creature"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_counter_target_spell() {
        let clauses = semantic_clauses("Counter target creature spell.");
        assert_eq!(clauses, vec!["Counter target creature".to_string()]);
    }

    #[test]
    fn test_semantic_clauses_normalize_cost_modifier_conditions() {
        let clauses =
            semantic_clauses("This spell costs {3} less to cast as long as you control an Island.");
        assert!(
            clauses.contains(
                &"Spells cost {3} less to cast as long as you control an Island".to_string()
            ),
            "cost modifier normalization must preserve gating conditions"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_spells_that_target_cost_conditional() {
        let clauses = semantic_clauses("Spells that target tapped creature cost {2} less to cast.");
        assert!(
            clauses
                .contains(&"Spells that target tapped creature cost {2} less to cast".to_string()),
            "target-qualified cost modifiers must preserve target qualifiers"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_spells_that_target_cost_conditional_with_non_ascii() {
        let clauses = semantic_clauses("Spells that target tapped creature cost {2} less to cast.");
        assert!(
            clauses
                .contains(&"Spells that target tapped creature cost {2} less to cast".to_string())
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_choose_target_phrase() {
        let clauses = semantic_clauses(
            "Choose target creature an opponent controls. target an opponent's creature can't be blocked until your next turn.",
        );
        assert!(
            clauses.contains(&"Target creature an opponent controls".to_string())
                || clauses.contains(&"Target creature an opponent controls.".to_string())
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("can't be blocked until your next turn")),
        );
    }

    #[test]
    fn test_semantic_clauses_preserve_choose_verb() {
        let clauses =
            semantic_clauses("Target opponent chooses target creature an opponent controls.");
        assert!(
            clauses.iter().any(|clause| clause
                .to_ascii_lowercase()
                .contains("chooses target creature")),
            "choose-target clauses should not be rewritten into unrelated action verbs"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_basic_land_type_among_lands() {
        let clauses = semantic_clauses(
            "Target creature gets +1/+1 until end of turn for each basic land type among lands.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("basic land type among lands"))
        );
    }

    #[test]
    fn test_semantic_clauses_preserve_one_one_qualifier() {
        let clauses = semantic_clauses("Each 1/1 creature gets +1/+0 until end of turn.");
        assert!(
            clauses.iter().any(|clause| clause.contains("1/1 creature")),
            "1/1 qualifier should not be erased by normalization"
        );
    }

    #[test]
    fn test_semantic_clauses_preserve_no_islands_controller_scope() {
        let clauses = semantic_clauses("You control no islands: Sacrifice this creature.");
        assert_eq!(
            clauses,
            vec!["When you control no islands, sacrifice this creature".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_preserve_choose_new_targets_for_it_outside_copy_context() {
        let clauses = semantic_clauses("Counter target spell. You may choose new targets for it.");
        assert!(
            clauses.iter().any(|clause| clause
                .to_ascii_lowercase()
                .contains("choose new targets for it")),
            "pronoun retargeting should not be rewritten into copy-specific wording without copy context"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_choose_new_targets_for_it_in_copy_context() {
        let clauses = semantic_clauses(
            "Copy target instant or sorcery spell. You may choose new targets for it.",
        );
        assert!(
            clauses.iter().any(|clause| clause
                .to_ascii_lowercase()
                .contains("choose new targets for the copy")),
            "copy-context retargeting should still normalize to copy wording"
        );
    }

    #[test]
    fn test_semantic_clauses_preserve_half_that_permanent_outside_saw_context() {
        let clauses = semantic_clauses(
            "Its power and toughness are each half that permanent's power and toughness, rounded up.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("half that permanent's power and toughness")),
            "half-that-permanent wording should not be rewritten to creature outside explicit saw-in-half context"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_damage_to_this_creature() {
        let clauses = semantic_clauses(
            "This creature deals 2 damage to any target. Deal 1 damage to this creature.",
        );
        assert!(
            clauses.iter().any(|clause| clause.contains("to itself")),
            "Deal-to-this-creature should normalize to itself"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_each_players_upkeep() {
        let clauses =
            semantic_clauses("At the beginning of each player's upkeep, you gain 3 life.");
        assert_eq!(
            clauses,
            vec!["At the beginning of each upkeep, you gain 3 life".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_cumulative_upkeep() {
        let clauses = semantic_clauses("Cumulative upkeep put a -1/-1 counter on this creature.");
        assert_eq!(
            clauses,
            vec!["Cumulative upkeep put a -1/-1 counter on this creature".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_scope_then_put_on_that_object() {
        let clauses = semantic_clauses(
            "Whenever this creature attacks, for each attacking creature, Put a +1/+1 counter on that object.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each attacking creature"))
                && clauses
                    .iter()
                    .any(|clause| clause.contains("Put a +1/+1 counter on that object"))
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_numbered_attacking_creature_scope() {
        let clauses = semantic_clauses(
            "Whenever this creature attacks, put the number of a attacking creature you control +1/+1 counter(s) on it.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each attacking creature you control"))
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_counter_scope_from_number_prefix() {
        let clauses = semantic_clauses(
            "At the beginning of each end step, put the number of creature +1/+1 counter(s) on this creature.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each creature"))
        );

        let clauses = semantic_clauses(
            "At the beginning of your upkeep, put the number of card in your hand +1/+1 counter(s) on this creature. Remove up to one counters from creature card in an opponent's hand.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each card in your hand"))
        );

        let clauses = semantic_clauses(
            "When this creature enters, mill 2 cards, then put the number of artifact or creature card in your graveyard +1/+1 counter(s) on this creature.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause
                    .contains("for each artifact or creature card in your graveyard"))
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_cost_prefixed_damage_to_itself() {
        let clauses = semantic_clauses(" {T}: This creature deals 2 damage to this creature.");
        assert!(
            clauses.iter().any(|clause| clause.contains("to itself")),
            "Cost-prefixed self-damage should normalize to itself"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_strive_cost_modifier() {
        let clauses = semantic_clauses(
            "Strive — This spell costs {1}{G} more to cast for each target beyond the first.",
        );
        assert!(
            clauses.iter().any(|clause| {
                let lower = clause.to_ascii_lowercase();
                (lower.contains("spells cost {1}{g} more to cast")
                    || lower.contains("this spell costs {1}{g} more to cast"))
                    && lower.contains("for each target beyond the first")
            }),
            "strive cost modifiers should preserve the per-target condition"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_saprazzan_graveyard_split() {
        let clauses = semantic_clauses(
            "When this creature enters, exile all artifact. Exile all enchantment card from a graveyard.",
        );
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("exile all artifact and enchantment cards from all graveyards"));
    }

    #[test]
    fn test_compare_semantics_normalizes_card_name_self_reference() {
        let oracle = "{2}, Sacrifice Five Hundred Year Diary: Draw a card.";
        let compiled =
            vec!["Activated ability 1: {2}, Sacrifice this artifact: Draw a card.".to_string()];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("Five Hundred Year Diary", oracle, &compiled, None);
        assert!(!mismatch);
        assert!(oracle_coverage >= 0.5);
        assert!(compiled_coverage >= 0.5);
    }

    #[test]
    fn test_compare_semantics_penalizes_unless_pay_role_inversion() {
        let oracle =
            "Whenever an opponent casts a spell, you may draw a card unless opponent pays {1}.";
        let compiled = vec![
            "Triggered ability 1: Whenever an opponent casts a spell, you may draw a card unless you pay {1}.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Rhystic Study",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "payer-role inversion must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "payer-role inversion should not remain above strict 0.99 score floor (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_missing_esper_sentinel_where_x_power_clause() {
        let oracle = "Whenever an opponent casts their first noncreature spell each turn, draw a card unless that player pays {X}, where X is this creature's power.";
        let compiled = vec![
            "Triggered ability 1: Whenever an opponent casts noncreature spell as that player's first spell this turn, you draw a card unless they pay {X}.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Esper Sentinel",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "missing where-X power clause must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "missing where-X power clause should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_missing_activated_ability_cost_floor_clause() {
        let oracle = "Activated abilities of creatures you control cost {2} less to activate.
This effect can't reduce the mana in that cost to less than one mana.";
        let compiled = vec![
            "Static ability 1: Activated abilities of creatures you control cost {2} less to activate."
                .to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Training Grounds",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "missing minimum-cost floor clause must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "missing minimum-cost floor clause should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_random_card_count_erasure() {
        let oracle = "Return two cards at random from your graveyard to your hand.";
        let compiled = vec!["Return a card from your graveyard to your hand.".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Random Return Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "random-card count erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "random-card count erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_exchange_shared_type_condition_erasure() {
        let oracle = "Exchange control of two target permanents that share a card type.";
        let compiled = vec!["Exchange control of two target permanents.".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Exchange Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "shared-type condition erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "shared-type condition erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_phase_out_clause_erasure() {
        let oracle = "Target creature phases out.";
        let compiled = vec!["Choose target creature.".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Phase Out Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "phase-out erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "phase-out erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_blocked_quantifier_erasure() {
        let oracle = "Each blocked creature gets +1/+0 until end of turn.";
        let compiled = vec!["Each creature gets +1/+0 until end of turn.".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Blocked Quantifier Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "blocked-quantifier erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "blocked-quantifier erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_controller_scope_erasure() {
        let oracle = "Creatures you control get +1/+1 until end of turn.";
        let compiled = vec!["Creatures get +1/+1 until end of turn.".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Controller Scope Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "controller-scope erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "controller-scope erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_cost_modifier_condition_erasure() {
        let oracle = "This spell costs {3} less to cast as long as you control an Island.";
        let compiled = vec!["Spells cost {3} less to cast.".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Cost Condition Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "cost-condition erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "cost-condition erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_domain_type_count_erasure() {
        let oracle = "Target creature gets +1/+1 until end of turn for each basic land type among lands you control.";
        let compiled = vec![
            "Target creature gets +1/+1 until end of turn for each basic land you control."
                .to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Domain Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "domain type-count erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "domain type-count erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_enchanted_type_erasure_from_tagged_object_scaffolding() {
        let oracle = "Destroy enchanted creature.";
        let compiled = vec![
            "Spell effects: Tag the object attached to this Aura as 'enchanted'. Destroy target tagged object 'enchanted'."
                .to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Enchanted Target Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "enchanted-type erasure must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "enchanted-type erasure should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_springheart_nantuko_strict_similarity() {
        let oracle = "Bestow {1}{G}\nEnchanted creature gets +1/+1.\nLandfall — Whenever a land you control enters, you may pay {1}{G} if this permanent is attached to a creature you control. If you do, create a token that's a copy of that creature. If you didn't create a token this way, create a 1/1 green Insect creature token.";
        let compiled = vec![
            "Bestow {1}{G}".to_string(),
            "Static ability 1: Enchanted creature gets +1/+1.".to_string(),
            "Triggered ability 2: Whenever a land you control enters, tag the object attached to this permanent as 'enchanted'. you may If enchanted creature matches a creature you control, you pay {1}{G}. If you do, Create a token that's a copy of enchanted creature. If effect #0 that doesn't happen, Create a 1/1 green Insect creature token.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Springheart Nantuko",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "Springheart Nantuko should clear strict semantic gate after normalization (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "Springheart Nantuko similarity should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_soulbond_keyword_scaffolding() {
        let oracle = "Soulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)
As long as this creature is paired with another creature, each of those creatures has \"Whenever this creature deals damage to an opponent, draw a card.\"";
        let compiled = vec![
            "Triggered ability 1: Whenever a creature you control enters, effect(SoulbondPairEffect)"
                .to_string(),
            "Static ability 2: As long as this is paired with another creature each of those creatures has \"Whenever this creature deals damage to an opponent, draw a card.\"".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Tandem Lookout",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "soulbond scaffolding should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "soulbond normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_echo_counter_scaffolding() {
        let oracle = "Flying, protection from black
Echo {3}{W}{W}
When this creature enters, return target creature card from your graveyard to the battlefield.";
        let compiled = vec![
            "Keyword ability 1: Flying, Protection from black".to_string(),
            "Static ability 3: This creature enters with an echo counter on it.".to_string(),
            "Triggered ability 4: At the beginning of your upkeep, remove an echo counter from this creature. If effect #0 happened, Sacrifice this creature unless you pay {3}{W}{W}.".to_string(),
            "Triggered ability 5: When this creature enters, return target creature card from your graveyard to the battlefield.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Karmic Guide",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "echo scaffolding should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "echo normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_grant_play_tagged_scaffolding() {
        let oracle = "Sacrifice a Treasure: Exile the top card of your library. You may play that card this turn.";
        let compiled = vec!["Activated ability 3: Sacrifice a Treasure you control: you exile the top card of your library. you may Effect(GrantPlayTaggedEffect { tag: TagKey(\"exiled_0\"), player: You, duration: UntilEndOfTurn })".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Professional Face-Breaker",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "grant-play-tagged scaffolding should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "grant-play-tagged normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_generic_effect_scaffolding_not_as_play_permission() {
        let oracle = "Sacrifice a Treasure: Exile the top card of your library. You may play that card this turn.";
        let compiled = vec!["Activated ability 3: Sacrifice a Treasure you control: you exile the top card of your library. you may Effect(SomeOtherEffect { player: You })".to_string()];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Generic Effect Probe",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "generic Effect(...) scaffolding should not be normalized as play permission"
        );
        assert!(
            similarity_score < 0.99,
            "generic Effect(...) scaffolding should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_named_counter_wording() {
        let oracle = "This artifact enters with three wish counters on it.
{1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.";
        let compiled = vec![
            "Static ability 1: This artifact enters with three wish counters on it.".to_string(),
            "Activated ability 2: {1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Wishclaw Talisman",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "named-counter wording should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "named-counter normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_pact_upkeep_payment_clause() {
        let oracle = "Counter target spell.
At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.";
        let compiled = vec![
            "Spell effects: Counter target spell.".to_string(),
            "Triggered ability 1: At the beginning of your upkeep, you pay {3}{U}{U}. If that doesn't happen, you lose the game.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Pact of Negation",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "pact upkeep wording should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "pact upkeep normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_flags_homeward_path_owned_creatures_quantifier_loss() {
        let oracle = "{T}: Add {C}.
{T}: Each player gains control of all creatures they own.";
        let compiled = vec![
            "Mana ability 1: {T}: Add {C}.".to_string(),
            "Activated ability 2: {T}: For each player, that player gains control of a creature that player owns.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Homeward Path",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "quantifier loss should count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "quantifier loss should stay below strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_heat_shimmer_temporary_copy_clause() {
        let oracle = "Create a token that's a copy of target creature, except it has haste and \"At the beginning of the end step, exile this token.\"";
        let compiled = vec![
            "Spell effects: Create a token that's a copy of target creature, with haste, and exile it at the beginning of the next end step.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Heat Shimmer",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "temporary-copy wording should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "temporary-copy normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_boggart_trawler_graveyard_exile_clause() {
        let oracle = "When this creature enters, exile target player's graveyard.";
        let compiled = vec![
            "Triggered ability 1: When this creature enters, exile all cards from target player's graveyard.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Boggart Trawler",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "graveyard-exile wording should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "graveyard-exile normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_static_prison_sentence_split_and_pay_typo() {
        let oracle = "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield. You get {E}{E}.
At the beginning of your first main phase, sacrifice this enchantment unless you pay {E}.";
        let compiled = vec![
            "Triggered ability 1: When this enchantment enters, exile target opponent's nonland permanent until this enchantment leaves the battlefield and you get {E}{E}.".to_string(),
            "Triggered ability 2: At the beginning of your first main phase, sacrifice this enchantment unless you Pay {E}.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Static Prison",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "static-prison wording should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "static-prison normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_saw_in_half_death_copy_wording() {
        let oracle = "Destroy target creature. If that creature dies this way, its controller creates two tokens that are copies of that creature, except their power is half that creature's power and their toughness is half that creature's toughness. Round up each time.";
        let compiled = vec![
            "Spell effects: Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under its controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Saw in Half",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            eprintln!("similarity_score={similarity_score} mismatch={mismatch}");
        }
        assert!(
            !mismatch,
            "saw-in-half wording should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "saw-in-half normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_hullbreaker_horror_modal_bullet_formatting() {
        let oracle = "Whenever you cast a spell, choose up to one —
• Return target spell you don't control to its owner's hand.
• Return target nonland permanent to its owner's hand.";
        let compiled = vec![
            "Triggered ability 3: Whenever you cast a spell, choose up to one - Return target spell you don't control to its owner's hand. • Return target nonland permanent to its owner's hand.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Hullbreaker Horror",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "hullbreaker modal formatting should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "hullbreaker modal formatting normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_ertai_modal_bullet_formatting() {
        let oracle = "When this creature enters, choose up to one —
• Counter target spell, activated ability, or triggered ability. Its controller draws a card.
• Destroy another target creature or planeswalker. Its controller draws a card.";
        let compiled = vec![
            "Triggered ability 2: When this creature enters, choose up to one - Counter target spell, activated ability, or triggered ability. Its controller draws a card. • Destroy another target creature or planeswalker. Its controller draws a card.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Ertai Resurrected",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "ertai modal formatting should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "ertai modal formatting normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_compare_semantics_normalizes_urzas_saga_zero_or_one_mana_cost_wording() {
        let oracle = "III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.";
        let compiled = vec![
            "Triggered ability 3: III — Search your library for an artifact card with mana value 1 or less, put it onto the battlefield, then shuffle.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Urza's Saga",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            !mismatch,
            "urza-saga mana-cost wording should normalize to no mismatch (score={similarity_score})"
        );
        assert!(
            similarity_score >= 0.99,
            "urza-saga mana-cost normalization should meet strict 0.99 threshold (score={similarity_score})"
        );
    }

    #[test]
    fn test_csv_push_row_escapes_special_characters() {
        let mut out = String::new();
        csv_push_row(
            &mut out,
            vec![
                "plain".to_string(),
                "has,comma".to_string(),
                "has\"quote".to_string(),
                "has\nnewline".to_string(),
            ],
        );
        assert_eq!(
            out,
            "plain,\"has,comma\",\"has\"\"quote\",\"has\nnewline\"\n"
        );
    }
}
